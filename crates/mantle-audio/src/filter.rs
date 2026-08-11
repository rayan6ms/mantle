use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use super::{AudioFrameError, COMPATIBLE_SAMPLE_RATE, PcmFormat, PcmFrame};

pub const EQUALIZER_BANDS: usize = 15;
pub const MAX_FILTERS_PER_CHAIN: usize = 32;

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
    filters: Vec<Box<dyn PcmFilter>>,
    limit: usize,
}

impl FilterChainBuilder {
    fn new(limit: usize) -> Self {
        Self {
            filters: Vec::with_capacity(limit),
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
        if self.filters.len() >= self.limit {
            return Err(AudioFrameError::FilterLimitExceeded { limit: self.limit });
        }
        self.filters.push(Box::new(filter));
        Ok(())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.filters.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }
}

/// Per-track chain whose storage and filter count are fixed outside steady-state processing.
pub struct FilterPipeline {
    format: PcmFormat,
    filters: Vec<Box<dyn PcmFilter>>,
    filter_limit: usize,
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
            filters: Vec::with_capacity(filter_limit),
            filter_limit,
        })
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
        let mut next = FilterChainBuilder::new(self.filter_limit);
        if let Some(factory) = factory {
            factory.build(self.format, &mut next)?;
        }
        self.filters = next.filters;
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
        for filter in &mut self.filters {
            filter.process(frame)?;
        }
        Ok(())
    }

    /// Clears state in every active filter after a seek or track reset.
    pub fn reset(&mut self) {
        for filter in &mut self.filters {
            filter.reset();
        }
    }

    #[must_use]
    pub fn filter_count(&self) -> usize {
        self.filters.len()
    }
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
    use super::{
        EqualizerFactory, FilterChainBuilder, FilterPipeline, PcmFilter, PcmFilterFactory,
    };
    use crate::{AudioFrameError, PcmFormat, PcmFrame};

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

    fn frame(format: PcmFormat, samples: &[f32]) -> PcmFrame {
        let mut frame = PcmFrame::with_capacity(samples.len());
        frame.copy_from_interleaved(samples, format, None).unwrap();
        frame
    }
}
