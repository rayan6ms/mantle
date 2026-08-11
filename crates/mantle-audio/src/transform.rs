use super::{AudioFrameError, PcmFormat, PcmFrame};

/// Lavaplayer-compatible player volume in the inclusive range `0..=1000`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VolumeLevel(u16);

impl VolumeLevel {
    pub const MUTED: Self = Self(0);
    pub const NORMAL: Self = Self(100);
    pub const MAXIMUM: Self = Self(1_000);

    #[must_use]
    pub fn new(volume: i32) -> Self {
        Self(u16::try_from(volume.clamp(0, 1_000)).unwrap_or(0))
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }

    fn multiplier(self) -> i32 {
        let volume = i32::from(self.0);
        if volume <= 150 {
            #[allow(clippy::cast_possible_truncation)]
            let multiplier = ((f32::from(self.0) * 0.0079).tan() * 10_000.0) as i32;
            multiplier
        } else {
            24_621 * volume / 150
        }
    }
}

/// Converts canonical PCM to Lavaplayer-compatible signed 16-bit samples without allocation.
///
/// # Errors
///
/// Returns an error before mutation when the frame is uninitialized or the output is too small.
pub fn convert_to_i16(frame: &PcmFrame, output: &mut [i16]) -> Result<usize, AudioFrameError> {
    frame.format().ok_or(AudioFrameError::MissingPcmFormat)?;
    if frame.samples().len() > output.len() {
        return Err(AudioFrameError::SampleBufferTooSmall {
            required: frame.samples().len(),
            capacity: output.len(),
        });
    }
    for (output_sample, input_sample) in output.iter_mut().zip(frame.samples()) {
        #[allow(clippy::cast_possible_truncation)]
        let sample = (*input_sample * 32_768.0) as i32;
        *output_sample =
            i16::try_from(sample.clamp(i32::from(i16::MIN), i32::from(i16::MAX))).unwrap_or(0);
    }
    Ok(frame.samples().len())
}

/// Applies Lavaplayer's nonlinear player-volume curve to signed 16-bit PCM in place.
///
/// Volume 100 is unity. Volume zero intentionally leaves samples unchanged because Lavaplayer
/// represents muted frames as normal-volume encoded bytes carrying volume metadata zero.
pub fn apply_volume(samples: &mut [i16], volume: VolumeLevel) {
    if matches!(volume, VolumeLevel::MUTED | VolumeLevel::NORMAL) {
        return;
    }
    let multiplier = volume.multiplier();
    for sample in samples {
        let scaled = i32::from(*sample) * multiplier / 10_000;
        *sample =
            i16::try_from(scaled.clamp(i32::from(i16::MIN), i32::from(i16::MAX))).unwrap_or(0);
    }
}

/// Maps canonical PCM between mono and stereo in place without growing its allocation.
///
/// Mono is duplicated to stereo. Stereo-to-mono retains the first channel, matching
/// Lavaplayer's `ChannelCountPcmAudioFilter` behavior.
///
/// # Errors
///
/// Returns an error before mutation when the frame is uninitialized or stereo expansion exceeds
/// its preallocated capacity.
pub fn map_channels(frame: &mut PcmFrame, output_channels: u16) -> Result<(), AudioFrameError> {
    let input_format = frame.format().ok_or(AudioFrameError::MissingPcmFormat)?;
    let output_format = PcmFormat::new(input_format.sample_rate(), output_channels)?;
    if input_format == output_format {
        return Ok(());
    }

    let timestamp = frame.timestamp();
    match (input_format.channels(), output_channels) {
        (1, 2) => {
            let input_samples = frame.samples().len();
            let output_samples =
                input_samples
                    .checked_mul(2)
                    .ok_or(AudioFrameError::PcmCapacityExceeded {
                        required: usize::MAX,
                        capacity: frame.capacity(),
                    })?;
            if output_samples > frame.capacity() {
                return Err(AudioFrameError::PcmCapacityExceeded {
                    required: output_samples,
                    capacity: frame.capacity(),
                });
            }
            frame.prepare(output_samples, output_format, timestamp)?;
            for index in (0..input_samples).rev() {
                let sample = frame.samples[index];
                frame.samples[index * 2] = sample;
                frame.samples[index * 2 + 1] = sample;
            }
        }
        (2, 1) => {
            let output_samples = frame.samples().len() / 2;
            for index in 0..output_samples {
                frame.samples[index] = frame.samples[index * 2];
            }
            frame.prepare(output_samples, output_format, timestamp)?;
        }
        _ => unreachable!("PcmFormat limits channels to mono or stereo"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{VolumeLevel, apply_volume, convert_to_i16, map_channels};
    use crate::{AudioFrameError, PcmFormat, PcmFrame};

    #[test]
    fn channel_mapping_is_reference_compatible_and_allocation_stable() {
        let mono = PcmFormat::new(44_100, 1).unwrap();
        let timestamp = Some(Duration::from_millis(17));
        let mut frame = PcmFrame::with_capacity(8);
        frame
            .copy_from_interleaved(&[0.1, -0.2, 0.3, -0.4], mono, timestamp)
            .unwrap();
        let storage = frame.samples.as_ptr();

        map_channels(&mut frame, 2).unwrap();
        assert_eq!(
            frame.samples(),
            [0.1, 0.1, -0.2, -0.2, 0.3, 0.3, -0.4, -0.4]
        );
        assert_eq!(frame.format(), Some(PcmFormat::new(44_100, 2).unwrap()));
        assert_eq!(frame.timestamp(), timestamp);
        assert_eq!(frame.samples.as_ptr(), storage);

        map_channels(&mut frame, 1).unwrap();
        assert_eq!(frame.samples(), [0.1, -0.2, 0.3, -0.4]);
        assert_eq!(frame.samples.as_ptr(), storage);
    }

    #[test]
    fn channel_mapping_rejects_capacity_before_mutation() {
        let mono = PcmFormat::new(48_000, 1).unwrap();
        let mut frame = PcmFrame::with_capacity(3);
        frame
            .copy_from_interleaved(&[0.1, 0.2, 0.3], mono, None)
            .unwrap();
        assert!(matches!(
            map_channels(&mut frame, 2),
            Err(AudioFrameError::PcmCapacityExceeded {
                required: 6,
                capacity: 3
            })
        ));
        assert_eq!(frame.samples(), [0.1, 0.2, 0.3]);
        assert_eq!(frame.format(), Some(mono));
    }

    #[test]
    fn sample_conversion_and_volume_match_reference_integer_semantics() {
        assert_eq!(VolumeLevel::new(-1), VolumeLevel::MUTED);
        assert_eq!(VolumeLevel::new(2_000), VolumeLevel::MAXIMUM);

        let format = PcmFormat::new(48_000, 2).unwrap();
        let mut frame = PcmFrame::with_capacity(4);
        frame
            .copy_from_interleaved(&[-2.0, -1.0, 0.25, 2.0], format, None)
            .unwrap();
        let mut samples = [0_i16; 4];
        assert_eq!(convert_to_i16(&frame, &mut samples).unwrap(), 4);
        assert_eq!(samples, [i16::MIN, i16::MIN, 8_192, i16::MAX]);
        apply_volume(&mut samples, VolumeLevel::MUTED);
        apply_volume(&mut samples, VolumeLevel::NORMAL);
        assert_eq!(samples, [i16::MIN, i16::MIN, 8_192, i16::MAX]);

        apply_volume(&mut samples, VolumeLevel::new(200));
        assert_eq!(samples, [i16::MIN, i16::MIN, 26_892, i16::MAX]);
    }

    #[test]
    fn sample_conversion_rejects_small_output_before_mutation() {
        let format = PcmFormat::new(48_000, 1).unwrap();
        let mut frame = PcmFrame::with_capacity(2);
        frame
            .copy_from_interleaved(&[0.25, -0.25], format, None)
            .unwrap();
        let mut output = [123_i16];
        assert_eq!(
            convert_to_i16(&frame, &mut output),
            Err(AudioFrameError::SampleBufferTooSmall {
                required: 2,
                capacity: 1
            })
        );
        assert_eq!(output, [123]);
    }
}
