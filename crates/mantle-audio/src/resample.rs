use std::time::Duration;

use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{
    Async, FixedAsync, Indexing, PolynomialDegree, Resampler, SincInterpolationParameters,
    SincInterpolationType, WindowFunction,
};

use super::{AudioFrameError, PcmFormat, PcmFrame};

pub const MAX_RESAMPLER_CHUNK_FRAMES: usize = 8_192;
pub const MAX_RESAMPLER_BUFFERED_FRAMES: usize = 32_768;
const MAX_RESAMPLE_FACTOR: f64 = 8.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ResamplingQuality {
    Low,
    #[default]
    Medium,
    High,
}

/// Bounded streaming interleaved PCM resampler with preallocated input and output storage.
pub struct PcmResampler {
    inner: Async<f32>,
    source_format: PcmFormat,
    target_format: PcmFormat,
    output_chunk_frames: usize,
    max_buffered_frames: usize,
    pending: Vec<f32>,
    scratch: Vec<f32>,
    ready: Vec<f32>,
    delay_remaining: usize,
    base_timestamp: Option<Duration>,
    timestamp_initialized: bool,
    input_frames_received: u64,
    output_frames_delivered: u64,
    expected_output_frames: Option<u64>,
}

impl PcmResampler {
    /// Creates a fixed-ratio resampler. All allocations occur in this constructor.
    ///
    /// `input_chunk_frames` controls backend processing granularity. `output_chunk_frames`
    /// controls the blocks returned by [`Self::read`]. The final block may be shorter.
    ///
    /// # Errors
    ///
    /// Returns an error for an unnecessary or excessive ratio, zero or oversized chunks, or an
    /// input buffer that cannot hold at least one backend chunk.
    pub fn new(
        source_format: PcmFormat,
        target_rate: u32,
        quality: ResamplingQuality,
        input_chunk_frames: usize,
        output_chunk_frames: usize,
        max_buffered_frames: usize,
    ) -> Result<Self, AudioFrameError> {
        if source_format.sample_rate() == target_rate {
            return Err(AudioFrameError::InvalidResamplerConfiguration(
                "source and target rates are equal",
            ));
        }
        if input_chunk_frames == 0 || output_chunk_frames == 0 {
            return Err(AudioFrameError::InvalidResamplerConfiguration(
                "chunk sizes must be non-zero",
            ));
        }
        if input_chunk_frames > MAX_RESAMPLER_CHUNK_FRAMES
            || output_chunk_frames > MAX_RESAMPLER_CHUNK_FRAMES
        {
            return Err(AudioFrameError::InvalidResamplerConfiguration(
                "chunk size exceeds the global frame limit",
            ));
        }
        if max_buffered_frames < input_chunk_frames
            || max_buffered_frames > MAX_RESAMPLER_BUFFERED_FRAMES
        {
            return Err(AudioFrameError::InvalidResamplerConfiguration(
                "buffered-frame limit must contain a chunk and stay under the global limit",
            ));
        }

        let target_format = PcmFormat::new(target_rate, source_format.channels())?;
        let ratio = f64::from(target_rate) / f64::from(source_format.sample_rate());
        if !(1.0 / MAX_RESAMPLE_FACTOR..=MAX_RESAMPLE_FACTOR).contains(&ratio) {
            return Err(AudioFrameError::UnsupportedResampleRatio {
                source_rate: source_format.sample_rate(),
                target_rate,
            });
        }
        let channels = usize::from(source_format.channels());
        let inner = build_resampler(ratio, quality, input_chunk_frames, channels)?;
        let input_frames_max = inner.input_frames_max();
        if input_frames_max > max_buffered_frames {
            return Err(AudioFrameError::InvalidResamplerConfiguration(
                "buffered-frame limit is smaller than the backend maximum input",
            ));
        }
        let output_frames_max = inner.output_frames_max();
        let scratch_samples = output_frames_max.checked_mul(channels).ok_or(
            AudioFrameError::InvalidResamplerConfiguration("scratch sample count overflowed"),
        )?;
        let ready_frames = output_frames_max.checked_add(output_chunk_frames).ok_or(
            AudioFrameError::InvalidResamplerConfiguration("ready frame count overflowed"),
        )?;
        let ready_samples = ready_frames.checked_mul(channels).ok_or(
            AudioFrameError::InvalidResamplerConfiguration("ready sample count overflowed"),
        )?;
        let pending_samples = max_buffered_frames.checked_mul(channels).ok_or(
            AudioFrameError::InvalidResamplerConfiguration("input sample count overflowed"),
        )?;
        let delay_remaining = inner.output_delay();

        Ok(Self {
            inner,
            source_format,
            target_format,
            output_chunk_frames,
            max_buffered_frames,
            pending: Vec::with_capacity(pending_samples),
            scratch: vec![0.0; scratch_samples],
            ready: Vec::with_capacity(ready_samples),
            delay_remaining,
            base_timestamp: None,
            timestamp_initialized: false,
            input_frames_received: 0,
            output_frames_delivered: 0,
            expected_output_frames: None,
        })
    }

    /// Appends one source-rate block without processing or allocating.
    ///
    /// # Errors
    ///
    /// Returns an error before mutation for the wrong format, input after `finish`, or a pending
    /// input limit violation.
    pub fn push(&mut self, frame: &PcmFrame) -> Result<(), AudioFrameError> {
        if self.expected_output_frames.is_some() {
            return Err(AudioFrameError::ResamplerAlreadyFinished);
        }
        if frame.format() != Some(self.source_format) {
            return Err(AudioFrameError::PcmFormatMismatch {
                expected: self.source_format,
                actual: frame.format(),
            });
        }
        let channels = self.channels();
        let frame_count = frame.samples().len() / channels;
        let required_frames = self.pending_frames().checked_add(frame_count).ok_or(
            AudioFrameError::ResamplerInputLimitExceeded {
                required: usize::MAX,
                limit: self.max_buffered_frames,
            },
        )?;
        if required_frames > self.max_buffered_frames {
            return Err(AudioFrameError::ResamplerInputLimitExceeded {
                required: required_frames,
                limit: self.max_buffered_frames,
            });
        }
        let total_frames = self
            .input_frames_received
            .checked_add(u64::try_from(frame_count).unwrap_or(u64::MAX))
            .ok_or(AudioFrameError::InvalidResamplerConfiguration(
                "stream frame count overflowed",
            ))?;
        if !self.timestamp_initialized {
            self.base_timestamp = frame.timestamp();
            self.timestamp_initialized = true;
        }
        self.pending.extend_from_slice(frame.samples());
        self.input_frames_received = total_frames;
        Ok(())
    }

    /// Marks end of input and enables a final partial output block.
    ///
    /// # Errors
    ///
    /// Returns an error only when the exact output-frame count cannot be represented.
    pub fn finish(&mut self) -> Result<(), AudioFrameError> {
        if self.expected_output_frames.is_some() {
            return Ok(());
        }
        let numerator = u128::from(self.input_frames_received)
            .checked_mul(u128::from(self.target_format.sample_rate()))
            .and_then(|value| value.checked_add(u128::from(self.source_format.sample_rate()) - 1))
            .ok_or(AudioFrameError::InvalidResamplerConfiguration(
                "output frame count overflowed",
            ))?;
        let expected = numerator / u128::from(self.source_format.sample_rate());
        self.expected_output_frames = Some(u64::try_from(expected).map_err(|_| {
            AudioFrameError::InvalidResamplerConfiguration("output frame count exceeds u64")
        })?);
        Ok(())
    }

    /// Produces the next preconfigured target-rate block without allocating.
    ///
    /// Returns `false` when more input is required, or after all finished output is drained. A
    /// finished stream may return one final block shorter than `output_chunk_frames`.
    ///
    /// # Errors
    ///
    /// Returns an error when output storage is too small or the backend rejects validated buffers.
    pub fn read(&mut self, output: &mut PcmFrame) -> Result<bool, AudioFrameError> {
        while self.ready_frames() < self.output_chunk_frames && self.pump()? {}

        let available = self.ready_frames();
        let finished = self.expected_output_frames.is_some();
        if available == 0 || (!finished && available < self.output_chunk_frames) {
            output.clear();
            return Ok(false);
        }
        let frames = available.min(self.output_chunk_frames);
        let channels = self.channels();
        let samples = frames * channels;
        let timestamp = self.base_timestamp.map(|base| {
            base.checked_add(frames_to_duration(
                self.output_frames_delivered,
                self.target_format.sample_rate(),
            ))
            .unwrap_or(Duration::MAX)
        });
        output
            .prepare(samples, self.target_format, timestamp)?
            .copy_from_slice(&self.ready[..samples]);
        remove_prefix(&mut self.ready, samples);
        self.output_frames_delivered = self
            .output_frames_delivered
            .saturating_add(u64::try_from(frames).unwrap_or(u64::MAX));
        Ok(true)
    }

    pub fn reset(&mut self) {
        self.inner.reset();
        self.pending.clear();
        self.ready.clear();
        self.delay_remaining = self.inner.output_delay();
        self.base_timestamp = None;
        self.timestamp_initialized = false;
        self.input_frames_received = 0;
        self.output_frames_delivered = 0;
        self.expected_output_frames = None;
    }

    #[must_use]
    pub fn pending_frames(&self) -> usize {
        self.pending.len() / self.channels()
    }

    #[must_use]
    pub fn input_frames_next(&self) -> usize {
        self.inner.input_frames_next()
    }

    #[must_use]
    pub fn output_delay(&self) -> usize {
        self.inner.output_delay()
    }

    fn pump(&mut self) -> Result<bool, AudioFrameError> {
        let channels = self.channels();
        let required = self.inner.input_frames_next();
        let available = self.pending_frames();
        let expected = self.expected_output_frames;
        let retained_frames = self
            .output_frames_delivered
            .saturating_add(u64::try_from(self.ready_frames()).unwrap_or(u64::MAX));
        let (frames_to_read, partial) = if available >= required {
            (required, None)
        } else if expected.is_none() {
            return Ok(false);
        } else if available > 0 {
            (available, Some(available))
        } else if retained_frames < expected.unwrap_or(0) {
            (0, Some(0))
        } else {
            return Ok(false);
        };

        let input_samples = frames_to_read * channels;
        let input = InterleavedSlice::new(&self.pending[..input_samples], channels, frames_to_read)
            .map_err(|_| AudioFrameError::ResamplerFailure)?;
        let output_capacity_frames = self.scratch.len() / channels;
        let mut output =
            InterleavedSlice::new_mut(&mut self.scratch, channels, output_capacity_frames)
                .map_err(|_| AudioFrameError::ResamplerFailure)?;
        let indexing = partial.map(|frames| Indexing::new().partial_len(frames));
        let (_, frames_written) = self
            .inner
            .process_into_buffer(&input, &mut output, indexing.as_ref())
            .map_err(|_| AudioFrameError::ResamplerFailure)?;
        remove_prefix(&mut self.pending, input_samples);

        let discarded = self.delay_remaining.min(frames_written);
        self.delay_remaining -= discarded;
        let first_sample = discarded * channels;
        let mut retained = frames_written - discarded;
        if let Some(expected) = expected {
            let remaining = expected.saturating_sub(retained_frames);
            retained = retained.min(usize::try_from(remaining).unwrap_or(usize::MAX));
        }
        let retained_samples = retained * channels;
        if self.ready.len() + retained_samples > self.ready.capacity() {
            return Err(AudioFrameError::ResamplerFailure);
        }
        self.ready
            .extend_from_slice(&self.scratch[first_sample..first_sample + retained_samples]);
        Ok(true)
    }

    fn channels(&self) -> usize {
        usize::from(self.source_format.channels())
    }

    fn ready_frames(&self) -> usize {
        self.ready.len() / self.channels()
    }
}

fn build_resampler(
    ratio: f64,
    quality: ResamplingQuality,
    chunk_frames: usize,
    channels: usize,
) -> Result<Async<f32>, AudioFrameError> {
    match quality {
        ResamplingQuality::Low => Async::new_poly(
            ratio,
            1.0,
            PolynomialDegree::Linear,
            chunk_frames,
            channels,
            FixedAsync::Input,
        ),
        ResamplingQuality::Medium => {
            let parameters = SincInterpolationParameters::new(64, WindowFunction::Blackman2)
                .oversampling_factor(128)
                .interpolation(SincInterpolationType::Quadratic);
            Async::new_sinc(
                ratio,
                1.0,
                &parameters,
                chunk_frames,
                channels,
                FixedAsync::Input,
            )
        }
        ResamplingQuality::High => {
            let parameters = SincInterpolationParameters::new(128, WindowFunction::Blackman2)
                .oversampling_factor(256)
                .interpolation(SincInterpolationType::Cubic);
            Async::new_sinc(
                ratio,
                1.0,
                &parameters,
                chunk_frames,
                channels,
                FixedAsync::Input,
            )
        }
    }
    .map_err(|_| AudioFrameError::ResamplerFailure)
}

fn remove_prefix<T: Copy>(buffer: &mut Vec<T>, count: usize) {
    buffer.copy_within(count.., 0);
    buffer.truncate(buffer.len() - count);
}

fn frames_to_duration(frames: u64, sample_rate: u32) -> Duration {
    let nanos = u128::from(frames) * 1_000_000_000 / u128::from(sample_rate);
    Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;
    use std::time::Duration;

    use super::{PcmResampler, ResamplingQuality};
    use crate::{
        AudioFrameError, COMPATIBLE_PCM_SAMPLES, COMPATIBLE_SAMPLES_PER_CHANNEL, PcmFormat,
        PcmFrame,
    };

    #[test]
    fn streams_irregular_chunks_to_exact_length_without_reallocation() {
        let source = PcmFormat::new(44_100, 2).unwrap();
        let mut resampler = PcmResampler::new(
            source,
            48_000,
            ResamplingQuality::Medium,
            256,
            COMPATIBLE_SAMPLES_PER_CHANNEL,
            2_048,
        )
        .unwrap();
        let pending_storage = resampler.pending.as_ptr();
        let scratch_storage = resampler.scratch.as_ptr();
        let ready_storage = resampler.ready.as_ptr();
        let mut output = PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES);
        let output_storage = output.samples.as_ptr();
        let mut produced = Vec::new();
        let mut offset = 0;
        while offset < 4_410 {
            let frames = (4_410 - offset).min(173);
            let input = sine_frame(source, offset, frames);
            resampler.push(&input).unwrap();
            drain(&mut resampler, &mut output, &mut produced);
            offset += frames;
        }
        resampler.finish().unwrap();
        drain(&mut resampler, &mut output, &mut produced);

        assert_eq!(produced.len(), 4_800 * 2);
        assert!(produced.iter().any(|sample| sample.abs() > 0.1));
        assert_eq!(resampler.pending.as_ptr(), pending_storage);
        assert_eq!(resampler.scratch.as_ptr(), scratch_storage);
        assert_eq!(resampler.ready.as_ptr(), ready_storage);
        assert_eq!(output.samples.as_ptr(), output_storage);
    }

    #[test]
    fn reset_reproduces_output_and_clears_finished_state() {
        let source = PcmFormat::new(32_000, 1).unwrap();
        let mut resampler =
            PcmResampler::new(source, 48_000, ResamplingQuality::Low, 128, 160, 512).unwrap();
        let input = sine_frame(source, 0, 320);
        let first = run_clip(&mut resampler, &input, 160);
        resampler.reset();
        let second = run_clip(&mut resampler, &input, 160);
        assert_eq!(first, second);
        assert_eq!(first.len(), 480);
    }

    #[test]
    fn first_block_without_timestamp_keeps_stream_timestamps_absent() {
        let source = PcmFormat::new(32_000, 1).unwrap();
        let mut resampler =
            PcmResampler::new(source, 48_000, ResamplingQuality::Low, 128, 160, 512).unwrap();
        let first = sine_frame_at(source, 0, 160, None);
        let second = sine_frame_at(source, 160, 160, Some(Duration::from_secs(10)));
        resampler.push(&first).unwrap();
        resampler.push(&second).unwrap();
        resampler.finish().unwrap();

        let mut output = PcmFrame::with_capacity(160);
        while resampler.read(&mut output).unwrap() {
            assert_eq!(output.timestamp(), None);
        }
    }

    #[test]
    fn configuration_format_finish_and_input_bounds_are_enforced() {
        let source = PcmFormat::new(48_000, 2).unwrap();
        assert!(matches!(
            PcmResampler::new(source, 48_000, ResamplingQuality::Low, 128, 960, 128),
            Err(AudioFrameError::InvalidResamplerConfiguration(_))
        ));
        assert!(matches!(
            PcmResampler::new(source, 1_000, ResamplingQuality::Low, 128, 960, 128),
            Err(AudioFrameError::UnsupportedResampleRatio { .. })
        ));

        let source = PcmFormat::new(44_100, 2).unwrap();
        let mut resampler =
            PcmResampler::new(source, 48_000, ResamplingQuality::Low, 128, 960, 128).unwrap();
        let oversized = sine_frame(source, 0, 129);
        assert!(matches!(
            resampler.push(&oversized),
            Err(AudioFrameError::ResamplerInputLimitExceeded { limit: 128, .. })
        ));
        let wrong = sine_frame(PcmFormat::new(32_000, 2).unwrap(), 0, 1);
        assert!(matches!(
            resampler.push(&wrong),
            Err(AudioFrameError::PcmFormatMismatch { .. })
        ));
        resampler.finish().unwrap();
        assert!(matches!(
            resampler.push(&sine_frame(source, 0, 1)),
            Err(AudioFrameError::ResamplerAlreadyFinished)
        ));
    }

    fn run_clip(resampler: &mut PcmResampler, input: &PcmFrame, output_frames: usize) -> Vec<f32> {
        resampler.push(input).unwrap();
        resampler.finish().unwrap();
        let mut output = PcmFrame::with_capacity(output_frames);
        let mut samples = Vec::new();
        drain(resampler, &mut output, &mut samples);
        samples
    }

    fn drain(resampler: &mut PcmResampler, output: &mut PcmFrame, samples: &mut Vec<f32>) {
        while resampler.read(output).unwrap() {
            samples.extend_from_slice(output.samples());
        }
    }

    fn sine_frame(format: PcmFormat, offset: usize, frames: usize) -> PcmFrame {
        sine_frame_at(format, offset, frames, None)
    }

    fn sine_frame_at(
        format: PcmFormat,
        offset: usize,
        frames: usize,
        timestamp: Option<Duration>,
    ) -> PcmFrame {
        let channels = usize::from(format.channels());
        let mut samples = Vec::with_capacity(frames * channels);
        for frame in offset..offset + frames {
            #[allow(clippy::cast_precision_loss)]
            let phase = TAU * 440.0 * frame as f32 / format.sample_rate() as f32;
            let sample = phase.sin() * 0.5;
            for _ in 0..channels {
                samples.push(sample);
            }
        }
        let mut output = PcmFrame::with_capacity(samples.len());
        output
            .copy_from_interleaved(&samples, format, timestamp)
            .unwrap();
        output
    }
}
