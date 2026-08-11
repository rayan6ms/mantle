//! Safe, allocation-stable ownership of Mantle's narrow libopus FFI boundary.
//!
//! Native encoder state is allocated only at construction. Encoding writes into caller-owned
//! slices, validates every length before exposing a pointer, and never returns backend pointers.

use std::fmt;
use std::ptr::NonNull;

use opus_head_sys as ffi;

const OPUS_OK: i32 = 0;
const OPUS_APPLICATION_AUDIO: i32 = ffi::OPUS_APPLICATION_AUDIO.cast_signed();
const OPUS_SET_COMPLEXITY_REQUEST: i32 = ffi::OPUS_SET_COMPLEXITY_REQUEST.cast_signed();
const OPUS_RESET_STATE: i32 = ffi::OPUS_RESET_STATE.cast_signed();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpusError {
    InvalidConfiguration(&'static str),
    InvalidInputLength {
        expected: usize,
        actual: usize,
    },
    NativeStatus {
        operation: &'static str,
        status: i32,
    },
    NativeContract(&'static str),
}

impl fmt::Display for OpusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration(message) => {
                write!(formatter, "invalid Opus configuration: {message}")
            }
            Self::InvalidInputLength { expected, actual } => write!(
                formatter,
                "Opus input contains {actual} interleaved samples; expected {expected}"
            ),
            Self::NativeStatus { operation, status } => {
                write!(formatter, "libopus {operation} failed with status {status}")
            }
            Self::NativeContract(message) => {
                write!(formatter, "libopus contract failed: {message}")
            }
        }
    }
}

impl std::error::Error for OpusError {}

/// Validates an Opus packet and returns its total samples at `sample_rate`.
///
/// # Errors
///
/// Returns an error for an empty or oversized packet, an invalid sample rate, or a packet rejected
/// by libopus's structural parser.
pub fn packet_samples(packet: &[u8], sample_rate: u32) -> Result<usize, OpusError> {
    if packet.is_empty() {
        return Err(OpusError::InvalidConfiguration("packet must not be empty"));
    }
    if sample_rate == 0 || !sample_rate.is_multiple_of(400) {
        return Err(OpusError::InvalidConfiguration(
            "packet sample rate must be a non-zero multiple of 400 Hz",
        ));
    }
    let packet_len = i32::try_from(packet.len()).map_err(|_| {
        OpusError::InvalidConfiguration("packet length does not fit the native API")
    })?;
    let sample_rate = i32::try_from(sample_rate)
        .map_err(|_| OpusError::InvalidConfiguration("sample rate does not fit the native API"))?;
    // SAFETY: `packet` contains `packet_len` readable initialized bytes for the duration of this
    // call, and libopus retains no pointer. The sample rate satisfies the parser's documented
    // multiple-of-400 requirement.
    let samples =
        unsafe { ffi::opus_packet_get_nb_samples(packet.as_ptr(), packet_len, sample_rate) };
    if samples < 0 {
        return Err(OpusError::NativeStatus {
            operation: "packet validation",
            status: samples,
        });
    }
    usize::try_from(samples)
        .map_err(|_| OpusError::NativeContract("packet sample count is negative"))
}

/// One independently owned libopus encoder state.
pub struct OpusEncoder {
    state: NonNull<ffi::OpusEncoder>,
    channels: usize,
}

impl OpusEncoder {
    /// Creates an audio-mode encoder and applies Lavaplayer's `0..=10` complexity setting.
    ///
    /// # Errors
    ///
    /// Returns an error for unsupported rates, channels, complexity, or native initialization.
    pub fn new(sample_rate: u32, channels: u16, complexity: u8) -> Result<Self, OpusError> {
        if !matches!(sample_rate, 8_000 | 12_000 | 16_000 | 24_000 | 48_000) {
            return Err(OpusError::InvalidConfiguration(
                "sample rate must be 8000, 12000, 16000, 24000, or 48000 Hz",
            ));
        }
        if !(1..=2).contains(&channels) {
            return Err(OpusError::InvalidConfiguration(
                "channel count must be one or two",
            ));
        }
        if complexity > 10 {
            return Err(OpusError::InvalidConfiguration(
                "complexity must be between zero and ten",
            ));
        }

        let sample_rate = i32::try_from(sample_rate).map_err(|_| {
            OpusError::InvalidConfiguration("sample rate does not fit the native API")
        })?;
        let mut status = OPUS_OK;
        // SAFETY: all scalar parameters are validated against libopus's documented domain, and
        // `status` is a live writable `i32` for the duration of the call.
        let state = unsafe {
            ffi::opus_encoder_create(
                sample_rate,
                i32::from(channels),
                OPUS_APPLICATION_AUDIO,
                &raw mut status,
            )
        };
        let state = NonNull::new(state).ok_or(OpusError::NativeStatus {
            operation: "encoder creation",
            status,
        })?;
        if status != OPUS_OK {
            // SAFETY: a non-null pointer returned by `opus_encoder_create` is owned by this call
            // and has not been published. Destroying it here prevents a native leak on a
            // contradictory status result.
            unsafe { ffi::opus_encoder_destroy(state.as_ptr()) };
            return Err(OpusError::NativeStatus {
                operation: "encoder creation",
                status,
            });
        }

        let mut encoder = Self {
            state,
            channels: usize::from(channels),
        };
        encoder.set_complexity(complexity)?;
        Ok(encoder)
    }

    /// Encodes one interleaved floating-point frame into caller-owned storage.
    ///
    /// # Errors
    ///
    /// Returns an error before calling native code when lengths cannot be represented or the PCM
    /// sample count does not exactly match `frames_per_channel * channels`.
    pub fn encode(
        &mut self,
        pcm: &[i16],
        frames_per_channel: usize,
        output: &mut [u8],
    ) -> Result<usize, OpusError> {
        let expected = frames_per_channel.checked_mul(self.channels).ok_or(
            OpusError::InvalidConfiguration("PCM sample count overflowed"),
        )?;
        if pcm.len() != expected {
            return Err(OpusError::InvalidInputLength {
                expected,
                actual: pcm.len(),
            });
        }
        if output.is_empty() {
            return Err(OpusError::InvalidConfiguration(
                "output buffer must not be empty",
            ));
        }
        let frame_size = i32::try_from(frames_per_channel).map_err(|_| {
            OpusError::InvalidConfiguration("frame size does not fit the native API")
        })?;
        let output_capacity = i32::try_from(output.len()).map_err(|_| {
            OpusError::InvalidConfiguration("output capacity does not fit the native API")
        })?;

        // SAFETY: `state` is an exclusively borrowed live encoder. The validated PCM slice holds
        // exactly `frame_size * channels` initialized samples, and the writable output slice holds
        // `output_capacity` bytes. libopus retains neither pointer after the call.
        let written = unsafe {
            ffi::opus_encode(
                self.state.as_ptr(),
                pcm.as_ptr(),
                frame_size,
                output.as_mut_ptr(),
                output_capacity,
            )
        };
        if written < 0 {
            return Err(OpusError::NativeStatus {
                operation: "encoding",
                status: written,
            });
        }
        let written = usize::try_from(written)
            .map_err(|_| OpusError::NativeContract("negative output escaped status handling"))?;
        if written > output.len() {
            return Err(OpusError::NativeContract(
                "encoded length exceeds caller output capacity",
            ));
        }
        Ok(written)
    }

    /// Clears codec history while preserving the configured application and complexity.
    ///
    /// # Errors
    ///
    /// Returns an error if libopus rejects its reset control request.
    pub fn reset(&mut self) -> Result<(), OpusError> {
        // SAFETY: `state` is an exclusively borrowed live encoder and `OPUS_RESET_STATE` takes no
        // variadic argument.
        let status = unsafe { ffi::opus_encoder_ctl(self.state.as_ptr(), OPUS_RESET_STATE) };
        native_status("encoder reset", status)
    }

    fn set_complexity(&mut self, complexity: u8) -> Result<(), OpusError> {
        // SAFETY: `state` is an exclusively borrowed live encoder. The request requires one C int,
        // and `complexity` was validated to the inclusive `0..=10` domain.
        let status = unsafe {
            ffi::opus_encoder_ctl(
                self.state.as_ptr(),
                OPUS_SET_COMPLEXITY_REQUEST,
                i32::from(complexity),
            )
        };
        native_status("complexity configuration", status)
    }
}

impl Drop for OpusEncoder {
    fn drop(&mut self) {
        // SAFETY: `state` is non-null, owned solely by this value, and destroyed exactly once.
        unsafe { ffi::opus_encoder_destroy(self.state.as_ptr()) };
    }
}

// SAFETY: libopus permits separate encoder states to move across threads. This wrapper never
// shares a state and all stateful operations require `&mut self`, so it intentionally remains
// `Send` but not `Sync`.
unsafe impl Send for OpusEncoder {}

fn native_status(operation: &'static str, status: i32) -> Result<(), OpusError> {
    if status == OPUS_OK {
        Ok(())
    } else {
        Err(OpusError::NativeStatus { operation, status })
    }
}

#[cfg(test)]
mod tests {
    use super::{OpusEncoder, OpusError, packet_samples};

    #[test]
    fn encodes_into_caller_storage_and_reset_reproduces_the_packet() {
        let mut encoder = OpusEncoder::new(48_000, 2, 10).unwrap();
        let pcm = [0_i16; 960 * 2];
        let mut output = [0_u8; 1_568];
        let first_len = encoder.encode(&pcm, 960, &mut output).unwrap();
        let first = output[..first_len].to_vec();
        assert_eq!(packet_samples(&first, 48_000).unwrap(), 960);

        encoder.reset().unwrap();
        let second_len = encoder.encode(&pcm, 960, &mut output).unwrap();
        assert_eq!(first_len, second_len);
        assert_eq!(first, output[..second_len]);
    }

    #[test]
    fn rejects_invalid_configuration_and_lengths_before_output_mutation() {
        assert!(matches!(
            OpusEncoder::new(44_100, 2, 10),
            Err(OpusError::InvalidConfiguration(_))
        ));
        assert!(matches!(
            OpusEncoder::new(48_000, 3, 10),
            Err(OpusError::InvalidConfiguration(_))
        ));
        assert!(matches!(
            OpusEncoder::new(48_000, 2, 11),
            Err(OpusError::InvalidConfiguration(_))
        ));

        let mut encoder = OpusEncoder::new(48_000, 2, 10).unwrap();
        let mut output = [0xa5_u8; 64];
        assert_eq!(
            encoder.encode(&[0; 100], 960, &mut output),
            Err(OpusError::InvalidInputLength {
                expected: 1_920,
                actual: 100,
            })
        );
        assert_eq!(output, [0xa5; 64]);
        assert!(packet_samples(&[], 48_000).is_err());
        assert!(packet_samples(&[(19 << 3) | 3, 7], 48_000).is_err());
    }
}
