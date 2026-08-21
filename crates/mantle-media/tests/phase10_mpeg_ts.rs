use std::fs;
use std::path::{Path, PathBuf};

use mantle_media::{
    Codec, Container, MediaLimits, MpegTsError, MpegTsLimits, PcmFrame, extract_mpeg_ts_adts,
};

const PMT_PID: u16 = 0x0100;
const AUDIO_PID: u16 = 0x0101;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/media/fixtures")
        .join(name)
}

#[test]
fn extracts_pat_pmt_pes_adts_and_sdt_metadata_then_uses_the_existing_decoder() {
    let adts = fs::read(fixture("tone-aac-lc.adts")).unwrap();
    let transport = fs::read(fixture("tone-aac-lc.ts")).unwrap();

    let extracted = extract_mpeg_ts_adts(&transport, MpegTsLimits::default()).unwrap();
    assert_eq!(extracted.adts_bytes(), adts);
    assert_eq!(
        extracted.metadata().service_provider.as_deref(),
        Some("Mantle Provider")
    );
    assert_eq!(
        extracted.metadata().service_name.as_deref(),
        Some("Mantle Service")
    );

    let mut session = extracted
        .into_media_session(MediaLimits::default())
        .unwrap();
    assert_eq!(session.info().container, Container::Adts);
    assert_eq!(session.info().codec, Codec::AacLc);
    let mut pcm = PcmFrame::with_capacity(session.limits().max_pcm_samples_per_frame);
    let mut frames = 0;
    while session.read_pcm(&mut pcm).unwrap() {
        frames += 1;
    }
    assert!(frames > 10);
    assert!(!session.read_pcm(&mut pcm).unwrap());
}

#[test]
fn packet_psi_pes_output_and_metadata_limits_are_enforced() {
    let adts = fs::read(fixture("tone-aac-lc.adts")).unwrap();
    let transport = transport_stream(&adts, 0x0f, "provider", "service");
    let packet_count = transport.len() / 188;

    assert!(matches!(
        extract_mpeg_ts_adts(
            &transport,
            MpegTsLimits {
                max_packets: packet_count - 1,
                ..MpegTsLimits::default()
            },
        ),
        Err(MpegTsError::TooManyPackets { .. })
    ));
    assert!(matches!(
        extract_mpeg_ts_adts(
            &transport,
            MpegTsLimits {
                max_psi_section_bytes: 8,
                ..MpegTsLimits::default()
            },
        ),
        Err(MpegTsError::PsiSectionTooLarge { .. })
    ));
    assert!(matches!(
        extract_mpeg_ts_adts(
            &transport,
            MpegTsLimits {
                max_pes_payload_bytes: adts.len() - 1,
                ..MpegTsLimits::default()
            },
        ),
        Err(MpegTsError::PesPayloadTooLarge { .. })
    ));
    assert!(matches!(
        extract_mpeg_ts_adts(
            &transport,
            MpegTsLimits {
                max_metadata_string_bytes: 3,
                ..MpegTsLimits::default()
            },
        ),
        Err(MpegTsError::MetadataTooLarge { .. })
    ));
}

#[test]
fn malformed_transport_packets_continuity_and_pes_truncation_fail_explicitly() {
    let adts = fs::read(fixture("tone-aac-lc.adts")).unwrap();
    let transport = transport_stream(&adts, 0x0f, "provider", "service");

    let mut truncated_packet = transport.clone();
    truncated_packet.pop();
    assert!(matches!(
        extract_mpeg_ts_adts(&truncated_packet, MpegTsLimits::default()),
        Err(MpegTsError::TruncatedPacket { .. })
    ));

    let mut bad_sync = transport.clone();
    bad_sync[0] = 0;
    assert!(matches!(
        extract_mpeg_ts_adts(&bad_sync, MpegTsLimits::default()),
        Err(MpegTsError::InvalidPacket { packet: 0, .. })
    ));

    let mut discontinuous = transport.clone();
    let audio_packets = discontinuous
        .chunks_exact(188)
        .enumerate()
        .filter(|(_, packet)| packet_pid(packet) == AUDIO_PID)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let second = audio_packets[1] * 188;
    discontinuous[second + 3] = (discontinuous[second + 3] & 0xf0) | 0x0f;
    assert!(matches!(
        extract_mpeg_ts_adts(&discontinuous, MpegTsLimits::default()),
        Err(MpegTsError::Continuity { pid: AUDIO_PID, .. })
    ));

    let mut truncated_pes = transport;
    let last_audio = truncated_pes
        .chunks_exact(188)
        .enumerate()
        .filter(|(_, packet)| packet_pid(packet) == AUDIO_PID)
        .map(|(index, _)| index)
        .next_back()
        .unwrap();
    truncated_pes.drain(last_audio * 188..(last_audio + 1) * 188);
    assert!(matches!(
        extract_mpeg_ts_adts(&truncated_pes, MpegTsLimits::default()),
        Err(MpegTsError::TruncatedPes)
    ));
}

#[test]
fn missing_tables_or_adts_streams_fail_without_guessing_a_pid() {
    let adts = fs::read(fixture("tone-aac-lc.adts")).unwrap();
    let no_pat = transport_stream(&adts, 0x0f, "provider", "service")[188..].to_vec();
    assert!(matches!(
        extract_mpeg_ts_adts(&no_pat, MpegTsLimits::default()),
        Err(MpegTsError::MissingProgramMap)
    ));

    let wrong_stream_type = transport_stream(&adts, 0x1b, "provider", "service");
    assert!(matches!(
        extract_mpeg_ts_adts(&wrong_stream_type, MpegTsLimits::default()),
        Err(MpegTsError::MissingAdtsStream)
    ));

    let mut removed_stream = transport_stream(&adts, 0x0f, "provider", "service");
    removed_stream.extend_from_slice(&psi_packet(PMT_PID, &pmt_section(0x1b), 1));
    assert!(matches!(
        extract_mpeg_ts_adts(&removed_stream, MpegTsLimits::default()),
        Err(MpegTsError::MissingAdtsStream)
    ));
}

#[test]
fn every_mpeg_ts_limit_must_be_nonzero() {
    let bytes = [0_u8; 188];
    for limits in [
        MpegTsLimits {
            max_packets: 0,
            ..MpegTsLimits::default()
        },
        MpegTsLimits {
            max_psi_section_bytes: 0,
            ..MpegTsLimits::default()
        },
        MpegTsLimits {
            max_pes_payload_bytes: 0,
            ..MpegTsLimits::default()
        },
        MpegTsLimits {
            max_metadata_string_bytes: 0,
            ..MpegTsLimits::default()
        },
    ] {
        assert!(matches!(
            extract_mpeg_ts_adts(&bytes, limits),
            Err(MpegTsError::InvalidLimits(_))
        ));
    }
}

fn transport_stream(adts: &[u8], stream_type: u8, provider: &str, service: &str) -> Vec<u8> {
    let mut stream = Vec::new();
    stream.extend_from_slice(&psi_packet(0, &pat_section(), 0));
    stream.extend_from_slice(&psi_packet(PMT_PID, &pmt_section(stream_type), 0));
    stream.extend_from_slice(&psi_packet(0x0011, &sdt_section(provider, service), 0));

    let pes_length = adts.len().checked_add(3).unwrap();
    let mut pes = Vec::with_capacity(adts.len() + 9);
    pes.extend_from_slice(&[0x00, 0x00, 0x01, 0xc0]);
    pes.extend_from_slice(&u16::try_from(pes_length).unwrap().to_be_bytes());
    pes.extend_from_slice(&[0x80, 0x00, 0x00]);
    pes.extend_from_slice(adts);
    stream.extend_from_slice(&payload_packets(AUDIO_PID, &pes));
    stream
}

fn pat_section() -> Vec<u8> {
    vec![
        0x00, 0xb0, 0x0d, 0x00, 0x01, 0xc1, 0x00, 0x00, 0x00, 0x01, 0xe1, 0x00, 0, 0, 0, 0,
    ]
}

fn pmt_section(stream_type: u8) -> Vec<u8> {
    vec![
        0x02,
        0xb0,
        0x12,
        0x00,
        0x01,
        0xc1,
        0x00,
        0x00,
        0xe1,
        0x01,
        0xf0,
        0x00,
        stream_type,
        0xe1,
        0x01,
        0xf0,
        0x00,
        0,
        0,
        0,
        0,
    ]
}

fn sdt_section(provider: &str, service: &str) -> Vec<u8> {
    let descriptor_length = 3 + provider.len() + service.len();
    let service_loop_length = 7 + descriptor_length;
    let section_length = 8 + service_loop_length + 4;
    let mut section = vec![
        0x42,
        0xf0 | u8::try_from(section_length >> 8).unwrap(),
        u8::try_from(section_length & 0xff).unwrap(),
        0x00,
        0x01,
        0xc1,
        0x00,
        0x00,
        0x00,
        0x01,
        0xff,
        0x00,
        0x01,
        0xfc,
        0xf0 | u8::try_from((descriptor_length + 2) >> 8).unwrap(),
        u8::try_from((descriptor_length + 2) & 0xff).unwrap(),
        0x48,
        u8::try_from(descriptor_length).unwrap(),
        0x01,
        u8::try_from(provider.len()).unwrap(),
    ];
    section.extend_from_slice(provider.as_bytes());
    section.push(u8::try_from(service.len()).unwrap());
    section.extend_from_slice(service.as_bytes());
    section.extend_from_slice(&[0, 0, 0, 0]);
    section
}

fn psi_packet(pid: u16, section: &[u8], continuity: u8) -> [u8; 188] {
    assert!(section.len() < 184);
    let mut payload = Vec::with_capacity(section.len() + 1);
    payload.push(0);
    payload.extend_from_slice(section);
    single_packet(pid, true, continuity, &payload)
}

fn payload_packets(pid: u16, payload: &[u8]) -> Vec<u8> {
    let mut packets = Vec::new();
    let mut position = 0;
    let mut continuity = 0;
    while position < payload.len() {
        let length = (payload.len() - position).min(184);
        packets.extend_from_slice(&single_packet(
            pid,
            position == 0,
            continuity,
            &payload[position..position + length],
        ));
        position += length;
        continuity = (continuity + 1) & 0x0f;
    }
    packets
}

fn single_packet(pid: u16, unit_start: bool, continuity: u8, payload: &[u8]) -> [u8; 188] {
    assert!(!payload.is_empty() && payload.len() <= 184);
    let mut packet = [0xff; 188];
    packet[0] = 0x47;
    packet[1] = u8::try_from(pid >> 8).unwrap() & 0x1f;
    if unit_start {
        packet[1] |= 0x40;
    }
    packet[2] = u8::try_from(pid & 0xff).unwrap();
    if payload.len() == 184 {
        packet[3] = 0x10 | continuity;
        packet[4..].copy_from_slice(payload);
    } else {
        packet[3] = 0x30 | continuity;
        let adaptation_length = 183 - payload.len();
        packet[4] = u8::try_from(adaptation_length).unwrap();
        if adaptation_length > 0 {
            packet[5] = 0;
        }
        let payload_start = 5 + adaptation_length;
        packet[payload_start..].copy_from_slice(payload);
    }
    packet
}

fn packet_pid(packet: &[u8]) -> u16 {
    (u16::from(packet[1] & 0x1f) << 8) | u16::from(packet[2])
}
