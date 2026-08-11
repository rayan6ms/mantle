use std::time::Duration;

use mantle_opus::packet_samples;

use super::{
    AudioFrameError, COMPATIBLE_CHANNELS, COMPATIBLE_SAMPLE_RATE, EncodedFrameSlot,
    MAX_COMPATIBLE_OPUS_FRAME_BYTES, PcmFormat, VolumeLevel,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpusPipelineMode {
    Passthrough,
    Transcode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpusModeTransition {
    EnabledPassthrough,
    DisabledPassthrough,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpusPacketRoute {
    pub mode: OpusPipelineMode,
    pub transition: Option<OpusModeTransition>,
}

impl OpusPacketRoute {
    #[must_use]
    pub const fn delivered(self) -> bool {
        matches!(self.mode, OpusPipelineMode::Passthrough)
    }
}

/// Stateful direct-packet eligibility and delivery for one Opus track.
pub struct OpusPassthrough {
    input_format: PcmFormat,
    volume: VolumeLevel,
    filters_active: bool,
    mode: OpusPipelineMode,
}

impl OpusPassthrough {
    #[must_use]
    pub const fn new(input_format: PcmFormat) -> Self {
        Self {
            input_format,
            volume: VolumeLevel::NORMAL,
            filters_active: false,
            mode: OpusPipelineMode::Transcode,
        }
    }

    pub fn set_input_format(&mut self, input_format: PcmFormat) {
        self.input_format = input_format;
    }

    pub fn set_volume(&mut self, volume: VolumeLevel) {
        self.volume = volume;
    }

    pub fn set_filters_active(&mut self, active: bool) {
        self.filters_active = active;
    }

    /// Classifies and, when compatible, copies one packet into fixed inline output storage.
    ///
    /// Ineligible packets are left available to the caller for decoding and do not mutate
    /// `output`. This method performs no allocation.
    ///
    /// # Errors
    ///
    /// Returns an error only if the fixed output-slot contract rejects a packet that passed the
    /// stricter eligibility checks.
    pub fn route_packet(
        &mut self,
        packet: &[u8],
        timestamp: Option<Duration>,
        output: &mut EncodedFrameSlot,
    ) -> Result<OpusPacketRoute, AudioFrameError> {
        let next_mode = if self.is_eligible(packet) {
            OpusPipelineMode::Passthrough
        } else {
            OpusPipelineMode::Transcode
        };
        let transition = match (self.mode, next_mode) {
            (OpusPipelineMode::Transcode, OpusPipelineMode::Passthrough) => {
                Some(OpusModeTransition::EnabledPassthrough)
            }
            (OpusPipelineMode::Passthrough, OpusPipelineMode::Transcode) => {
                Some(OpusModeTransition::DisabledPassthrough)
            }
            _ => None,
        };
        if next_mode == OpusPipelineMode::Passthrough {
            output.write(packet, timestamp, VolumeLevel::NORMAL)?;
        }
        self.mode = next_mode;
        Ok(OpusPacketRoute {
            mode: next_mode,
            transition,
        })
    }

    /// Clears transition history after a seek or track reset.
    pub fn reset(&mut self) {
        self.mode = OpusPipelineMode::Transcode;
    }

    #[must_use]
    pub const fn mode(&self) -> OpusPipelineMode {
        self.mode
    }

    fn is_eligible(&self, packet: &[u8]) -> bool {
        self.input_format.sample_rate() == COMPATIBLE_SAMPLE_RATE
            && self.input_format.channels() == COMPATIBLE_CHANNELS
            && self.volume == VolumeLevel::NORMAL
            && !self.filters_active
            && !packet.is_empty()
            && packet.len() <= MAX_COMPATIBLE_OPUS_FRAME_BYTES
            && packet_samples(packet, COMPATIBLE_SAMPLE_RATE)
                == Ok(super::COMPATIBLE_SAMPLES_PER_CHANNEL)
    }
}

/// Derives the total duration of one bounded Opus packet from its TOC fields.
#[must_use]
pub fn opus_packet_duration(packet: &[u8]) -> Option<Duration> {
    let toc = *packet.first()?;
    let config = toc >> 3;
    let frame_count = match toc & 0b11 {
        0 => 1_u64,
        1 | 2 => 2,
        3 => u64::from(*packet.get(1)? & 0x3f),
        _ => unreachable!(),
    };
    if frame_count == 0 {
        return None;
    }
    let frame_micros = match config {
        0..=11 => [10_000_u64, 20_000, 40_000, 60_000][usize::from(config & 0b11)],
        12..=15 => [10_000_u64, 20_000][usize::from(config & 0b1)],
        16..=31 => [2_500_u64, 5_000, 10_000, 20_000][usize::from(config & 0b11)],
        _ => unreachable!(),
    };
    let total_micros = frame_micros.checked_mul(frame_count)?;
    (total_micros <= 120_000).then(|| Duration::from_micros(total_micros))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{OpusModeTransition, OpusPassthrough, OpusPipelineMode, opus_packet_duration};
    use crate::{
        COMPATIBLE_FRAME_DURATION, EncodedFrameSlot, MAX_COMPATIBLE_OPUS_FRAME_BYTES, PcmFormat,
        VolumeLevel,
    };

    const TWENTY_MS_PACKET: [u8; 3] = [19 << 3, 0x11, 0x22];

    #[test]
    fn compatible_packets_transition_to_exact_direct_delivery() {
        let format = PcmFormat::new(48_000, 2).unwrap();
        let mut router = OpusPassthrough::new(format);
        let mut output = EncodedFrameSlot::new();
        let storage = output.data.as_ptr();
        let timestamp = Some(Duration::from_millis(80));

        let first = router
            .route_packet(&TWENTY_MS_PACKET, timestamp, &mut output)
            .unwrap();
        assert_eq!(first.mode, OpusPipelineMode::Passthrough);
        assert_eq!(
            first.transition,
            Some(OpusModeTransition::EnabledPassthrough)
        );
        assert!(first.delivered());
        assert_eq!(output.data(), TWENTY_MS_PACKET);
        assert_eq!(output.timestamp(), timestamp);
        assert_eq!(output.volume(), VolumeLevel::NORMAL);
        assert_eq!(output.data.as_ptr(), storage);

        let next = router
            .route_packet(&TWENTY_MS_PACKET, timestamp, &mut output)
            .unwrap();
        assert_eq!(next.transition, None);
    }

    #[test]
    fn volume_filters_and_geometry_force_explicit_transcode_transitions() {
        let format = PcmFormat::new(48_000, 2).unwrap();
        let mut router = OpusPassthrough::new(format);
        let mut output = EncodedFrameSlot::new();
        router
            .route_packet(&TWENTY_MS_PACKET, None, &mut output)
            .unwrap();
        let delivered = output.data().to_vec();

        router.set_volume(VolumeLevel::MUTED);
        let muted = router
            .route_packet(&TWENTY_MS_PACKET, None, &mut output)
            .unwrap();
        assert_eq!(muted.mode, OpusPipelineMode::Transcode);
        assert_eq!(
            muted.transition,
            Some(OpusModeTransition::DisabledPassthrough)
        );
        assert!(!muted.delivered());
        assert_eq!(output.data(), delivered);

        router.set_volume(VolumeLevel::NORMAL);
        router.set_filters_active(true);
        assert_eq!(
            router
                .route_packet(&TWENTY_MS_PACKET, None, &mut output)
                .unwrap()
                .mode,
            OpusPipelineMode::Transcode
        );
        router.set_filters_active(false);
        assert_eq!(
            router
                .route_packet(&TWENTY_MS_PACKET, None, &mut output)
                .unwrap()
                .transition,
            Some(OpusModeTransition::EnabledPassthrough)
        );

        router.set_input_format(PcmFormat::new(48_000, 1).unwrap());
        assert_eq!(
            router
                .route_packet(&TWENTY_MS_PACKET, None, &mut output)
                .unwrap()
                .mode,
            OpusPipelineMode::Transcode
        );
    }

    #[test]
    fn duration_and_packet_bounds_are_checked_without_mutating_output() {
        let mut router = OpusPassthrough::new(PcmFormat::new(48_000, 2).unwrap());
        let mut output = EncodedFrameSlot::new();
        output
            .write(&[9, 8, 7], None, VolumeLevel::new(175))
            .unwrap();

        for packet in [
            &[][..],
            &[16 << 3][..],
            &vec![0; MAX_COMPATIBLE_OPUS_FRAME_BYTES + 1],
        ] {
            assert_eq!(
                router.route_packet(packet, None, &mut output).unwrap().mode,
                OpusPipelineMode::Transcode
            );
            assert_eq!(output.data(), [9, 8, 7]);
            assert_eq!(output.volume(), VolumeLevel::new(175));
        }

        assert_eq!(
            opus_packet_duration(&TWENTY_MS_PACKET),
            Some(COMPATIBLE_FRAME_DURATION)
        );
        assert_eq!(opus_packet_duration(&[(19 << 3) | 3, 7]), None);
    }
}
