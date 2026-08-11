use crate::{Track, TrackInfo};
use std::fmt;
use std::time::Duration;

const TRACK_INFO_VERSIONED: u32 = 1;
const TRACK_INFO_VERSION: u8 = 3;
const MESSAGE_SIZE_MASK: u32 = 0x3fff_ffff;
const SYNTHETIC_SOURCE: &str = "mantle-oracle";
const SYNTHETIC_PAYLOAD: &str = "oracle-v1";

/// Bounds applied before serialized track data is allocated or decoded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SerializationLimits {
    pub message_bytes: usize,
    pub string_bytes: usize,
}

impl Default for SerializationLimits {
    fn default() -> Self {
        Self {
            message_bytes: 1 << 20,
            string_bytes: usize::from(u16::MAX),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedTrack {
    pub info: TrackInfo,
    pub position: Duration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SerializationError {
    MessageTooLarge,
    StringTooLarge,
    Truncated,
    InvalidModifiedUtf8,
    InvalidDuration,
    InvalidPosition,
    UnexpectedTerminator,
    UnknownSource,
    UnknownSourcePayload,
}

impl fmt::Display for SerializationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MessageTooLarge => "serialized track message exceeds its byte limit",
            Self::StringTooLarge => "serialized track string exceeds its modified UTF-8 limit",
            Self::Truncated => "serialized track data is truncated",
            Self::InvalidModifiedUtf8 => "serialized track contains invalid modified UTF-8",
            Self::InvalidDuration => "serialized track has a negative or unsupported duration",
            Self::InvalidPosition => "serialized track has a negative or unsupported position",
            Self::UnexpectedTerminator => {
                "expected a serialized track but found the stream terminator"
            }
            Self::UnknownSource => "serialized track uses an unknown source manager",
            Self::UnknownSourcePayload => {
                "serialized synthetic track has an unknown source payload"
            }
        })
    }
}

impl std::error::Error for SerializationError {}

/// Encodes the source-specific detail bytes used by the Phase 5 synthetic source.
///
/// # Errors
///
/// Returns a configured size-limit error if the detail bytes cannot be represented.
pub fn encode_synthetic_track_details(
    limits: SerializationLimits,
) -> Result<Vec<u8>, SerializationError> {
    let mut output = Writer::new(limits);
    output.write_modified_utf8(SYNTHETIC_SOURCE)?;
    output.write_modified_utf8(SYNTHETIC_PAYLOAD)?;
    Ok(output.finish())
}

/// Validates source-specific detail bytes produced by Lavaplayer or Mantle.
///
/// Like Lavaplayer's `decodeTrackDetails`, trailing source bytes are ignored.
///
/// # Errors
///
/// Returns an error for malformed, oversized, or unsupported details.
pub fn decode_synthetic_track_details(
    bytes: &[u8],
    limits: SerializationLimits,
) -> Result<(), SerializationError> {
    if bytes.len() > limits.message_bytes {
        return Err(SerializationError::MessageTooLarge);
    }
    let mut input = Reader::new(bytes, limits);
    if input.read_modified_utf8()? != SYNTHETIC_SOURCE {
        return Err(SerializationError::UnknownSource);
    }
    if input.read_modified_utf8()? != SYNTHETIC_PAYLOAD {
        return Err(SerializationError::UnknownSourcePayload);
    }
    Ok(())
}

/// Encodes one complete Lavaplayer `MessageOutput` track record using version 3 metadata.
///
/// # Errors
///
/// Returns an error if a field or the complete message exceeds its configured bound.
pub fn encode_synthetic_track(
    track: &Track,
    limits: SerializationLimits,
) -> Result<Vec<u8>, SerializationError> {
    let mut payload = Writer::new(limits);
    payload.write_u8(TRACK_INFO_VERSION)?;
    payload.write_modified_utf8(&track.info.title)?;
    payload.write_modified_utf8(&track.info.author)?;
    payload.write_i64(duration_millis(track.info.duration)?)?;
    payload.write_modified_utf8(&track.info.identifier)?;
    payload.write_u8(u8::from(track.info.is_stream))?;
    payload.write_optional_modified_utf8(track.info.uri.as_deref())?;
    payload.write_optional_modified_utf8(track.info.artwork_url.as_deref())?;
    payload.write_optional_modified_utf8(track.info.isrc.as_deref())?;
    payload.write_bytes(&encode_synthetic_track_details(limits)?)?;
    payload.write_i64(position_millis(track.position)?)?;
    let payload = payload.finish();
    if payload.len() > MESSAGE_SIZE_MASK as usize {
        return Err(SerializationError::MessageTooLarge);
    }

    let mut encoded = Vec::with_capacity(payload.len().saturating_add(4));
    let header = u32::try_from(payload.len()).map_err(|_| SerializationError::MessageTooLarge)?
        | TRACK_INFO_VERSIONED << 30;
    encoded.extend_from_slice(&header.to_be_bytes());
    encoded.extend_from_slice(&payload);
    Ok(encoded)
}

/// Decodes one complete Lavaplayer `MessageInput` track record.
///
/// Versions 1–3 are accepted. A versioned record newer than version 3 is parsed
/// with the version 3 field set, matching Lavaplayer 2.2.6's forward behavior.
///
/// # Errors
///
/// Returns an error for malformed, oversized, unsupported, or terminator input.
pub fn decode_synthetic_track(
    bytes: &[u8],
    limits: SerializationLimits,
) -> Result<DecodedTrack, SerializationError> {
    if bytes.len() > limits.message_bytes.saturating_add(4) {
        return Err(SerializationError::MessageTooLarge);
    }
    let mut envelope = Reader::new(bytes, limits);
    let header = envelope.read_u32()?;
    let message_size = (header & MESSAGE_SIZE_MASK) as usize;
    if message_size == 0 {
        return Err(SerializationError::UnexpectedTerminator);
    }
    if message_size > limits.message_bytes {
        return Err(SerializationError::MessageTooLarge);
    }
    let flags = header >> 30;
    let message = envelope.read_exact(message_size)?;
    let mut input = Reader::new(message, limits);
    let version = if flags & TRACK_INFO_VERSIONED != 0 {
        input.read_u8()?
    } else {
        1
    };
    let title = input.read_modified_utf8()?;
    let author = input.read_modified_utf8()?;
    let duration = nonnegative_duration(input.read_i64()?, SerializationError::InvalidDuration)?;
    let identifier = input.read_modified_utf8()?;
    let is_stream = input.read_u8()? != 0;
    let uri = if version >= 2 {
        input.read_optional_modified_utf8()?
    } else {
        None
    };
    let (artwork_url, isrc) = if version >= 3 {
        (
            input.read_optional_modified_utf8()?,
            input.read_optional_modified_utf8()?,
        )
    } else {
        (None, None)
    };
    if input.read_modified_utf8()? != SYNTHETIC_SOURCE {
        return Err(SerializationError::UnknownSource);
    }
    if input.read_modified_utf8()? != SYNTHETIC_PAYLOAD {
        return Err(SerializationError::UnknownSourcePayload);
    }
    let position = nonnegative_duration(input.read_i64()?, SerializationError::InvalidPosition)?;

    Ok(DecodedTrack {
        info: TrackInfo {
            title,
            author,
            duration,
            identifier,
            is_stream,
            uri,
            artwork_url,
            isrc,
        },
        position,
    })
}

fn duration_millis(value: Duration) -> Result<i64, SerializationError> {
    i64::try_from(value.as_millis()).map_err(|_| SerializationError::InvalidDuration)
}

fn position_millis(value: Duration) -> Result<i64, SerializationError> {
    i64::try_from(value.as_millis()).map_err(|_| SerializationError::InvalidPosition)
}

fn nonnegative_duration(
    value: i64,
    error: SerializationError,
) -> Result<Duration, SerializationError> {
    u64::try_from(value)
        .map(Duration::from_millis)
        .map_err(|_| error)
}

struct Writer {
    bytes: Vec<u8>,
    limits: SerializationLimits,
}

impl Writer {
    fn new(limits: SerializationLimits) -> Self {
        Self {
            bytes: Vec::new(),
            limits,
        }
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }

    fn write_u8(&mut self, value: u8) -> Result<(), SerializationError> {
        self.write_bytes(&[value])
    }

    fn write_i64(&mut self, value: i64) -> Result<(), SerializationError> {
        self.write_bytes(&value.to_be_bytes())
    }

    fn write_optional_modified_utf8(
        &mut self,
        value: Option<&str>,
    ) -> Result<(), SerializationError> {
        self.write_u8(u8::from(value.is_some()))?;
        if let Some(value) = value {
            self.write_modified_utf8(value)?;
        }
        Ok(())
    }

    fn write_modified_utf8(&mut self, value: &str) -> Result<(), SerializationError> {
        let encoded_length = modified_utf8_length(value)?;
        if encoded_length > self.limits.string_bytes || encoded_length > usize::from(u16::MAX) {
            return Err(SerializationError::StringTooLarge);
        }
        let encoded_length =
            u16::try_from(encoded_length).map_err(|_| SerializationError::StringTooLarge)?;
        self.reserve(usize::from(encoded_length).saturating_add(2))?;
        self.bytes.extend_from_slice(&encoded_length.to_be_bytes());
        for unit in value.encode_utf16() {
            match unit {
                0x0001..=0x007f => self.bytes.push(low_byte(unit)),
                0x0000..=0x07ff => {
                    self.bytes.push(low_byte(0xc0 | (unit >> 6)));
                    self.bytes.push(low_byte(0x80 | (unit & 0x3f)));
                }
                _ => {
                    self.bytes.push(low_byte(0xe0 | (unit >> 12)));
                    self.bytes.push(low_byte(0x80 | ((unit >> 6) & 0x3f)));
                    self.bytes.push(low_byte(0x80 | (unit & 0x3f)));
                }
            }
        }
        Ok(())
    }

    fn write_bytes(&mut self, value: &[u8]) -> Result<(), SerializationError> {
        self.reserve(value.len())?;
        self.bytes.extend_from_slice(value);
        Ok(())
    }

    fn reserve(&self, additional: usize) -> Result<(), SerializationError> {
        if self.bytes.len().saturating_add(additional) > self.limits.message_bytes {
            Err(SerializationError::MessageTooLarge)
        } else {
            Ok(())
        }
    }
}

fn low_byte(value: u16) -> u8 {
    value.to_be_bytes()[1]
}

fn modified_utf8_length(value: &str) -> Result<usize, SerializationError> {
    value.encode_utf16().try_fold(0_usize, |length, unit| {
        length
            .checked_add(match unit {
                0x0001..=0x007f => 1,
                0x0000..=0x07ff => 2,
                _ => 3,
            })
            .ok_or(SerializationError::StringTooLarge)
    })
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
    limits: SerializationLimits,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], limits: SerializationLimits) -> Self {
        Self {
            bytes,
            offset: 0,
            limits,
        }
    }

    fn read_u8(&mut self) -> Result<u8, SerializationError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, SerializationError> {
        let bytes = self.read_exact(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, SerializationError> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_i64(&mut self) -> Result<i64, SerializationError> {
        let bytes = self.read_exact(8)?;
        Ok(i64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_optional_modified_utf8(&mut self) -> Result<Option<String>, SerializationError> {
        if self.read_u8()? == 0 {
            Ok(None)
        } else {
            self.read_modified_utf8().map(Some)
        }
    }

    fn read_modified_utf8(&mut self) -> Result<String, SerializationError> {
        let length = usize::from(self.read_u16()?);
        if length > self.limits.string_bytes {
            return Err(SerializationError::StringTooLarge);
        }
        decode_modified_utf8(self.read_exact(length)?)
    }

    fn read_exact(&mut self, length: usize) -> Result<&'a [u8], SerializationError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(SerializationError::Truncated)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(SerializationError::Truncated)?;
        self.offset = end;
        Ok(value)
    }
}

fn decode_modified_utf8(bytes: &[u8]) -> Result<String, SerializationError> {
    let mut units = Vec::with_capacity(bytes.len());
    let mut offset = 0;
    while offset < bytes.len() {
        let first = bytes[offset];
        match first >> 4 {
            0..=7 => {
                units.push(u16::from(first));
                offset += 1;
            }
            12 | 13 => {
                let second = *bytes
                    .get(offset + 1)
                    .ok_or(SerializationError::InvalidModifiedUtf8)?;
                if second & 0xc0 != 0x80 {
                    return Err(SerializationError::InvalidModifiedUtf8);
                }
                units.push((u16::from(first & 0x1f) << 6) | u16::from(second & 0x3f));
                offset += 2;
            }
            14 => {
                let second = *bytes
                    .get(offset + 1)
                    .ok_or(SerializationError::InvalidModifiedUtf8)?;
                let third = *bytes
                    .get(offset + 2)
                    .ok_or(SerializationError::InvalidModifiedUtf8)?;
                if second & 0xc0 != 0x80 || third & 0xc0 != 0x80 {
                    return Err(SerializationError::InvalidModifiedUtf8);
                }
                units.push(
                    (u16::from(first & 0x0f) << 12)
                        | (u16::from(second & 0x3f) << 6)
                        | u16::from(third & 0x3f),
                );
                offset += 3;
            }
            _ => return Err(SerializationError::InvalidModifiedUtf8),
        }
    }
    String::from_utf16(&units).map_err(|_| SerializationError::InvalidModifiedUtf8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Frame, TrackState};
    use std::collections::VecDeque;

    const REFERENCE_DETAILS: &[u8] = b"\0\rmantle-oracle\0\toracle-v1";
    const REFERENCE_FULL_TRACK_HEX: &str = concat!(
        "4000009d03",
        "001953796e74686574696320c080207469746c6520eda0beedb680",
        "001053796e74686574696320617574686f72",
        "00000000000003e8",
        "000a676174653a747261636b00",
        "0100136f7261636c653a2f2f676174653a747261636b",
        "0100106f7261636c653a2f2f617274776f726b",
        "01000c4f5241434c45303030303031",
        "000d6d616e746c652d6f7261636c6500096f7261636c652d7631",
        "000000000000000a"
    );

    fn track() -> Track {
        Track {
            info: TrackInfo {
                title: "Synthetic \0 title 🦀".into(),
                author: "Synthetic author".into(),
                duration: Duration::from_secs(1),
                identifier: "gate:track".into(),
                is_stream: false,
                uri: Some("oracle://gate:track".into()),
                artwork_url: Some("oracle://artwork".into()),
                isrc: Some("ORACLE000001".into()),
            },
            state: TrackState::Inactive,
            position: Duration::from_millis(10),
            user_data: None,
            markers: Vec::new(),
            frames: VecDeque::<Frame>::new(),
        }
    }

    #[test]
    fn synthetic_details_match_the_lavaplayer_2_2_6_golden() {
        let limits = SerializationLimits::default();
        assert_eq!(
            encode_synthetic_track_details(limits).unwrap(),
            REFERENCE_DETAILS
        );
        assert_eq!(
            decode_synthetic_track_details(REFERENCE_DETAILS, limits),
            Ok(())
        );
    }

    #[test]
    fn full_track_round_trip_preserves_metadata_position_and_modified_utf8() {
        let limits = SerializationLimits::default();
        let expected = track();
        let encoded = encode_synthetic_track(&expected, limits).unwrap();
        assert_eq!(hex(&encoded), REFERENCE_FULL_TRACK_HEX);
        let decoded = decode_synthetic_track(&encoded, limits).unwrap();
        assert_eq!(decoded.info, expected.info);
        assert_eq!(decoded.position, expected.position);
    }

    #[test]
    fn version_one_records_default_newer_nullable_metadata() {
        let limits = SerializationLimits::default();
        let expected = track();
        let mut payload = Writer::new(limits);
        payload.write_modified_utf8(&expected.info.title).unwrap();
        payload.write_modified_utf8(&expected.info.author).unwrap();
        payload.write_i64(1_000).unwrap();
        payload
            .write_modified_utf8(&expected.info.identifier)
            .unwrap();
        payload.write_u8(0).unwrap();
        payload.write_modified_utf8(SYNTHETIC_SOURCE).unwrap();
        payload.write_modified_utf8(SYNTHETIC_PAYLOAD).unwrap();
        payload.write_i64(10).unwrap();
        let payload = payload.finish();
        let mut encoded = Vec::from(u32::try_from(payload.len()).unwrap().to_be_bytes());
        encoded.extend_from_slice(&payload);

        let decoded = decode_synthetic_track(&encoded, limits).unwrap();
        assert_eq!(decoded.info.uri, None);
        assert_eq!(decoded.info.artwork_url, None);
        assert_eq!(decoded.info.isrc, None);
    }

    #[test]
    fn configured_limits_fail_before_oversized_fields_are_accepted() {
        let small_message = SerializationLimits {
            message_bytes: 8,
            ..SerializationLimits::default()
        };
        assert_eq!(
            encode_synthetic_track_details(small_message),
            Err(SerializationError::MessageTooLarge)
        );
        let small_string = SerializationLimits {
            string_bytes: 4,
            ..SerializationLimits::default()
        };
        assert_eq!(
            decode_synthetic_track_details(REFERENCE_DETAILS, small_string),
            Err(SerializationError::StringTooLarge)
        );
    }

    #[test]
    fn every_truncation_and_a_deterministic_malformed_corpus_are_panic_free() {
        let limits = SerializationLimits::default();
        let encoded = encode_synthetic_track(&track(), limits).unwrap();
        for length in 0..encoded.len() {
            assert!(decode_synthetic_track(&encoded[..length], limits).is_err());
        }

        let mut state = 0x4d59_5df4_d0f3_3173_u64;
        for length in 0..512 {
            let mut bytes = vec![0_u8; length];
            for byte in &mut bytes {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                *byte = state.to_le_bytes()[0];
            }
            let _ = decode_synthetic_track(
                &bytes,
                SerializationLimits {
                    message_bytes: 512,
                    string_bytes: 256,
                },
            );
        }
    }

    #[test]
    fn malformed_details_have_stable_error_categories() {
        let limits = SerializationLimits::default();
        assert_eq!(
            decode_synthetic_track_details(&REFERENCE_DETAILS[..1], limits),
            Err(SerializationError::Truncated)
        );
        assert_eq!(
            decode_synthetic_track_details(b"\0\x01\xff", limits),
            Err(SerializationError::InvalidModifiedUtf8)
        );
        assert_eq!(
            decode_synthetic_track_details(b"\0\x05other\0\x00", limits),
            Err(SerializationError::UnknownSource)
        );
    }

    fn hex(bytes: &[u8]) -> String {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut output = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            output.push(char::from(DIGITS[usize::from(byte >> 4)]));
            output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
        }
        output
    }
}
