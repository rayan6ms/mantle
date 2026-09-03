use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use std::time::Duration;

use super::{
    AudioFrameError, COMPATIBLE_CHANNELS, COMPATIBLE_PCM_SAMPLES, COMPATIBLE_SAMPLE_RATE,
    PcmFormat, PcmFrame,
};

pub const EQUALIZER_BANDS: usize = 15;
pub const MAX_FILTERS_PER_CHAIN: usize = 32;
pub const MAX_STREAMING_PROCESSORS_PER_CHAIN: usize = 1;

/// Explicit progress made by one bounded streaming processor call.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StreamingPcmProgress {
    pub consumed_samples: usize,
    pub produced_samples: usize,
}

impl StreamingPcmProgress {
    #[must_use]
    pub const fn new(consumed_samples: usize, produced_samples: usize) -> Self {
        Self {
            consumed_samples,
            produced_samples,
        }
    }
}

/// Result of asking a processing pipeline for one canonical 20 ms PCM frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamingPcmPoll {
    Frame,
    NeedInput,
    Finished,
}

/// One bounded variable-rate stage in an ordered PCM processing graph.
///
/// Input and output are canonical interleaved 48 kHz stereo samples. `process` may consume input
/// while producing no output, and callers may invoke it with empty input to drain immediately
/// available surplus. `finish` is called repeatedly after EOF until it produces zero samples.
/// Implementations own their algorithm latency but must keep it bounded and return
/// [`AudioFrameError::StreamingProcessorCapacityExceeded`] instead of allocating past that bound.
pub trait StreamingPcmProcessor: Send {
    /// Consumes some input and writes a prefix of `output`.
    ///
    /// # Errors
    ///
    /// Returns a deterministic processing or capacity error. Reported counts are validated by
    /// Mantle before they are used.
    fn process(
        &mut self,
        input: &[f32],
        output: &mut [f32],
    ) -> Result<StreamingPcmProgress, AudioFrameError>;

    /// Drains terminal latency into a prefix of `output`, returning zero when fully finished.
    ///
    /// # Errors
    ///
    /// Returns a deterministic processing or capacity error.
    fn finish(&mut self, output: &mut [f32]) -> Result<usize, AudioFrameError>;

    /// Discards all retained samples after seek, replacement, removal, or shutdown.
    fn reset(&mut self);
}

/// One allocation-stable in-place canonical PCM filter.
pub trait PcmFilter: Send {
    /// Processes one frame without retaining it.
    ///
    /// # Errors
    ///
    /// Returns an error when the frame cannot be processed under the filter's contract.
    fn process(&mut self, frame: &mut PcmFrame) -> Result<(), AudioFrameError>;

    /// Clears stream history after a seek or track reset.
    fn reset(&mut self);
}

/// Builds a fresh per-track filter chain in processing order.
pub trait PcmFilterFactory: Send + Sync {
    /// Adds zero or more filters to `builder` in processing order.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested chain cannot satisfy its resource or format contract.
    fn build(
        &self,
        format: PcmFormat,
        builder: &mut FilterChainBuilder,
    ) -> Result<(), AudioFrameError>;
}

/// Resource-bounded builder exposed to filter factories.
pub struct FilterChainBuilder {
    before: Vec<Box<dyn PcmFilter>>,
    streaming: Option<Box<dyn StreamingPcmProcessor>>,
    after: Vec<Box<dyn PcmFilter>>,
    nodes: usize,
    limit: usize,
}

impl FilterChainBuilder {
    fn new(limit: usize) -> Self {
        Self {
            before: Vec::with_capacity(limit),
            streaming: None,
            after: Vec::with_capacity(limit),
            nodes: 0,
            limit,
        }
    }

    /// Adds a filter to the end of the processing chain.
    ///
    /// # Errors
    ///
    /// Returns an error before allocation when the configured chain limit is full.
    pub fn push<F>(&mut self, filter: F) -> Result<(), AudioFrameError>
    where
        F: PcmFilter + 'static,
    {
        if self.nodes >= self.limit {
            return Err(AudioFrameError::FilterLimitExceeded { limit: self.limit });
        }
        if self.streaming.is_some() {
            self.after.push(Box::new(filter));
        } else {
            self.before.push(Box::new(filter));
        }
        self.nodes += 1;
        Ok(())
    }

    /// Inserts the single variable-rate stage at this point in processing order.
    ///
    /// Filters pushed before this call run on input blocks; filters pushed after it run on every
    /// produced block. This deliberately models the one timescale stage in Lavalink's core chain.
    ///
    /// # Errors
    ///
    /// Returns an error before mutation when the node limit is full or a streaming stage already
    /// exists.
    pub fn push_streaming<P>(&mut self, processor: P) -> Result<(), AudioFrameError>
    where
        P: StreamingPcmProcessor + 'static,
    {
        if self.nodes >= self.limit {
            return Err(AudioFrameError::FilterLimitExceeded { limit: self.limit });
        }
        if self.streaming.is_some() {
            return Err(AudioFrameError::StreamingProcessorLimitExceeded {
                limit: MAX_STREAMING_PROCESSORS_PER_CHAIN,
            });
        }
        self.streaming = Some(Box::new(processor));
        self.nodes += 1;
        Ok(())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes == 0
    }
}

/// Per-track chain whose storage and filter count are fixed outside steady-state processing.
pub struct FilterPipeline {
    format: PcmFormat,
    before: Vec<Box<dyn PcmFilter>>,
    streaming: Option<Box<dyn StreamingPcmProcessor>>,
    after: Vec<Box<dyn PcmFilter>>,
    filter_limit: usize,
    pending_input: PcmFrame,
    pending_offset: usize,
    processor_output: PcmFrame,
    processor_output_offset: usize,
    assembled: [f32; COMPATIBLE_PCM_SAMPLES],
    assembled_len: usize,
    input_finished: bool,
    processor_finished: bool,
    source_base_timestamp: Option<Duration>,
    source_timestamp_initialized: bool,
    source_frames_consumed: u64,
    output_frames_emitted: u64,
}

impl FilterPipeline {
    /// Creates an empty chain with a bounded filter count.
    ///
    /// # Errors
    ///
    /// Returns an error when `filter_limit` exceeds Mantle's global per-chain ceiling.
    pub fn new(format: PcmFormat, filter_limit: usize) -> Result<Self, AudioFrameError> {
        if filter_limit > MAX_FILTERS_PER_CHAIN {
            return Err(AudioFrameError::FilterLimitExceeded {
                limit: MAX_FILTERS_PER_CHAIN,
            });
        }
        Ok(Self {
            format,
            before: Vec::with_capacity(filter_limit),
            streaming: None,
            after: Vec::with_capacity(filter_limit),
            filter_limit,
            pending_input: PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES),
            pending_offset: 0,
            processor_output: PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES),
            processor_output_offset: 0,
            assembled: [0.0; COMPATIBLE_PCM_SAMPLES],
            assembled_len: 0,
            input_finished: false,
            processor_finished: false,
            source_base_timestamp: None,
            source_timestamp_initialized: false,
            source_frames_consumed: 0,
            output_frames_emitted: 0,
        })
    }

    /// Builds a complete replacement without changing the active graph.
    ///
    /// This lets playback orchestration complete fallible decoder/encoder resets before an
    /// infallible graph commit.
    ///
    /// # Errors
    ///
    /// Returns the factory, resource, or canonical-format error while preserving `self`.
    pub fn replacement(
        &self,
        factory: Option<&dyn PcmFilterFactory>,
    ) -> Result<Self, AudioFrameError> {
        let mut next = Self::new(self.format, self.filter_limit)?;
        let mut builder = FilterChainBuilder::new(self.filter_limit);
        if let Some(factory) = factory {
            factory.build(self.format, &mut builder)?;
        }
        if builder.streaming.is_some()
            && (self.format.sample_rate() != COMPATIBLE_SAMPLE_RATE
                || self.format.channels() != COMPATIBLE_CHANNELS)
        {
            return Err(AudioFrameError::StreamingProcessorFormatUnsupported {
                format: self.format,
            });
        }
        next.before = builder.before;
        next.streaming = builder.streaming;
        next.after = builder.after;
        Ok(next)
    }

    /// Commits a previously built replacement and deterministically resets the discarded graph.
    pub fn commit_replacement(&mut self, replacement: Self) {
        *self = replacement;
    }

    /// Rebuilds a chain and swaps it in only after the factory succeeds.
    ///
    /// Passing `None` removes user filters. Allocations and destruction happen here, never in
    /// `process`.
    ///
    /// # Errors
    ///
    /// Returns the factory or resource error while preserving the current active chain.
    pub fn install_factory(
        &mut self,
        factory: Option<&dyn PcmFilterFactory>,
    ) -> Result<(), AudioFrameError> {
        let next = self.replacement(factory)?;
        self.commit_replacement(next);
        Ok(())
    }

    /// Runs the active filters in factory order.
    ///
    /// # Errors
    ///
    /// Returns an error for a frame with the wrong format or when a filter rejects the frame.
    pub fn process(&mut self, frame: &mut PcmFrame) -> Result<(), AudioFrameError> {
        if frame.format() != Some(self.format) {
            return Err(AudioFrameError::PcmFormatMismatch {
                expected: self.format,
                actual: frame.format(),
            });
        }
        if self.streaming.is_some() {
            return Err(AudioFrameError::StreamingProcessorRequiresPull);
        }
        for filter in &mut self.before {
            filter.process(frame)?;
        }
        for filter in &mut self.after {
            filter.process(frame)?;
        }
        Ok(())
    }

    /// Submits one bounded canonical input block for streaming processing.
    ///
    /// The block is copied into fixed-capacity Mantle storage and every pre-stage fixed filter is
    /// run exactly once. A caller must drain it before submitting another block.
    ///
    /// # Errors
    ///
    /// Returns a format, capacity, state, or fixed-filter error before accepting another block.
    pub fn submit_input(&mut self, input: &PcmFrame) -> Result<(), AudioFrameError> {
        self.require_canonical_format(input.format())?;
        if self.input_finished {
            return Err(AudioFrameError::StreamingInputAlreadyFinished);
        }
        if self.pending_offset < self.pending_input.samples().len() {
            return Err(AudioFrameError::StreamingInputPending {
                samples: self.pending_input.samples().len() - self.pending_offset,
            });
        }
        if input.samples().is_empty() {
            return Ok(());
        }
        self.pending_input.copy_from_interleaved(
            input.samples(),
            self.format,
            input.timestamp(),
        )?;
        self.pending_offset = 0;
        for filter in &mut self.before {
            filter.process(&mut self.pending_input)?;
        }
        if !self.source_timestamp_initialized {
            self.source_base_timestamp = input.timestamp();
            self.source_timestamp_initialized = true;
        }
        Ok(())
    }

    /// Marks source EOF. Subsequent reads drain processor latency and then emit at most one padded
    /// terminal frame.
    pub fn finish_input(&mut self) {
        self.input_finished = true;
    }

    /// Produces one canonical 20 ms frame, requests more source input, or reports full drain.
    ///
    /// Output timestamps advance by paced output time. [`Self::source_position`] advances only by
    /// input samples the variable-rate processor reports consumed, so the two clocks intentionally
    /// differ when rate changes.
    ///
    /// # Errors
    ///
    /// Returns typed format, processor-progress, processor-capacity, or fixed-filter failures.
    pub fn read_output(
        &mut self,
        output: &mut PcmFrame,
    ) -> Result<StreamingPcmPoll, AudioFrameError> {
        self.require_canonical_format(Some(self.format))?;
        loop {
            self.append_processor_output();
            if self.assembled_len == COMPATIBLE_PCM_SAMPLES {
                self.emit_assembled(output)?;
                return Ok(StreamingPcmPoll::Frame);
            }
            if self.processor_finished {
                if self.assembled_len == 0 {
                    output.clear();
                    return Ok(StreamingPcmPoll::Finished);
                }
                self.assembled[self.assembled_len..].fill(0.0);
                self.emit_assembled(output)?;
                return Ok(StreamingPcmPoll::Frame);
            }

            if self.pending_offset < self.pending_input.samples().len() {
                self.process_pending()?;
                continue;
            }
            self.pending_input.clear();
            self.pending_offset = 0;

            if self.input_finished {
                self.flush_processor()?;
                continue;
            }

            if self.streaming.is_some() && self.process_empty()? {
                continue;
            }
            output.clear();
            return Ok(StreamingPcmPoll::NeedInput);
        }
    }

    /// Clears state in every active filter after a seek or track reset.
    pub fn reset(&mut self) {
        for filter in &mut self.before {
            filter.reset();
        }
        if let Some(processor) = self.streaming.as_mut() {
            processor.reset();
        }
        for filter in &mut self.after {
            filter.reset();
        }
        self.pending_input.clear();
        self.pending_offset = 0;
        self.processor_output.clear();
        self.processor_output_offset = 0;
        self.assembled.fill(0.0);
        self.assembled_len = 0;
        self.input_finished = false;
        self.processor_finished = false;
        self.source_base_timestamp = None;
        self.source_timestamp_initialized = false;
        self.source_frames_consumed = 0;
        self.output_frames_emitted = 0;
    }

    #[must_use]
    pub fn filter_count(&self) -> usize {
        self.before.len() + self.after.len() + usize::from(self.streaming.is_some())
    }

    #[must_use]
    pub const fn has_streaming_processor(&self) -> bool {
        self.streaming.is_some()
    }

    /// Source-media position immediately after the latest consumed canonical input sample.
    #[must_use]
    pub fn source_position(&self) -> Option<Duration> {
        self.source_base_timestamp
            .map(|base| base.saturating_add(frames_to_duration(self.source_frames_consumed)))
    }

    fn require_canonical_format(&self, actual: Option<PcmFormat>) -> Result<(), AudioFrameError> {
        let canonical = PcmFormat::new(COMPATIBLE_SAMPLE_RATE, COMPATIBLE_CHANNELS)
            .expect("canonical PCM format is valid");
        if self.format != canonical {
            return Err(AudioFrameError::StreamingProcessorFormatUnsupported {
                format: self.format,
            });
        }
        if actual != Some(self.format) {
            return Err(AudioFrameError::PcmFormatMismatch {
                expected: self.format,
                actual,
            });
        }
        Ok(())
    }

    fn append_processor_output(&mut self) {
        let available = &self.processor_output.samples()[self.processor_output_offset..];
        let copied = available
            .len()
            .min(COMPATIBLE_PCM_SAMPLES - self.assembled_len);
        self.assembled[self.assembled_len..self.assembled_len + copied]
            .copy_from_slice(&available[..copied]);
        self.assembled_len += copied;
        self.processor_output_offset += copied;
        if self.processor_output_offset == self.processor_output.samples().len() {
            self.processor_output.clear();
            self.processor_output_offset = 0;
        }
    }

    fn process_pending(&mut self) -> Result<(), AudioFrameError> {
        if self.streaming.is_none() {
            let input = &self.pending_input.samples()[self.pending_offset..];
            self.processor_output
                .copy_from_interleaved(input, self.format, None)?;
            self.advance_source(input.len());
            self.pending_offset = self.pending_input.samples().len();
            self.process_after()?;
            return Ok(());
        }

        let input_len = self.pending_input.samples().len() - self.pending_offset;
        let output = self
            .processor_output
            .prepare(COMPATIBLE_PCM_SAMPLES, self.format, None)?;
        let progress = match self
            .streaming
            .as_mut()
            .expect("streaming processor checked")
            .process(&self.pending_input.samples()[self.pending_offset..], output)
        {
            Ok(progress) => progress,
            Err(error) => {
                self.processor_output.clear();
                return Err(error);
            }
        };
        self.validate_progress(progress, input_len)?;
        self.processor_output
            .prepare(progress.produced_samples, self.format, None)?;
        self.pending_offset += progress.consumed_samples;
        self.advance_source(progress.consumed_samples);
        if progress.produced_samples != 0 {
            self.process_after()?;
        }
        Ok(())
    }

    fn process_empty(&mut self) -> Result<bool, AudioFrameError> {
        let output = self
            .processor_output
            .prepare(COMPATIBLE_PCM_SAMPLES, self.format, None)?;
        let progress = match self
            .streaming
            .as_mut()
            .expect("streaming processor checked")
            .process(&[], output)
        {
            Ok(progress) => progress,
            Err(error) => {
                self.processor_output.clear();
                return Err(error);
            }
        };
        self.validate_progress(progress, 0)?;
        self.processor_output
            .prepare(progress.produced_samples, self.format, None)?;
        if progress.produced_samples != 0 {
            self.process_after()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn flush_processor(&mut self) -> Result<(), AudioFrameError> {
        let Some(processor) = self.streaming.as_mut() else {
            self.processor_finished = true;
            return Ok(());
        };
        let output = self
            .processor_output
            .prepare(COMPATIBLE_PCM_SAMPLES, self.format, None)?;
        let produced = match processor.finish(output) {
            Ok(produced) => produced,
            Err(error) => {
                self.processor_output.clear();
                return Err(error);
            }
        };
        let progress = StreamingPcmProgress::new(0, produced);
        self.validate_progress(progress, 0)?;
        self.processor_output.prepare(produced, self.format, None)?;
        if produced == 0 {
            self.processor_finished = true;
        } else {
            self.process_after()?;
        }
        Ok(())
    }

    fn process_after(&mut self) -> Result<(), AudioFrameError> {
        for filter in &mut self.after {
            if let Err(error) = filter.process(&mut self.processor_output) {
                self.processor_output.clear();
                return Err(error);
            }
        }
        Ok(())
    }

    fn validate_progress(
        &mut self,
        progress: StreamingPcmProgress,
        input_samples: usize,
    ) -> Result<(), AudioFrameError> {
        let channels = usize::from(self.format.channels());
        if progress.consumed_samples > input_samples
            || progress.produced_samples > COMPATIBLE_PCM_SAMPLES
            || !progress.consumed_samples.is_multiple_of(channels)
            || !progress.produced_samples.is_multiple_of(channels)
        {
            self.processor_output.clear();
            return Err(AudioFrameError::InvalidStreamingProcessorProgress {
                consumed: progress.consumed_samples,
                produced: progress.produced_samples,
                input: input_samples,
                output_capacity: COMPATIBLE_PCM_SAMPLES,
            });
        }
        if input_samples != 0 && progress.consumed_samples == 0 && progress.produced_samples == 0 {
            self.processor_output.clear();
            return Err(AudioFrameError::StreamingProcessorStalled);
        }
        Ok(())
    }

    fn advance_source(&mut self, interleaved_samples: usize) {
        let frames = interleaved_samples / usize::from(self.format.channels());
        self.source_frames_consumed = self
            .source_frames_consumed
            .saturating_add(u64::try_from(frames).unwrap_or(u64::MAX));
    }

    fn emit_assembled(&mut self, output: &mut PcmFrame) -> Result<(), AudioFrameError> {
        let timestamp = self.source_base_timestamp.map(|base| {
            base.saturating_add(frames_to_duration(
                self.output_frames_emitted
                    .saturating_mul(u64::try_from(super::COMPATIBLE_SAMPLES_PER_CHANNEL).unwrap()),
            ))
        });
        output.copy_from_interleaved(&self.assembled, self.format, timestamp)?;
        self.assembled.fill(0.0);
        self.assembled_len = 0;
        self.output_frames_emitted = self.output_frames_emitted.saturating_add(1);
        Ok(())
    }
}

impl Drop for FilterPipeline {
    fn drop(&mut self) {
        self.reset();
    }
}

fn frames_to_duration(frames: u64) -> Duration {
    let nanos =
        u128::from(frames).saturating_mul(1_000_000_000) / u128::from(COMPATIBLE_SAMPLE_RATE);
    Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
}

/// A live-updatable Lavaplayer-compatible 15-band equalizer factory.
#[derive(Clone)]
pub struct EqualizerFactory {
    gains: Arc<EqualizerGains>,
}

impl EqualizerFactory {
    #[must_use]
    pub fn new() -> Self {
        Self {
            gains: Arc::new(EqualizerGains::new()),
        }
    }

    /// Sets and clamps one band to Lavaplayer's `-0.25..=1.0` range.
    ///
    /// An out-of-range band has no effect. Filters already built by this factory observe updates.
    pub fn set_gain(&self, band: usize, gain: f32) {
        if let Some(value) = self.gains.values.get(band) {
            value.store(gain.clamp(-0.25, 1.0).to_bits(), Ordering::Relaxed);
        }
    }

    /// Returns one band gain, or zero for an out-of-range band.
    #[must_use]
    pub fn gain(&self, band: usize) -> f32 {
        self.gains
            .values
            .get(band)
            .map_or(0.0, |value| f32::from_bits(value.load(Ordering::Relaxed)))
    }
}

impl Default for EqualizerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl PcmFilterFactory for EqualizerFactory {
    fn build(
        &self,
        format: PcmFormat,
        builder: &mut FilterChainBuilder,
    ) -> Result<(), AudioFrameError> {
        if format.sample_rate() == COMPATIBLE_SAMPLE_RATE {
            builder.push(Equalizer::new(format.channels(), Arc::clone(&self.gains)))?;
        }
        Ok(())
    }
}

struct EqualizerGains {
    values: [AtomicU32; EQUALIZER_BANDS],
}

impl EqualizerGains {
    fn new() -> Self {
        Self {
            values: std::array::from_fn(|_| AtomicU32::new(0.0_f32.to_bits())),
        }
    }

    fn get(&self, band: usize) -> f32 {
        f32::from_bits(self.values[band].load(Ordering::Relaxed))
    }
}

struct Equalizer {
    channels: [ChannelProcessor; 2],
    channel_count: usize,
    gains: Arc<EqualizerGains>,
}

impl Equalizer {
    fn new(channels: u16, gains: Arc<EqualizerGains>) -> Self {
        Self {
            channels: std::array::from_fn(|_| ChannelProcessor::new()),
            channel_count: usize::from(channels),
            gains,
        }
    }
}

impl PcmFilter for Equalizer {
    fn process(&mut self, frame: &mut PcmFrame) -> Result<(), AudioFrameError> {
        let gains = &self.gains;
        for channel_index in 0..self.channel_count {
            let processor = &mut self.channels[channel_index];
            for sample in frame.samples_mut()[channel_index..]
                .iter_mut()
                .step_by(self.channel_count)
            {
                *sample = processor.process(*sample, gains);
            }
        }
        Ok(())
    }

    fn reset(&mut self) {
        for channel in &mut self.channels[..self.channel_count] {
            channel.reset();
        }
    }
}

struct ChannelProcessor {
    history: [f32; EQUALIZER_BANDS * 6],
    current: usize,
    minus_one: usize,
    minus_two: usize,
}

impl ChannelProcessor {
    fn new() -> Self {
        Self {
            history: [0.0; EQUALIZER_BANDS * 6],
            current: 0,
            minus_one: 2,
            minus_two: 1,
        }
    }

    fn process(&mut self, sample: f32, gains: &EqualizerGains) -> f32 {
        let mut result = sample * 0.25;
        for (band, coefficients) in COEFFICIENTS_48KHZ.iter().enumerate() {
            let input_history = band * 6;
            let output_history = input_history + 3;
            let band_result = coefficients.alpha
                * (sample - self.history[input_history + self.minus_two])
                + coefficients.gamma * self.history[output_history + self.minus_one]
                - coefficients.beta * self.history[output_history + self.minus_two];
            self.history[input_history + self.current] = sample;
            self.history[output_history + self.current] = band_result;
            result += band_result * gains.get(band);
        }
        self.current = (self.current + 1) % 3;
        self.minus_one = (self.minus_one + 1) % 3;
        self.minus_two = (self.minus_two + 1) % 3;
        (result * 4.0).clamp(-1.0, 1.0)
    }

    fn reset(&mut self) {
        self.history.fill(0.0);
    }
}

#[derive(Clone, Copy)]
struct Coefficients {
    beta: f32,
    alpha: f32,
    gamma: f32,
}

const COEFFICIENTS_48KHZ: [Coefficients; EQUALIZER_BANDS] = [
    Coefficients {
        beta: 9.984_755e-1,
        alpha: 7.622_667e-4,
        gamma: 1.998_464_8,
    },
    Coefficients {
        beta: 9.975_618e-1,
        alpha: 1.219_076_7e-3,
        gamma: 1.997_534_5,
    },
    Coefficients {
        beta: 9.961_626e-1,
        alpha: 1.918_693_1e-3,
        gamma: 1.996_094_7,
    },
    Coefficients {
        beta: 9.939_158e-1,
        alpha: 3.042_107_2e-3,
        gamma: 1.993_745,
    },
    Coefficients {
        beta: 9.902_831e-1,
        alpha: 4.858_464e-3,
        gamma: 1.989_846_6,
    },
    Coefficients {
        beta: 9.848_59e-1,
        alpha: 7.570_513_5e-3,
        gamma: 1.983_796_2,
    },
    Coefficients {
        beta: 9.758_851e-1,
        alpha: 1.205_743_7e-2,
        gamma: 1.973_177_2,
    },
    Coefficients {
        beta: 9.622_852e-1,
        alpha: 1.885_739e-2,
        gamma: 1.955_616_5,
    },
    Coefficients {
        beta: 9.408_093e-1,
        alpha: 2.959_533_4e-2,
        gamma: 1.924_205_4,
    },
    Coefficients {
        beta: 9.070_206e-1,
        alpha: 4.648_970_4e-2,
        gamma: 1.865_347_6,
    },
    Coefficients {
        beta: 8.586_8e-1,
        alpha: 7.065_998e-2,
        gamma: 1.760_040_2,
    },
    Coefficients {
        beta: 7.840_961e-1,
        alpha: 1.079_519_5e-1,
        gamma: 1.545_072_6,
    },
    Coefficients {
        beta: 6.833_286e-1,
        alpha: 1.583_357e-1,
        gamma: 1.142_644_8,
    },
    Coefficients {
        beta: 5.526_752e-1,
        alpha: 2.236_624e-1,
        gamma: 4.018_619e-1,
    },
    Coefficients {
        beta: 4.181_189e-1,
        alpha: 2.909_405_5e-1,
        gamma: -7.090_594e-1,
    },
];

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use super::{
        EqualizerFactory, FilterChainBuilder, FilterPipeline, PcmFilter, PcmFilterFactory,
        StreamingPcmPoll, StreamingPcmProcessor, StreamingPcmProgress,
    };
    use crate::{AudioFrameError, COMPATIBLE_PCM_SAMPLES, PcmFormat, PcmFrame};

    struct Add(f32);

    impl PcmFilter for Add {
        fn process(&mut self, frame: &mut PcmFrame) -> Result<(), AudioFrameError> {
            for sample in frame.samples_mut() {
                *sample += self.0;
            }
            Ok(())
        }

        fn reset(&mut self) {}
    }

    struct OrderedFactory;

    impl PcmFilterFactory for OrderedFactory {
        fn build(
            &self,
            _format: PcmFormat,
            builder: &mut FilterChainBuilder,
        ) -> Result<(), AudioFrameError> {
            builder.push(Add(1.0))?;
            builder.push(Multiply(2.0))
        }
    }

    struct Multiply(f32);

    impl PcmFilter for Multiply {
        fn process(&mut self, frame: &mut PcmFrame) -> Result<(), AudioFrameError> {
            for sample in frame.samples_mut() {
                *sample *= self.0;
            }
            Ok(())
        }

        fn reset(&mut self) {}
    }

    struct OversizedFactory;

    impl PcmFilterFactory for OversizedFactory {
        fn build(
            &self,
            _format: PcmFormat,
            builder: &mut FilterChainBuilder,
        ) -> Result<(), AudioFrameError> {
            builder.push(Multiply(3.0))?;
            builder.push(Multiply(4.0))?;
            builder.push(Multiply(5.0))
        }
    }

    #[derive(Clone, Copy)]
    enum TestRate {
        DoubleSpeed,
        HalfSpeed,
    }

    struct RateProcessor {
        rate: TestRate,
        source_phase: bool,
        duplicate: Option<[f32; 2]>,
    }

    impl RateProcessor {
        fn new(rate: TestRate) -> Self {
            Self {
                rate,
                source_phase: false,
                duplicate: None,
            }
        }
    }

    impl StreamingPcmProcessor for RateProcessor {
        fn process(
            &mut self,
            input: &[f32],
            output: &mut [f32],
        ) -> Result<StreamingPcmProgress, AudioFrameError> {
            let mut consumed = 0;
            let mut produced = 0;
            match self.rate {
                TestRate::DoubleSpeed => {
                    while consumed + 2 <= input.len() && produced + 2 <= output.len() {
                        if !self.source_phase {
                            output[produced..produced + 2]
                                .copy_from_slice(&input[consumed..consumed + 2]);
                            produced += 2;
                        }
                        self.source_phase = !self.source_phase;
                        consumed += 2;
                    }
                }
                TestRate::HalfSpeed => {
                    while produced + 2 <= output.len() {
                        if let Some(frame) = self.duplicate.take() {
                            output[produced..produced + 2].copy_from_slice(&frame);
                            produced += 2;
                            continue;
                        }
                        if consumed + 2 > input.len() {
                            break;
                        }
                        let frame = [input[consumed], input[consumed + 1]];
                        output[produced..produced + 2].copy_from_slice(&frame);
                        self.duplicate = Some(frame);
                        consumed += 2;
                        produced += 2;
                    }
                }
            }
            Ok(StreamingPcmProgress::new(consumed, produced))
        }

        fn finish(&mut self, output: &mut [f32]) -> Result<usize, AudioFrameError> {
            if let Some(frame) = self.duplicate.take() {
                output[..2].copy_from_slice(&frame);
                Ok(2)
            } else {
                Ok(0)
            }
        }

        fn reset(&mut self) {
            self.source_phase = false;
            self.duplicate = None;
        }
    }

    struct RateFactory(TestRate);

    impl PcmFilterFactory for RateFactory {
        fn build(
            &self,
            _format: PcmFormat,
            builder: &mut FilterChainBuilder,
        ) -> Result<(), AudioFrameError> {
            builder.push_streaming(RateProcessor::new(self.0))
        }
    }

    struct TailProcessor {
        tail_emitted: bool,
    }

    impl StreamingPcmProcessor for TailProcessor {
        fn process(
            &mut self,
            input: &[f32],
            output: &mut [f32],
        ) -> Result<StreamingPcmProgress, AudioFrameError> {
            let copied = input.len().min(output.len());
            output[..copied].copy_from_slice(&input[..copied]);
            Ok(StreamingPcmProgress::new(copied, copied))
        }

        fn finish(&mut self, output: &mut [f32]) -> Result<usize, AudioFrameError> {
            if self.tail_emitted {
                return Ok(0);
            }
            output[..2].fill(0.75);
            self.tail_emitted = true;
            Ok(2)
        }

        fn reset(&mut self) {
            self.tail_emitted = false;
        }
    }

    struct TailFactory;

    impl PcmFilterFactory for TailFactory {
        fn build(
            &self,
            _format: PcmFormat,
            builder: &mut FilterChainBuilder,
        ) -> Result<(), AudioFrameError> {
            builder.push_streaming(TailProcessor {
                tail_emitted: false,
            })
        }
    }

    struct ScriptedSurplusProcessor {
        calls: usize,
    }

    impl StreamingPcmProcessor for ScriptedSurplusProcessor {
        fn process(
            &mut self,
            input: &[f32],
            output: &mut [f32],
        ) -> Result<StreamingPcmProgress, AudioFrameError> {
            if input.is_empty() {
                return Ok(StreamingPcmProgress::default());
            }
            self.calls += 1;
            let produced = if self.calls == 1 { 1_000 } else { output.len() };
            let multiplier = if self.calls == 1 { 1.0 } else { 2.0 };
            output[..produced].fill(input[0] * multiplier);
            Ok(StreamingPcmProgress::new(input.len(), produced))
        }

        fn finish(&mut self, _output: &mut [f32]) -> Result<usize, AudioFrameError> {
            Ok(0)
        }

        fn reset(&mut self) {
            self.calls = 0;
        }
    }

    struct OrderedStreamingFactory;

    impl PcmFilterFactory for OrderedStreamingFactory {
        fn build(
            &self,
            _format: PcmFormat,
            builder: &mut FilterChainBuilder,
        ) -> Result<(), AudioFrameError> {
            builder.push(Add(1.0))?;
            builder.push_streaming(ScriptedSurplusProcessor { calls: 0 })?;
            builder.push(Multiply(2.0))
        }
    }

    struct CapacityProcessor;

    impl StreamingPcmProcessor for CapacityProcessor {
        fn process(
            &mut self,
            _input: &[f32],
            output: &mut [f32],
        ) -> Result<StreamingPcmProgress, AudioFrameError> {
            Err(AudioFrameError::StreamingProcessorCapacityExceeded {
                required: output.len() + 2,
                capacity: output.len(),
            })
        }

        fn finish(&mut self, _output: &mut [f32]) -> Result<usize, AudioFrameError> {
            Ok(0)
        }

        fn reset(&mut self) {}
    }

    struct CapacityFactory;

    impl PcmFilterFactory for CapacityFactory {
        fn build(
            &self,
            _format: PcmFormat,
            builder: &mut FilterChainBuilder,
        ) -> Result<(), AudioFrameError> {
            builder.push_streaming(CapacityProcessor)
        }
    }

    struct ResetProcessor(Arc<AtomicUsize>);

    impl StreamingPcmProcessor for ResetProcessor {
        fn process(
            &mut self,
            input: &[f32],
            output: &mut [f32],
        ) -> Result<StreamingPcmProgress, AudioFrameError> {
            let copied = input.len().min(output.len());
            output[..copied].copy_from_slice(&input[..copied]);
            Ok(StreamingPcmProgress::new(copied, copied))
        }

        fn finish(&mut self, _output: &mut [f32]) -> Result<usize, AudioFrameError> {
            Ok(0)
        }

        fn reset(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    struct ResetFactory(Arc<AtomicUsize>);

    impl PcmFilterFactory for ResetFactory {
        fn build(
            &self,
            _format: PcmFormat,
            builder: &mut FilterChainBuilder,
        ) -> Result<(), AudioFrameError> {
            builder.push_streaming(ResetProcessor(Arc::clone(&self.0)))
        }
    }

    #[test]
    fn factory_order_limit_and_atomic_replacement_are_explicit() {
        let format = PcmFormat::new(48_000, 1).unwrap();
        let mut pipeline = FilterPipeline::new(format, 2).unwrap();
        pipeline.install_factory(Some(&OrderedFactory)).unwrap();
        assert_eq!(pipeline.filter_count(), 2);

        let mut first = frame(format, &[1.0]);
        pipeline.process(&mut first).unwrap();
        assert_eq!(first.samples(), [4.0]);

        assert!(matches!(
            pipeline.install_factory(Some(&OversizedFactory)),
            Err(AudioFrameError::FilterLimitExceeded { limit: 2 })
        ));
        let mut second = frame(format, &[1.0]);
        pipeline.process(&mut second).unwrap();
        assert_eq!(second.samples(), [4.0]);

        pipeline.install_factory(None).unwrap();
        assert_eq!(pipeline.filter_count(), 0);
    }

    #[test]
    fn equalizer_is_live_clamped_resettable_and_48khz_only() {
        let factory = EqualizerFactory::new();
        factory.set_gain(0, 2.0);
        factory.set_gain(1, -2.0);
        factory.set_gain(99, 0.5);
        assert_eq!(factory.gain(0).to_bits(), 1.0_f32.to_bits());
        assert_eq!(factory.gain(1).to_bits(), (-0.25_f32).to_bits());
        assert_eq!(factory.gain(99).to_bits(), 0.0_f32.to_bits());
        factory.set_gain(1, 0.0);

        let format = PcmFormat::new(48_000, 2).unwrap();
        let mut pipeline = FilterPipeline::new(format, 1).unwrap();
        pipeline.install_factory(Some(&factory)).unwrap();
        assert_eq!(pipeline.filter_count(), 1);
        let input = [0.25, -0.25, 0.0, 0.0];
        let mut first = frame(format, &input);
        pipeline.process(&mut first).unwrap();
        assert!((first.samples()[0] - 0.250_762_25).abs() < 1.0e-7);
        assert!((first.samples()[1] + 0.250_762_25).abs() < 1.0e-7);

        pipeline.reset();
        let mut second = frame(format, &input);
        pipeline.process(&mut second).unwrap();
        assert_eq!(first.samples(), second.samples());

        let other_format = PcmFormat::new(44_100, 2).unwrap();
        let mut incompatible = FilterPipeline::new(other_format, 1).unwrap();
        incompatible.install_factory(Some(&factory)).unwrap();
        assert_eq!(incompatible.filter_count(), 0);
    }

    #[test]
    fn pipeline_rejects_wrong_or_uninitialized_frame_format() {
        let format = PcmFormat::new(48_000, 2).unwrap();
        let mut pipeline = FilterPipeline::new(format, 1).unwrap();
        let mut empty = PcmFrame::with_capacity(2);
        assert!(matches!(
            pipeline.process(&mut empty),
            Err(AudioFrameError::PcmFormatMismatch { actual: None, .. })
        ));
        let mut wrong = frame(PcmFormat::new(44_100, 2).unwrap(), &[0.0, 0.0]);
        assert!(matches!(
            pipeline.process(&mut wrong),
            Err(AudioFrameError::PcmFormatMismatch { .. })
        ));
    }

    #[test]
    fn variable_rate_changes_paced_duration_and_keeps_both_clocks_monotonic() {
        let fast = run_rate(TestRate::DoubleSpeed);
        let slow = run_rate(TestRate::HalfSpeed);
        assert_eq!(fast.output_frames, 25);
        assert_eq!(slow.output_frames, 100);
        assert_eq!(fast.last_output_timestamp, Duration::from_millis(480));
        assert_eq!(slow.last_output_timestamp, Duration::from_millis(1_980));
        assert_eq!(fast.source_position, Duration::from_secs(1));
        assert_eq!(slow.source_position, Duration::from_secs(1));
    }

    #[test]
    fn finish_emits_terminal_samples_before_one_final_padding() {
        let format = canonical_format();
        let mut pipeline = FilterPipeline::new(format, 1).unwrap();
        pipeline.install_factory(Some(&TailFactory)).unwrap();
        let input = frame_with_timestamp(format, &[0.25; 200], Duration::ZERO);
        pipeline.submit_input(&input).unwrap();
        pipeline.finish_input();
        let mut output = PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES);
        assert_eq!(
            pipeline.read_output(&mut output).unwrap(),
            StreamingPcmPoll::Frame
        );
        assert_eq!(&output.samples()[..200], &[0.25; 200]);
        assert_eq!(&output.samples()[200..202], &[0.75; 2]);
        assert!(output.samples()[202..].iter().all(|sample| *sample == 0.0));
        assert_eq!(
            pipeline.read_output(&mut output).unwrap(),
            StreamingPcmPoll::Finished
        );
    }

    #[test]
    fn surplus_is_retained_across_reads_and_fixed_filters_keep_graph_order() {
        let format = canonical_format();
        let mut pipeline = FilterPipeline::new(format, 3).unwrap();
        pipeline
            .install_factory(Some(&OrderedStreamingFactory))
            .unwrap();
        let input = frame_with_timestamp(format, &[0.0; 2], Duration::ZERO);
        pipeline.submit_input(&input).unwrap();
        let mut output = PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES);
        assert_eq!(
            pipeline.read_output(&mut output).unwrap(),
            StreamingPcmPoll::NeedInput
        );
        pipeline.submit_input(&input).unwrap();
        assert_eq!(
            pipeline.read_output(&mut output).unwrap(),
            StreamingPcmPoll::Frame
        );
        assert!(
            output.samples()[..1_000]
                .iter()
                .all(|sample| sample.to_bits() == 2.0_f32.to_bits())
        );
        assert!(
            output.samples()[1_000..]
                .iter()
                .all(|sample| sample.to_bits() == 4.0_f32.to_bits())
        );
        pipeline.finish_input();
        assert_eq!(
            pipeline.read_output(&mut output).unwrap(),
            StreamingPcmPoll::Frame
        );
        assert!(
            output.samples()[..1_000]
                .iter()
                .all(|sample| sample.to_bits() == 4.0_f32.to_bits())
        );
        assert!(
            output.samples()[1_000..]
                .iter()
                .all(|sample| *sample == 0.0)
        );
    }

    #[test]
    fn reset_replacement_drop_and_capacity_failures_are_deterministic() {
        let format = canonical_format();
        let resets = Arc::new(AtomicUsize::new(0));
        {
            let mut pipeline = FilterPipeline::new(format, 1).unwrap();
            pipeline
                .install_factory(Some(&ResetFactory(Arc::clone(&resets))))
                .unwrap();
            pipeline.reset();
            assert_eq!(resets.load(Ordering::Relaxed), 1);
            pipeline.install_factory(None).unwrap();
            assert_eq!(resets.load(Ordering::Relaxed), 2);
        }

        let mut pipeline = FilterPipeline::new(format, 1).unwrap();
        pipeline.install_factory(Some(&CapacityFactory)).unwrap();
        pipeline
            .submit_input(&frame_with_timestamp(format, &[0.0; 2], Duration::ZERO))
            .unwrap();
        let mut output = PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES);
        assert_eq!(
            pipeline.read_output(&mut output),
            Err(AudioFrameError::StreamingProcessorCapacityExceeded {
                required: COMPATIBLE_PCM_SAMPLES + 2,
                capacity: COMPATIBLE_PCM_SAMPLES,
            })
        );
        assert_eq!(
            pipeline.read_output(&mut output),
            Err(AudioFrameError::StreamingProcessorCapacityExceeded {
                required: COMPATIBLE_PCM_SAMPLES + 2,
                capacity: COMPATIBLE_PCM_SAMPLES,
            })
        );
    }

    #[test]
    fn first_absent_timestamp_keeps_stream_clocks_absent() {
        let format = canonical_format();
        let mut pipeline = FilterPipeline::new(format, 1).unwrap();
        pipeline
            .install_factory(Some(&RateFactory(TestRate::DoubleSpeed)))
            .unwrap();
        pipeline
            .submit_input(&frame(format, &[0.125; COMPATIBLE_PCM_SAMPLES]))
            .unwrap();
        let mut output = PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES);
        assert_eq!(
            pipeline.read_output(&mut output).unwrap(),
            StreamingPcmPoll::NeedInput
        );
        pipeline
            .submit_input(&frame_with_timestamp(
                format,
                &[0.125; COMPATIBLE_PCM_SAMPLES],
                Duration::from_secs(5),
            ))
            .unwrap();
        assert_eq!(
            pipeline.read_output(&mut output).unwrap(),
            StreamingPcmPoll::Frame
        );
        assert_eq!(output.timestamp(), None);
        assert_eq!(pipeline.source_position(), None);
    }

    struct RateRun {
        output_frames: usize,
        last_output_timestamp: Duration,
        source_position: Duration,
    }

    fn run_rate(rate: TestRate) -> RateRun {
        let format = canonical_format();
        let mut pipeline = FilterPipeline::new(format, 1).unwrap();
        pipeline.install_factory(Some(&RateFactory(rate))).unwrap();
        let input = vec![0.125; COMPATIBLE_PCM_SAMPLES];
        let mut output = PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES);
        let mut output_frames = 0;
        let mut last_timestamp = None;
        let mut previous_source = Duration::ZERO;
        for block in 0_u32..50 {
            let timestamp = Duration::from_millis(u64::from(block) * 20);
            pipeline
                .submit_input(&frame_with_timestamp(format, &input, timestamp))
                .unwrap();
            loop {
                match pipeline.read_output(&mut output).unwrap() {
                    StreamingPcmPoll::Frame => {
                        let timestamp = output.timestamp().unwrap();
                        if let Some(previous) = last_timestamp {
                            assert_eq!(timestamp, previous + Duration::from_millis(20));
                        }
                        let source = pipeline.source_position().unwrap();
                        assert!(source >= previous_source);
                        previous_source = source;
                        last_timestamp = Some(timestamp);
                        output_frames += 1;
                    }
                    StreamingPcmPoll::NeedInput => break,
                    StreamingPcmPoll::Finished => panic!("pipeline ended before finish"),
                }
            }
        }
        pipeline.finish_input();
        loop {
            match pipeline.read_output(&mut output).unwrap() {
                StreamingPcmPoll::Frame => {
                    let timestamp = output.timestamp().unwrap();
                    if let Some(previous) = last_timestamp {
                        assert_eq!(timestamp, previous + Duration::from_millis(20));
                    }
                    last_timestamp = Some(timestamp);
                    output_frames += 1;
                }
                StreamingPcmPoll::Finished => break,
                StreamingPcmPoll::NeedInput => panic!("finished pipeline requested input"),
            }
        }
        RateRun {
            output_frames,
            last_output_timestamp: last_timestamp.unwrap(),
            source_position: pipeline.source_position().unwrap(),
        }
    }

    fn canonical_format() -> PcmFormat {
        PcmFormat::new(48_000, 2).unwrap()
    }

    fn frame_with_timestamp(format: PcmFormat, samples: &[f32], timestamp: Duration) -> PcmFrame {
        let mut frame = PcmFrame::with_capacity(samples.len());
        frame
            .copy_from_interleaved(samples, format, Some(timestamp))
            .unwrap();
        frame
    }

    fn frame(format: PcmFormat, samples: &[f32]) -> PcmFrame {
        let mut frame = PcmFrame::with_capacity(samples.len());
        frame.copy_from_interleaved(samples, format, None).unwrap();
        frame
    }
}
