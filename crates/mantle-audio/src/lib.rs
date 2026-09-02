//! Backend-independent, allocation-stable audio frame storage.

use std::fmt;
use std::time::Duration;

mod filter;
mod opus;
mod passthrough;
mod queue;
mod resample;
mod transform;

pub use filter::{
    EQUALIZER_BANDS, EqualizerFactory, FilterChainBuilder, FilterPipeline, MAX_FILTERS_PER_CHAIN,
    PcmFilter, PcmFilterFactory,
};
pub use opus::{OpusEncodingQuality, PcmOpusDecoder, PcmOpusEncoder};
pub use passthrough::{
    OpusModeTransition, OpusPacketRoute, OpusPassthrough, OpusPipelineMode, opus_packet_duration,
};
pub use queue::{
    DEFAULT_ENCODED_FRAME_QUEUE_CAPACITY, EncodedFrameConsumer, EncodedFrameProducer,
    EncodedFrameQueueConfigError, EncodedFrameQueueFull, MAX_ENCODED_FRAME_QUEUE_CAPACITY,
    encoded_frame_queue,
};
pub use resample::{
    MAX_RESAMPLER_BUFFERED_FRAMES, MAX_RESAMPLER_CHUNK_FRAMES, PcmResampler, ResamplingQuality,
};
pub use transform::{VolumeLevel, apply_volume, convert_to_i16, map_channels};

pub const COMPATIBLE_SAMPLE_RATE: u32 = 48_000;
pub const COMPATIBLE_CHANNELS: u16 = 2;
pub const COMPATIBLE_SAMPLES_PER_CHANNEL: usize = 960;
pub const COMPATIBLE_PCM_SAMPLES: usize = COMPATIBLE_SAMPLES_PER_CHANNEL * 2;
pub const COMPATIBLE_FRAME_DURATION: Duration = Duration::from_millis(20);
pub const MAX_COMPATIBLE_OPUS_FRAME_BYTES: usize = 1_568;

#[cfg(test)]
mod allocation_tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PcmFormat {
    sample_rate: u32,
    channels: u16,
}

impl PcmFormat {
    /// Creates a PCM format in Mantle's current mono/stereo pipeline scope.
    ///
    /// # Errors
    ///
    /// Returns an error for a zero sample rate or a channel count outside one or two.
    pub fn new(sample_rate: u32, channels: u16) -> Result<Self, AudioFrameError> {
        if sample_rate == 0 {
            return Err(AudioFrameError::InvalidSampleRate { sample_rate });
        }
        if !(1..=COMPATIBLE_CHANNELS).contains(&channels) {
            return Err(AudioFrameError::UnsupportedChannels { channels });
        }
        Ok(Self {
            sample_rate,
            channels,
        })
    }

    #[must_use]
    pub fn sample_rate(self) -> u32 {
        self.sample_rate
    }

    #[must_use]
    pub fn channels(self) -> u16 {
        self.channels
    }
}

/// Caller-owned reusable canonical interleaved `f32` PCM storage.
#[derive(Debug)]
pub struct PcmFrame {
    samples: Vec<f32>,
    format: Option<PcmFormat>,
    timestamp: Option<Duration>,
}

impl PcmFrame {
    #[must_use]
    pub fn with_capacity(sample_capacity: usize) -> Self {
        Self {
            samples: Vec::with_capacity(sample_capacity),
            format: None,
            timestamp: None,
        }
    }

    #[must_use]
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    #[must_use]
    pub fn samples_mut(&mut self) -> &mut [f32] {
        &mut self.samples
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.samples.capacity()
    }

    #[must_use]
    pub fn format(&self) -> Option<PcmFormat> {
        self.format
    }

    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.format.map_or(0, PcmFormat::sample_rate)
    }

    #[must_use]
    pub fn channels(&self) -> u16 {
        self.format.map_or(0, PcmFormat::channels)
    }

    #[must_use]
    pub fn timestamp(&self) -> Option<Duration> {
        self.timestamp
    }

    /// Prepares already-allocated storage for a backend to fill.
    ///
    /// The returned slice is interleaved. This method checks every invariant before changing the
    /// frame and never grows the backing allocation.
    ///
    /// # Errors
    ///
    /// Returns an error when the sample count is not channel-aligned or exceeds capacity.
    pub fn prepare(
        &mut self,
        sample_count: usize,
        format: PcmFormat,
        timestamp: Option<Duration>,
    ) -> Result<&mut [f32], AudioFrameError> {
        let channels = usize::from(format.channels());
        if !sample_count.is_multiple_of(channels) {
            return Err(AudioFrameError::MisalignedPcmSamples {
                samples: sample_count,
                channels: format.channels(),
            });
        }
        if sample_count > self.samples.capacity() {
            return Err(AudioFrameError::PcmCapacityExceeded {
                required: sample_count,
                capacity: self.samples.capacity(),
            });
        }
        self.samples.resize(sample_count, 0.0);
        self.format = Some(format);
        self.timestamp = timestamp;
        Ok(&mut self.samples)
    }

    /// Copies one interleaved block without growing storage.
    ///
    /// # Errors
    ///
    /// Returns an error when the input is not channel-aligned or exceeds capacity.
    pub fn copy_from_interleaved(
        &mut self,
        samples: &[f32],
        format: PcmFormat,
        timestamp: Option<Duration>,
    ) -> Result<(), AudioFrameError> {
        self.prepare(samples.len(), format, timestamp)?
            .copy_from_slice(samples);
        Ok(())
    }

    /// Converts signed 16-bit interleaved PCM into this canonical frame without growing storage.
    ///
    /// # Errors
    ///
    /// Returns an error when the input is not channel-aligned or exceeds capacity.
    pub fn copy_from_i16_interleaved(
        &mut self,
        samples: &[i16],
        format: PcmFormat,
        timestamp: Option<Duration>,
    ) -> Result<(), AudioFrameError> {
        let output = self.prepare(samples.len(), format, timestamp)?;
        for (output_sample, input_sample) in output.iter_mut().zip(samples) {
            *output_sample = f32::from(*input_sample) / 32_768.0;
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        self.samples.clear();
        self.timestamp = None;
    }
}

/// Inline storage for one Discord-compatible 20 ms Opus output frame.
#[derive(Clone)]
pub struct EncodedFrameSlot {
    data: [u8; MAX_COMPATIBLE_OPUS_FRAME_BYTES],
    len: u16,
    volume: VolumeLevel,
    timestamp: Option<Duration>,
}

impl EncodedFrameSlot {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            data: [0; MAX_COMPATIBLE_OPUS_FRAME_BYTES],
            len: 0,
            volume: VolumeLevel::NORMAL,
            timestamp: None,
        }
    }

    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data[..usize::from(self.len)]
    }

    #[must_use]
    pub fn timestamp(&self) -> Option<Duration> {
        self.timestamp
    }

    #[must_use]
    pub const fn volume(&self) -> VolumeLevel {
        self.volume
    }

    #[must_use]
    pub const fn duration(&self) -> Duration {
        COMPATIBLE_FRAME_DURATION
    }

    /// Copies an encoded frame into the inline slot.
    ///
    /// # Errors
    ///
    /// Returns an error before mutation when the packet exceeds the compatible fixed capacity.
    pub fn write(
        &mut self,
        data: &[u8],
        timestamp: Option<Duration>,
        volume: VolumeLevel,
    ) -> Result<(), AudioFrameError> {
        if data.len() > MAX_COMPATIBLE_OPUS_FRAME_BYTES {
            return Err(AudioFrameError::EncodedFrameTooLarge {
                actual: data.len(),
                limit: MAX_COMPATIBLE_OPUS_FRAME_BYTES,
            });
        }
        let len = u16::try_from(data.len()).map_err(|_| AudioFrameError::EncodedFrameTooLarge {
            actual: data.len(),
            limit: MAX_COMPATIBLE_OPUS_FRAME_BYTES,
        })?;
        self.data[..data.len()].copy_from_slice(data);
        self.len = len;
        self.volume = volume;
        self.timestamp = timestamp;
        Ok(())
    }

    pub fn clear(&mut self) {
        self.len = 0;
        self.volume = VolumeLevel::NORMAL;
        self.timestamp = None;
    }
}

impl Default for EncodedFrameSlot {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for EncodedFrameSlot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EncodedFrameSlot")
            .field("len", &self.len)
            .field("volume", &self.volume)
            .field("timestamp", &self.timestamp)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioFrameError {
    InvalidSampleRate {
        sample_rate: u32,
    },
    UnsupportedChannels {
        channels: u16,
    },
    MissingPcmFormat,
    MisalignedPcmSamples {
        samples: usize,
        channels: u16,
    },
    PcmCapacityExceeded {
        required: usize,
        capacity: usize,
    },
    SampleBufferTooSmall {
        required: usize,
        capacity: usize,
    },
    FilterLimitExceeded {
        limit: usize,
    },
    PcmFormatMismatch {
        expected: PcmFormat,
        actual: Option<PcmFormat>,
    },
    InvalidResamplerConfiguration(&'static str),
    UnsupportedResampleRatio {
        source_rate: u32,
        target_rate: u32,
    },
    ResamplerInputLimitExceeded {
        required: usize,
        limit: usize,
    },
    ResamplerAlreadyFinished,
    ResamplerFailure,
    InvalidOpusPcmSamples {
        expected: usize,
        actual: usize,
    },
    OpusEncodingFailure,
    OpusDecodingFailure,
    EncodedFrameTooLarge {
        actual: usize,
        limit: usize,
    },
}

impl fmt::Display for AudioFrameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSampleRate { sample_rate } => {
                write!(
                    formatter,
                    "PCM sample rate must be non-zero, got {sample_rate}"
                )
            }
            Self::UnsupportedChannels { channels } => {
                write!(
                    formatter,
                    "PCM channel count must be one or two, got {channels}"
                )
            }
            Self::MissingPcmFormat => formatter.write_str("PCM frame has no initialized format"),
            Self::MisalignedPcmSamples { samples, channels } => write!(
                formatter,
                "{samples} interleaved PCM samples are not aligned to {channels} channels"
            ),
            Self::PcmCapacityExceeded { required, capacity } => write!(
                formatter,
                "PCM frame capacity is {capacity} samples; {required} are required"
            ),
            Self::SampleBufferTooSmall { required, capacity } => write!(
                formatter,
                "sample output capacity is {capacity}; {required} samples are required"
            ),
            Self::FilterLimitExceeded { limit } => {
                write!(
                    formatter,
                    "PCM filter chain exceeds its {limit}-filter limit"
                )
            }
            Self::PcmFormatMismatch { expected, actual } => write!(
                formatter,
                "PCM frame format {actual:?} does not match pipeline format {expected:?}"
            ),
            Self::InvalidResamplerConfiguration(message) => {
                write!(formatter, "invalid resampler configuration: {message}")
            }
            Self::UnsupportedResampleRatio {
                source_rate,
                target_rate,
            } => write!(
                formatter,
                "unsupported resample ratio from {source_rate} Hz to {target_rate} Hz"
            ),
            Self::ResamplerInputLimitExceeded { required, limit } => write!(
                formatter,
                "resampler input requires {required} buffered frames; limit is {limit}"
            ),
            Self::ResamplerAlreadyFinished => {
                formatter.write_str("cannot push PCM after finishing the resampler")
            }
            Self::ResamplerFailure => formatter.write_str("PCM resampling failed"),
            Self::InvalidOpusPcmSamples { expected, actual } => write!(
                formatter,
                "Opus input contains {actual} PCM samples; expected {expected}"
            ),
            Self::OpusEncodingFailure => formatter.write_str("Opus encoding failed"),
            Self::OpusDecodingFailure => formatter.write_str("Opus decoding failed"),
            Self::EncodedFrameTooLarge { actual, limit } => write!(
                formatter,
                "encoded frame has {actual} bytes; fixed slot limit is {limit}"
            ),
        }
    }
}

impl std::error::Error for AudioFrameError {}

#[cfg(test)]
mod tests {
    use std::mem;

    use super::{
        AudioFrameError, COMPATIBLE_CHANNELS, COMPATIBLE_FRAME_DURATION, COMPATIBLE_PCM_SAMPLES,
        COMPATIBLE_SAMPLE_RATE, EncodedFrameSlot, MAX_COMPATIBLE_OPUS_FRAME_BYTES, PcmFormat,
        PcmFrame, VolumeLevel,
    };

    #[test]
    fn pcm_storage_never_grows_and_rejects_invalid_geometry_before_mutation() {
        let format = PcmFormat::new(COMPATIBLE_SAMPLE_RATE, COMPATIBLE_CHANNELS).unwrap();
        let mut frame = PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES);
        let storage = frame.samples.as_ptr();
        frame
            .copy_from_interleaved(&vec![0.25; COMPATIBLE_PCM_SAMPLES], format, None)
            .unwrap();
        assert_eq!(frame.samples.as_ptr(), storage);
        assert_eq!(frame.format(), Some(format));
        assert!(matches!(
            frame.prepare(COMPATIBLE_PCM_SAMPLES - 1, format, None),
            Err(AudioFrameError::MisalignedPcmSamples { .. })
        ));
        assert_eq!(frame.samples().len(), COMPATIBLE_PCM_SAMPLES);
        assert!(matches!(
            frame.prepare(COMPATIBLE_PCM_SAMPLES + 2, format, None),
            Err(AudioFrameError::PcmCapacityExceeded { .. })
        ));
        assert_eq!(frame.samples.as_ptr(), storage);
    }

    #[test]
    fn fixed_encoded_slot_rejects_oversize_before_mutation() {
        let mut slot = EncodedFrameSlot::new();
        let storage = slot.data.as_ptr();
        slot.write(
            &[1, 2, 3],
            Some(COMPATIBLE_FRAME_DURATION),
            VolumeLevel::new(175),
        )
        .unwrap();
        assert_eq!(slot.data(), [1, 2, 3]);
        assert_eq!(slot.data.as_ptr(), storage);
        assert_eq!(slot.duration(), COMPATIBLE_FRAME_DURATION);
        assert_eq!(slot.volume(), VolumeLevel::new(175));
        assert!(matches!(
            slot.write(
                &vec![0; MAX_COMPATIBLE_OPUS_FRAME_BYTES + 1],
                None,
                VolumeLevel::NORMAL,
            ),
            Err(AudioFrameError::EncodedFrameTooLarge { .. })
        ));
        assert_eq!(slot.data(), [1, 2, 3]);
        assert_eq!(slot.volume(), VolumeLevel::new(175));
        assert!(mem::size_of::<EncodedFrameSlot>() <= 1_600);
    }

    #[test]
    fn signed_pcm_conversion_matches_the_reference_scale() {
        let format = PcmFormat::new(48_000, 1).unwrap();
        let mut frame = PcmFrame::with_capacity(5);
        frame
            .copy_from_i16_interleaved(&[i16::MIN, -1, 0, 1, i16::MAX], format, None)
            .unwrap();
        assert_eq!(
            frame.samples(),
            [
                -1.0,
                -1.0 / 32_768.0,
                0.0,
                1.0 / 32_768.0,
                32_767.0 / 32_768.0
            ]
        );
    }

    #[test]
    fn format_rejects_zero_rate_and_out_of_scope_channels() {
        assert!(matches!(
            PcmFormat::new(0, 2),
            Err(AudioFrameError::InvalidSampleRate { .. })
        ));
        assert!(matches!(
            PcmFormat::new(48_000, 3),
            Err(AudioFrameError::UnsupportedChannels { .. })
        ));
    }
}
