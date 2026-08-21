use std::fmt;

use crate::{MediaError, MediaLimits, MediaSession, MemoryInput};

const TS_PACKET_BYTES: usize = 188;
const PAT_PID: u16 = 0;
const SDT_PID: u16 = 0x0011;
const ADTS_STREAM_TYPE: u8 = 0x0f;
const PID_COUNT: usize = 1 << 13;

/// Resource limits for one finite MPEG-TS segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MpegTsLimits {
    pub max_packets: usize,
    pub max_psi_section_bytes: usize,
    pub max_pes_payload_bytes: usize,
    pub max_metadata_string_bytes: usize,
}

impl Default for MpegTsLimits {
    fn default() -> Self {
        Self {
            max_packets: 65_536,
            max_psi_section_bytes: 4 * 1024,
            max_pes_payload_bytes: 8 * 1024 * 1024,
            max_metadata_string_bytes: 4 * 1024,
        }
    }
}

impl MpegTsLimits {
    pub(crate) fn validate(self) -> Result<Self, MpegTsError> {
        if self.max_packets == 0 {
            return Err(MpegTsError::InvalidLimits("max_packets must be non-zero"));
        }
        if self.max_psi_section_bytes == 0 {
            return Err(MpegTsError::InvalidLimits(
                "max_psi_section_bytes must be non-zero",
            ));
        }
        if self.max_pes_payload_bytes == 0 {
            return Err(MpegTsError::InvalidLimits(
                "max_pes_payload_bytes must be non-zero",
            ));
        }
        if self.max_metadata_string_bytes == 0 {
            return Err(MpegTsError::InvalidLimits(
                "max_metadata_string_bytes must be non-zero",
            ));
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MpegTsMetadata {
    pub service_provider: Option<String>,
    pub service_name: Option<String>,
}

/// The bounded ADTS elementary payload extracted from one MPEG-TS segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MpegTsAdtsSegment {
    adts: Box<[u8]>,
    metadata: MpegTsMetadata,
}

impl MpegTsAdtsSegment {
    #[must_use]
    pub fn adts_bytes(&self) -> &[u8] {
        &self.adts
    }

    #[must_use]
    pub fn metadata(&self) -> &MpegTsMetadata {
        &self.metadata
    }

    pub(crate) fn into_adts_bytes(self) -> Box<[u8]> {
        self.adts
    }

    /// Opens the extracted elementary stream with Mantle's existing bounded ADTS decoder.
    ///
    /// # Errors
    ///
    /// Returns the ordinary media probe, profile, resource, or decoder initialization errors.
    pub fn into_media_session(self, limits: MediaLimits) -> Result<MediaSession, MediaError> {
        MediaSession::open(Box::new(MemoryInput::new(self.adts)), Some("aac"), limits)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MpegTsError {
    InvalidLimits(&'static str),
    TruncatedPacket { trailing_bytes: usize },
    TooManyPackets { actual: usize, limit: usize },
    InvalidPacket { packet: usize, reason: &'static str },
    Continuity { pid: u16, expected: u8, actual: u8 },
    PsiSectionTooLarge { actual: usize, limit: usize },
    InvalidPsi(&'static str),
    TruncatedPsi,
    MissingProgramMap,
    MissingAdtsStream,
    InvalidPes(&'static str),
    TruncatedPes,
    PesPayloadTooLarge { actual: usize, limit: usize },
    MissingAdtsPayload,
    MetadataTooLarge { actual: usize, limit: usize },
}

impl fmt::Display for MpegTsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits(message) => write!(formatter, "invalid MPEG-TS limits: {message}"),
            Self::TruncatedPacket { trailing_bytes } => write!(
                formatter,
                "MPEG-TS input ends with a {trailing_bytes}-byte partial packet"
            ),
            Self::TooManyPackets { actual, limit } => {
                write!(
                    formatter,
                    "MPEG-TS input has {actual} packets; limit is {limit}"
                )
            }
            Self::InvalidPacket { packet, reason } => {
                write!(formatter, "invalid MPEG-TS packet {packet}: {reason}")
            }
            Self::Continuity {
                pid,
                expected,
                actual,
            } => write!(
                formatter,
                "MPEG-TS PID {pid:#x} continuity expected {expected}, received {actual}"
            ),
            Self::PsiSectionTooLarge { actual, limit } => write!(
                formatter,
                "MPEG-TS PSI section has {actual} bytes; limit is {limit}"
            ),
            Self::InvalidPsi(message) => write!(formatter, "invalid MPEG-TS PSI: {message}"),
            Self::TruncatedPsi => formatter.write_str("MPEG-TS PSI section is truncated"),
            Self::MissingProgramMap => {
                formatter.write_str("MPEG-TS PAT did not select a program map")
            }
            Self::MissingAdtsStream => {
                formatter.write_str("MPEG-TS PMT did not declare an ADTS stream")
            }
            Self::InvalidPes(message) => write!(formatter, "invalid MPEG-TS PES: {message}"),
            Self::TruncatedPes => formatter.write_str("MPEG-TS PES packet is truncated"),
            Self::PesPayloadTooLarge { actual, limit } => write!(
                formatter,
                "MPEG-TS PES payload has {actual} bytes; limit is {limit}"
            ),
            Self::MissingAdtsPayload => {
                formatter.write_str("MPEG-TS ADTS stream has no PES payload")
            }
            Self::MetadataTooLarge { actual, limit } => write!(
                formatter,
                "MPEG-TS metadata string has {actual} bytes; limit is {limit}"
            ),
        }
    }
}

impl std::error::Error for MpegTsError {}

/// Extracts one bounded ADTS elementary stream from a finite MPEG-TS segment.
///
/// PAT and PMT select stream type `0x0f`; PES headers are removed and optional SDT service
/// metadata is retained. The input must contain complete 188-byte transport packets.
///
/// # Errors
///
/// Returns an explicit structural, continuity, missing-stream, truncation, or resource-limit
/// error. It never guesses an elementary PID when PAT/PMT are absent.
pub fn extract_mpeg_ts_adts(
    bytes: &[u8],
    limits: MpegTsLimits,
) -> Result<MpegTsAdtsSegment, MpegTsError> {
    let limits = limits.validate()?;
    let trailing_bytes = bytes.len() % TS_PACKET_BYTES;
    if trailing_bytes != 0 {
        return Err(MpegTsError::TruncatedPacket { trailing_bytes });
    }
    let packet_count = bytes.len() / TS_PACKET_BYTES;
    if packet_count > limits.max_packets {
        return Err(MpegTsError::TooManyPackets {
            actual: packet_count,
            limit: limits.max_packets,
        });
    }

    let mut pat = PsiAssembler::default();
    let mut pmt = PsiAssembler::default();
    let mut sdt = PsiAssembler::default();
    let mut pmt_pid = None;
    let mut audio_pid = None;
    let mut continuity = vec![None; PID_COUNT];
    let mut metadata = MpegTsMetadata::default();
    let mut pes = PesExtractor::new(limits.max_pes_payload_bytes);

    for (packet_index, raw) in bytes.chunks_exact(TS_PACKET_BYTES).enumerate() {
        let packet = TransportPacket::parse(raw, packet_index)?;
        let relevant = packet.pid == PAT_PID
            || packet.pid == SDT_PID
            || Some(packet.pid) == pmt_pid
            || Some(packet.pid) == audio_pid;
        if relevant && !packet.payload.is_empty() {
            validate_continuity(&packet, &mut continuity)?;
        }

        if packet.pid == PAT_PID {
            for section in pat.push(
                packet.payload,
                packet.unit_start,
                limits.max_psi_section_bytes,
            )? {
                if let Some(selected) = parse_pat(&section)?
                    && pmt_pid != Some(selected)
                {
                    pmt_pid = Some(selected);
                    audio_pid = None;
                    pmt = PsiAssembler::default();
                    pes = PesExtractor::new(limits.max_pes_payload_bytes);
                }
            }
        } else if packet.pid == SDT_PID {
            for section in sdt.push(
                packet.payload,
                packet.unit_start,
                limits.max_psi_section_bytes,
            )? {
                parse_sdt(&section, limits.max_metadata_string_bytes, &mut metadata)?;
            }
        } else if Some(packet.pid) == pmt_pid {
            for section in pmt.push(
                packet.payload,
                packet.unit_start,
                limits.max_psi_section_bytes,
            )? {
                let selected = parse_pmt(&section)?;
                if audio_pid != selected {
                    audio_pid = selected;
                    pes = PesExtractor::new(limits.max_pes_payload_bytes);
                }
            }
        } else if Some(packet.pid) == audio_pid {
            pes.push(packet.payload, packet.unit_start)?;
        }
    }

    if pat.is_partial() || pmt.is_partial() || sdt.is_partial() {
        return Err(MpegTsError::TruncatedPsi);
    }
    if pmt_pid.is_none() {
        return Err(MpegTsError::MissingProgramMap);
    }
    if audio_pid.is_none() {
        return Err(MpegTsError::MissingAdtsStream);
    }
    let adts = pes.finish()?;
    if adts.is_empty() {
        return Err(MpegTsError::MissingAdtsPayload);
    }
    Ok(MpegTsAdtsSegment {
        adts: adts.into_boxed_slice(),
        metadata,
    })
}

struct TransportPacket<'a> {
    pid: u16,
    unit_start: bool,
    continuity: u8,
    discontinuity: bool,
    payload: &'a [u8],
}

impl<'a> TransportPacket<'a> {
    fn parse(raw: &'a [u8], packet: usize) -> Result<Self, MpegTsError> {
        if raw[0] != 0x47 {
            return Err(invalid_packet(packet, "sync byte is not 0x47"));
        }
        if raw[1] & 0x80 != 0 {
            return Err(invalid_packet(packet, "transport error indicator is set"));
        }
        let pid = (u16::from(raw[1] & 0x1f) << 8) | u16::from(raw[2]);
        let scrambling = raw[3] >> 6;
        if scrambling != 0 {
            return Err(invalid_packet(packet, "scrambled payload is unsupported"));
        }
        let adaptation = (raw[3] >> 4) & 3;
        if adaptation == 0 {
            return Err(invalid_packet(packet, "adaptation control is zero"));
        }
        let continuity = raw[3] & 0x0f;
        let mut discontinuity = false;
        let payload_start = match adaptation {
            1 => 4,
            2 | 3 => {
                let length = usize::from(raw[4]);
                let end = 5_usize
                    .checked_add(length)
                    .ok_or_else(|| invalid_packet(packet, "adaptation field overflows"))?;
                if end > TS_PACKET_BYTES {
                    return Err(invalid_packet(packet, "adaptation field exceeds packet"));
                }
                if length > 0 {
                    discontinuity = raw[5] & 0x80 != 0;
                }
                end
            }
            _ => unreachable!(),
        };
        let payload = if adaptation == 2 {
            &raw[TS_PACKET_BYTES..]
        } else {
            &raw[payload_start..]
        };
        Ok(Self {
            pid,
            unit_start: raw[1] & 0x40 != 0,
            continuity,
            discontinuity,
            payload,
        })
    }
}

fn invalid_packet(packet: usize, reason: &'static str) -> MpegTsError {
    MpegTsError::InvalidPacket { packet, reason }
}

fn validate_continuity(
    packet: &TransportPacket<'_>,
    continuity: &mut [Option<u8>],
) -> Result<(), MpegTsError> {
    let slot = &mut continuity[usize::from(packet.pid)];
    if let Some(previous) = *slot
        && !packet.discontinuity
    {
        let expected = (previous + 1) & 0x0f;
        if packet.continuity != expected {
            return Err(MpegTsError::Continuity {
                pid: packet.pid,
                expected,
                actual: packet.continuity,
            });
        }
    }
    *slot = Some(packet.continuity);
    Ok(())
}

#[derive(Default)]
struct PsiAssembler {
    buffer: Vec<u8>,
    expected: Option<usize>,
}

impl PsiAssembler {
    fn push(
        &mut self,
        payload: &[u8],
        unit_start: bool,
        limit: usize,
    ) -> Result<Vec<Vec<u8>>, MpegTsError> {
        if payload.is_empty() {
            return Ok(Vec::new());
        }
        if !unit_start {
            if self.buffer.is_empty() {
                return Ok(Vec::new());
            }
            return self.consume(payload, limit);
        }

        let pointer = usize::from(payload[0]);
        if pointer > payload.len() - 1 {
            return Err(MpegTsError::InvalidPsi("pointer exceeds packet payload"));
        }
        let (prefix, sections) = payload[1..].split_at(pointer);
        let mut completed = Vec::new();
        if !self.buffer.is_empty() {
            completed.extend(self.consume(prefix, limit)?);
            if !self.buffer.is_empty() {
                return Err(MpegTsError::TruncatedPsi);
            }
        }
        completed.extend(self.consume(sections, limit)?);
        Ok(completed)
    }

    fn consume(&mut self, mut bytes: &[u8], limit: usize) -> Result<Vec<Vec<u8>>, MpegTsError> {
        let mut completed = Vec::new();
        while !bytes.is_empty() {
            if self.buffer.is_empty() && bytes[0] == 0xff {
                break;
            }
            if self.expected.is_none() {
                let needed = 3_usize.saturating_sub(self.buffer.len());
                let take = needed.min(bytes.len());
                self.buffer.extend_from_slice(&bytes[..take]);
                bytes = &bytes[take..];
                if self.buffer.len() < 3 {
                    break;
                }
                let section_length =
                    (usize::from(self.buffer[1] & 0x0f) << 8) | usize::from(self.buffer[2]);
                let expected = 3_usize
                    .checked_add(section_length)
                    .ok_or(MpegTsError::InvalidPsi("section length overflows"))?;
                if expected > limit {
                    return Err(MpegTsError::PsiSectionTooLarge {
                        actual: expected,
                        limit,
                    });
                }
                if expected < 3 {
                    return Err(MpegTsError::InvalidPsi("section is too short"));
                }
                self.expected = Some(expected);
            }
            let expected = self.expected.unwrap();
            let needed = expected - self.buffer.len();
            let take = needed.min(bytes.len());
            self.buffer.extend_from_slice(&bytes[..take]);
            bytes = &bytes[take..];
            if self.buffer.len() == expected {
                completed.push(std::mem::take(&mut self.buffer));
                self.expected = None;
            }
        }
        Ok(completed)
    }

    fn is_partial(&self) -> bool {
        !self.buffer.is_empty()
    }
}

fn parse_pat(section: &[u8]) -> Result<Option<u16>, MpegTsError> {
    if section[0] != 0x00 {
        return Ok(None);
    }
    if section.len() < 12 {
        return Err(MpegTsError::InvalidPsi("PAT is too short"));
    }
    let end = section.len() - 4;
    if !(end - 8).is_multiple_of(4) {
        return Err(MpegTsError::InvalidPsi("PAT program loop is malformed"));
    }
    for entry in section[8..end].chunks_exact(4) {
        let program = u16::from_be_bytes([entry[0], entry[1]]);
        if program != 0 {
            return Ok(Some(
                (u16::from(entry[2] & 0x1f) << 8) | u16::from(entry[3]),
            ));
        }
    }
    Ok(None)
}

fn parse_pmt(section: &[u8]) -> Result<Option<u16>, MpegTsError> {
    if section[0] != 0x02 {
        return Ok(None);
    }
    if section.len() < 16 {
        return Err(MpegTsError::InvalidPsi("PMT is too short"));
    }
    let end = section.len() - 4;
    let program_info_length = (usize::from(section[10] & 0x0f) << 8) | usize::from(section[11]);
    let mut position = 12_usize
        .checked_add(program_info_length)
        .ok_or(MpegTsError::InvalidPsi("PMT descriptor length overflows"))?;
    if position > end {
        return Err(MpegTsError::InvalidPsi(
            "PMT descriptors exceed the section",
        ));
    }
    let mut selected = None;
    while position < end {
        if end - position < 5 {
            return Err(MpegTsError::InvalidPsi("PMT stream entry is truncated"));
        }
        let stream_type = section[position];
        let pid = (u16::from(section[position + 1] & 0x1f) << 8) | u16::from(section[position + 2]);
        let info_length =
            (usize::from(section[position + 3] & 0x0f) << 8) | usize::from(section[position + 4]);
        position = position
            .checked_add(5)
            .and_then(|value| value.checked_add(info_length))
            .ok_or(MpegTsError::InvalidPsi("PMT stream length overflows"))?;
        if position > end {
            return Err(MpegTsError::InvalidPsi(
                "PMT stream descriptors exceed the section",
            ));
        }
        if stream_type == ADTS_STREAM_TYPE {
            selected = Some(pid);
        }
    }
    Ok(selected)
}

fn parse_sdt(
    section: &[u8],
    metadata_limit: usize,
    metadata: &mut MpegTsMetadata,
) -> Result<(), MpegTsError> {
    if !matches!(section[0], 0x42 | 0x46) {
        return Ok(());
    }
    if section.len() < 15 {
        return Err(MpegTsError::InvalidPsi("SDT is too short"));
    }
    let end = section.len() - 4;
    let mut service_position = 11;
    while service_position < end {
        if end - service_position < 5 {
            return Err(MpegTsError::InvalidPsi("SDT service is truncated"));
        }
        let descriptor_length = (usize::from(section[service_position + 3] & 0x0f) << 8)
            | usize::from(section[service_position + 4]);
        let descriptor_start = service_position + 5;
        let descriptor_end = descriptor_start
            .checked_add(descriptor_length)
            .ok_or(MpegTsError::InvalidPsi("SDT descriptor length overflows"))?;
        if descriptor_end > end {
            return Err(MpegTsError::InvalidPsi(
                "SDT descriptors exceed the section",
            ));
        }
        let mut position = descriptor_start;
        while position < descriptor_end {
            if descriptor_end - position < 2 {
                return Err(MpegTsError::InvalidPsi("SDT descriptor is truncated"));
            }
            let tag = section[position];
            let length = usize::from(section[position + 1]);
            let data_start = position + 2;
            let data_end = data_start
                .checked_add(length)
                .ok_or(MpegTsError::InvalidPsi("SDT descriptor overflows"))?;
            if data_end > descriptor_end {
                return Err(MpegTsError::InvalidPsi("SDT descriptor exceeds its loop"));
            }
            if tag == 0x48 {
                parse_service_descriptor(&section[data_start..data_end], metadata_limit, metadata)?;
            }
            position = data_end;
        }
        service_position = descriptor_end;
    }
    Ok(())
}

fn parse_service_descriptor(
    data: &[u8],
    limit: usize,
    metadata: &mut MpegTsMetadata,
) -> Result<(), MpegTsError> {
    if data.len() < 3 {
        return Err(MpegTsError::InvalidPsi(
            "SDT service descriptor is too short",
        ));
    }
    let provider_length = usize::from(data[1]);
    let provider_end = 2_usize
        .checked_add(provider_length)
        .ok_or(MpegTsError::InvalidPsi("SDT provider length overflows"))?;
    let Some(&service_length) = data.get(provider_end) else {
        return Err(MpegTsError::InvalidPsi("SDT provider is truncated"));
    };
    let service_length = usize::from(service_length);
    let service_start = provider_end + 1;
    let service_end = service_start
        .checked_add(service_length)
        .ok_or(MpegTsError::InvalidPsi("SDT service name length overflows"))?;
    if service_end > data.len() {
        return Err(MpegTsError::InvalidPsi("SDT service name is truncated"));
    }
    for length in [provider_length, service_length] {
        if length > limit {
            return Err(MpegTsError::MetadataTooLarge {
                actual: length,
                limit,
            });
        }
    }
    metadata.service_provider = nonempty_ascii(&data[2..provider_end]);
    metadata.service_name = nonempty_ascii(&data[service_start..service_end]);
    Ok(())
}

fn nonempty_ascii(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    Some(
        bytes
            .iter()
            .map(|byte| {
                if byte.is_ascii() {
                    char::from(*byte)
                } else {
                    char::REPLACEMENT_CHARACTER
                }
            })
            .collect(),
    )
}

struct PesExtractor {
    header: Vec<u8>,
    header_total: Option<usize>,
    header_complete: bool,
    active: bool,
    remaining: Option<usize>,
    output: Vec<u8>,
    limit: usize,
}

impl PesExtractor {
    fn new(limit: usize) -> Self {
        Self {
            header: Vec::with_capacity(264),
            header_total: None,
            header_complete: false,
            active: false,
            remaining: None,
            output: Vec::new(),
            limit,
        }
    }

    fn push(&mut self, mut payload: &[u8], unit_start: bool) -> Result<(), MpegTsError> {
        if unit_start {
            self.finish_current()?;
            self.active = true;
            self.header.clear();
            self.header_total = None;
            self.header_complete = false;
            self.remaining = None;
        } else if !self.active {
            return Ok(());
        }

        while !self.header_complete && !payload.is_empty() {
            let target = self.header_total.unwrap_or(9);
            let needed = target.saturating_sub(self.header.len());
            let take = needed.min(payload.len());
            self.header.extend_from_slice(&payload[..take]);
            payload = &payload[take..];
            if self.header.len() >= 6 && (self.header[..3] != [0x00, 0x00, 0x01]) {
                return Err(MpegTsError::InvalidPes("start code is missing"));
            }
            if self.header.len() == 9 && self.header_total.is_none() {
                if self.header[6] & 0xc0 != 0x80 {
                    return Err(MpegTsError::InvalidPes(
                        "MPEG-2 optional-header marker is invalid",
                    ));
                }
                self.header_total = Some(9 + usize::from(self.header[8]));
            }
            if self.header_total == Some(self.header.len()) {
                let packet_length =
                    usize::from(u16::from_be_bytes([self.header[4], self.header[5]]));
                let optional_bytes = usize::from(self.header[8]);
                self.remaining = if packet_length == 0 {
                    None
                } else {
                    Some(packet_length.checked_sub(3 + optional_bytes).ok_or(
                        MpegTsError::InvalidPes("declared length is shorter than its header"),
                    )?)
                };
                self.header_complete = true;
                self.header.clear();
            }
        }
        if self.header_complete {
            self.append_payload(payload)?;
        }
        Ok(())
    }

    fn append_payload(&mut self, payload: &[u8]) -> Result<(), MpegTsError> {
        let length = self
            .remaining
            .map_or(payload.len(), |left| left.min(payload.len()));
        let actual =
            self.output
                .len()
                .checked_add(length)
                .ok_or(MpegTsError::PesPayloadTooLarge {
                    actual: usize::MAX,
                    limit: self.limit,
                })?;
        if actual > self.limit {
            return Err(MpegTsError::PesPayloadTooLarge {
                actual,
                limit: self.limit,
            });
        }
        self.output.extend_from_slice(&payload[..length]);
        if let Some(remaining) = self.remaining.as_mut() {
            *remaining -= length;
        }
        Ok(())
    }

    fn finish_current(&self) -> Result<(), MpegTsError> {
        if self.active
            && (!self.header_complete || self.remaining.is_some_and(|remaining| remaining != 0))
        {
            return Err(MpegTsError::TruncatedPes);
        }
        Ok(())
    }

    fn finish(self) -> Result<Vec<u8>, MpegTsError> {
        self.finish_current()?;
        Ok(self.output)
    }
}
