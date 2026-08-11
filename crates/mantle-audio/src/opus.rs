use mantle_opus::OpusEncoder;

use super::{
    AudioFrameError, COMPATIBLE_CHANNELS, COMPATIBLE_PCM_SAMPLES, COMPATIBLE_SAMPLE_RATE,
    COMPATIBLE_SAMPLES_PER_CHANNEL, EncodedFrameSlot, MAX_COMPATIBLE_OPUS_FRAME_BYTES, PcmFormat,
    PcmFrame, VolumeLevel, apply_volume, convert_to_i16,
};

/// Lavaplayer-compatible libopus complexity in the inclusive range `0..=10`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpusEncodingQuality(u8);

impl OpusEncodingQuality {
    pub const MINIMUM: Self = Self(0);
    pub const MAXIMUM: Self = Self(10);

    #[must_use]
    pub fn new(quality: i32) -> Self {
        Self(u8::try_from(quality.clamp(0, 10)).unwrap_or(0))
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl Default for OpusEncodingQuality {
    fn default() -> Self {
        Self::MAXIMUM
    }
}

/// Allocation-stable encoder for Mantle's frozen 48 kHz stereo 20 ms output geometry.
pub struct PcmOpusEncoder {
    inner: OpusEncoder,
    pcm: [i16; COMPATIBLE_PCM_SAMPLES],
    scratch: [u8; MAX_COMPATIBLE_OPUS_FRAME_BYTES],
    quality: OpusEncodingQuality,
}

impl PcmOpusEncoder {
    /// Allocates native encoder state and fixed packet scratch storage.
    ///
    /// # Errors
    ///
    /// Returns an error when the pinned libopus backend cannot create or configure its state.
    pub fn new(quality: OpusEncodingQuality) -> Result<Self, AudioFrameError> {
        let inner = OpusEncoder::new(COMPATIBLE_SAMPLE_RATE, COMPATIBLE_CHANNELS, quality.get())
            .map_err(|_| AudioFrameError::OpusEncodingFailure)?;
        Ok(Self {
            inner,
            pcm: [0; COMPATIBLE_PCM_SAMPLES],
            scratch: [0; MAX_COMPATIBLE_OPUS_FRAME_BYTES],
            quality,
        })
    }

    /// Encodes one compatible canonical PCM frame into an inline slot without allocating.
    ///
    /// # Errors
    ///
    /// Returns an error before changing `output` when the PCM format or geometry is incompatible,
    /// or when libopus rejects the input.
    pub fn encode(
        &mut self,
        frame: &PcmFrame,
        output: &mut EncodedFrameSlot,
        volume: VolumeLevel,
    ) -> Result<(), AudioFrameError> {
        let expected_format = PcmFormat::new(COMPATIBLE_SAMPLE_RATE, COMPATIBLE_CHANNELS)
            .map_err(|_| AudioFrameError::OpusEncodingFailure)?;
        if frame.format() != Some(expected_format) {
            return Err(AudioFrameError::PcmFormatMismatch {
                expected: expected_format,
                actual: frame.format(),
            });
        }
        if frame.samples().len() != COMPATIBLE_PCM_SAMPLES {
            return Err(AudioFrameError::InvalidOpusPcmSamples {
                expected: COMPATIBLE_PCM_SAMPLES,
                actual: frame.samples().len(),
            });
        }
        convert_to_i16(frame, &mut self.pcm)?;
        apply_volume(&mut self.pcm, volume);
        let written = self
            .inner
            .encode(&self.pcm, COMPATIBLE_SAMPLES_PER_CHANNEL, &mut self.scratch)
            .map_err(|_| AudioFrameError::OpusEncodingFailure)?;
        output.write(&self.scratch[..written], frame.timestamp(), volume)
    }

    /// Clears encoder history after a seek or track reset.
    ///
    /// # Errors
    ///
    /// Returns an error if the native encoder rejects its reset control request.
    pub fn reset(&mut self) -> Result<(), AudioFrameError> {
        self.inner
            .reset()
            .map_err(|_| AudioFrameError::OpusEncodingFailure)
    }

    #[must_use]
    pub const fn quality(&self) -> OpusEncodingQuality {
        self.quality
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{OpusEncodingQuality, PcmOpusEncoder};
    use crate::{
        AudioFrameError, COMPATIBLE_PCM_SAMPLES, COMPATIBLE_SAMPLE_RATE, EncodedFrameSlot,
        PcmFormat, PcmFrame, VolumeLevel,
    };

    #[test]
    fn quality_clamps_like_lavaplayer() {
        assert_eq!(OpusEncodingQuality::new(-1), OpusEncodingQuality::MINIMUM);
        assert_eq!(OpusEncodingQuality::new(11), OpusEncodingQuality::MAXIMUM);
        assert_eq!(OpusEncodingQuality::default(), OpusEncodingQuality::MAXIMUM);
    }

    #[test]
    fn encodes_compatible_pcm_into_stable_inline_storage() {
        let timestamp = Some(Duration::from_millis(120));
        let format = PcmFormat::new(COMPATIBLE_SAMPLE_RATE, 2).unwrap();
        let mut frame = PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES);
        frame
            .copy_from_interleaved(&[0.0; COMPATIBLE_PCM_SAMPLES], format, timestamp)
            .unwrap();
        let mut encoder = PcmOpusEncoder::new(OpusEncodingQuality::MAXIMUM).unwrap();
        let mut output = EncodedFrameSlot::new();
        let storage = output.data.as_ptr();

        encoder
            .encode(&frame, &mut output, VolumeLevel::new(175))
            .unwrap();
        assert!(!output.data().is_empty());
        assert_eq!(output.data.as_ptr(), storage);
        assert_eq!(output.timestamp(), timestamp);
        assert_eq!(output.volume(), VolumeLevel::new(175));
    }

    #[test]
    fn reset_reproduces_output_volume_is_applied_and_bad_geometry_preserves_the_slot() {
        let format = PcmFormat::new(COMPATIBLE_SAMPLE_RATE, 2).unwrap();
        let mut frame = PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES);
        frame
            .copy_from_interleaved(&[0.125; COMPATIBLE_PCM_SAMPLES], format, None)
            .unwrap();
        let mut encoder = PcmOpusEncoder::new(OpusEncodingQuality::MAXIMUM).unwrap();
        let mut output = EncodedFrameSlot::new();
        encoder
            .encode(&frame, &mut output, VolumeLevel::NORMAL)
            .unwrap();
        let first = output.data().to_vec();
        encoder.reset().unwrap();
        encoder
            .encode(&frame, &mut output, VolumeLevel::NORMAL)
            .unwrap();
        assert_eq!(output.data(), first);

        encoder.reset().unwrap();
        encoder
            .encode(&frame, &mut output, VolumeLevel::MUTED)
            .unwrap();
        assert_eq!(output.data(), first);
        assert_eq!(output.volume(), VolumeLevel::MUTED);

        encoder.reset().unwrap();
        encoder
            .encode(&frame, &mut output, VolumeLevel::new(200))
            .unwrap();
        assert_ne!(output.data(), first);
        assert_eq!(output.volume(), VolumeLevel::new(200));
        let amplified = output.data().to_vec();

        frame.prepare(2, format, None).unwrap();
        assert_eq!(
            encoder.encode(&frame, &mut output, VolumeLevel::NORMAL),
            Err(AudioFrameError::InvalidOpusPcmSamples {
                expected: COMPATIBLE_PCM_SAMPLES,
                actual: 2,
            })
        );
        assert_eq!(output.data(), amplified);
    }
}
