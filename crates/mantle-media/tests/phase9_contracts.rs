mod support;

use std::fs;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use mantle_media::{
    Codec, Container, EncodedPacket, MediaCancellation, MediaError, MediaInput, MediaLimits,
    MediaMetadata, MediaSession, MemoryInput, PcmFrame,
};
use support::{PcmConformanceCase, assert_pcm_conformance};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/media/fixtures")
        .join(name)
}

#[test]
fn tagged_flac_satisfies_the_shared_pcm_contract() {
    let path = fixture("tone-metadata.flac");
    assert_pcm_conformance(&PcmConformanceCase {
        path: &path,
        container: Container::Flac,
        codec: Codec::Flac,
        metadata: MediaMetadata {
            title: Some("Mantle Fixture Title".to_owned()),
            author: Some("Mantle Fixture Artist".to_owned()),
            isrc: Some("BRMNT2600001".to_owned()),
        },
        seek_to: std::time::Duration::from_secs(2),
    });
}

#[test]
fn malformed_and_truncated_native_flac_terminate_within_bounds() {
    let bytes = fs::read(fixture("tone-metadata.flac")).unwrap();
    let first_frame = flac_first_frame_offset(&bytes).expect("fixture metadata should terminate");
    for end in [
        0,
        4,
        8,
        41,
        42,
        first_frame - 1,
        first_frame,
        bytes.len() / 2,
        bytes.len() - 1,
    ] {
        assert_bounded_media_termination(&bytes[..end], "flac", end);
    }

    let mut wrong_stream_info_type = bytes.clone();
    wrong_stream_info_type[4] = 4;
    assert_bounded_media_termination(&wrong_stream_info_type, "flac", usize::MAX);

    let mut short_stream_info = bytes.clone();
    short_stream_info[5..8].copy_from_slice(&[0, 0, 33]);
    assert_bounded_media_termination(&short_stream_info, "flac", usize::MAX - 1);

    let mut oversized_comment = bytes.clone();
    oversized_comment[43..46].copy_from_slice(&[0xff; 3]);
    assert_bounded_media_termination(&oversized_comment, "flac", usize::MAX - 2);

    let mut corrupted_frame = bytes;
    let corrupt_end = (first_frame + 32).min(corrupted_frame.len());
    corrupted_frame[first_frame..corrupt_end].fill(0xff);
    assert_bounded_media_termination(&corrupted_frame, "flac", usize::MAX - 3);
}

fn flac_first_frame_offset(bytes: &[u8]) -> Option<usize> {
    if !bytes.starts_with(b"fLaC") {
        return None;
    }
    let mut position = 4_usize;
    loop {
        let header = *bytes.get(position)?;
        let length = (usize::from(*bytes.get(position + 1)?) << 16)
            | (usize::from(*bytes.get(position + 2)?) << 8)
            | usize::from(*bytes.get(position + 3)?);
        position = position.checked_add(4)?.checked_add(length)?;
        if position > bytes.len() {
            return None;
        }
        if header & 0x80 != 0 {
            return Some(position);
        }
    }
}

#[test]
fn current_decoded_formats_satisfy_the_shared_pcm_contract() {
    for (name, container, codec) in [
        ("tone-pcm-s16le.wav", Container::Wave, Codec::PcmS16Le),
        ("tone-mp3.mp3", Container::Mp3, Codec::Mp3),
        ("tone-aac-lc.m4a", Container::Mp4, Codec::AacLc),
        ("tone-he-aac-v1.m4a", Container::Mp4, Codec::HeAacV1),
        ("tone-he-aac-v2.m4a", Container::Mp4, Codec::HeAacV2),
        ("tone-flac.flac", Container::Flac, Codec::Flac),
    ] {
        let path = fixture(name);
        assert_pcm_conformance(&PcmConformanceCase {
            path: &path,
            container,
            codec,
            metadata: MediaMetadata::default(),
            seek_to: std::time::Duration::from_secs(2),
        });
    }
}

#[test]
fn high_bit_depth_extensible_wav_satisfies_the_shared_pcm_contract() {
    for (name, codec) in [
        ("tone-pcm-s24le-extensible.wav", Codec::PcmS24Le),
        ("tone-pcm-s32le-extensible.wav", Codec::PcmS32Le),
    ] {
        let path = fixture(name);
        assert_pcm_conformance(&PcmConformanceCase {
            path: &path,
            container: Container::Wave,
            codec,
            metadata: MediaMetadata::default(),
            seek_to: std::time::Duration::from_millis(100),
        });
    }
}

#[test]
fn wav_sample_widths_decode_to_the_same_signal() {
    let s16 = decode_pcm("tone-pcm-s16le.wav");
    let s24 = decode_pcm("tone-pcm-s24le-extensible.wav");
    let s32 = decode_pcm("tone-pcm-s32le-extensible.wav");
    assert_eq!(s24.len(), 48_000);
    assert_eq!(s32.len(), s24.len());
    assert_eq!(&s16[..s24.len()], s24);
    assert_eq!(s24, s32);
}

#[test]
fn wav_mono_and_sample_rate_edges_satisfy_the_current_pcm_scope() {
    for (name, sample_rate, channels) in [
        ("tone-pcm-s16le-mono-8k.wav", 8_000, 1),
        ("tone-pcm-s16le-stereo-384k.wav", 384_000, 2),
    ] {
        let path = fixture(name);
        assert_pcm_conformance(&PcmConformanceCase {
            path: &path,
            container: Container::Wave,
            codec: Codec::PcmS16Le,
            metadata: MediaMetadata::default(),
            seek_to: std::time::Duration::from_millis(100),
        });

        let mut session = MediaSession::open_file(path, MediaLimits::default()).unwrap();
        assert_eq!(session.info().sample_rate, sample_rate);
        assert_eq!(session.info().channels, channels);
        let mut output = PcmFrame::with_capacity(session.limits().max_pcm_samples_per_frame);
        assert!(session.read_pcm(&mut output).unwrap());
        assert_eq!(output.sample_rate(), sample_rate);
        assert_eq!(output.channels(), channels);
    }
}

#[test]
fn wav_rejects_reference_invalid_channel_and_rate_headers() {
    let bytes = fs::read(fixture("tone-pcm-s16le.wav")).unwrap();
    for (offset, replacement) in [
        (22, 0_u16.to_le_bytes().to_vec()),
        (24, 99_u32.to_le_bytes().to_vec()),
        (24, 384_001_u32.to_le_bytes().to_vec()),
    ] {
        let mut invalid = bytes.clone();
        invalid[offset..offset + replacement.len()].copy_from_slice(&replacement);
        assert!(
            MediaSession::open(
                Box::new(MemoryInput::new(invalid)),
                Some("wav"),
                MediaLimits::default(),
            )
            .is_err()
        );
    }

    let mut outside_pcm_scope = bytes;
    outside_pcm_scope[22..24].copy_from_slice(&3_u16.to_le_bytes());
    outside_pcm_scope[28..32].copy_from_slice(&288_000_u32.to_le_bytes());
    outside_pcm_scope[32..34].copy_from_slice(&6_u16.to_le_bytes());
    assert!(matches!(
        MediaSession::open(
            Box::new(MemoryInput::new(outside_pcm_scope)),
            Some("wav"),
            MediaLimits::default(),
        ),
        Err(MediaError::UnsupportedCodec(message))
            if message == "WAVE PCM channel count 3 is outside Mantle's mono/stereo scope"
    ));
}

#[test]
fn malformed_and_truncated_extensible_wav_terminate_within_bounds() {
    let bytes = fs::read(fixture("tone-pcm-s24le-extensible.wav")).unwrap();
    assert_eq!(&bytes[20..22], &0xfffe_u16.to_le_bytes());
    assert_eq!(
        &bytes[44..60],
        &[1, 0, 0, 0, 0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113]
    );
    let classic = fs::read(fixture("tone-pcm-s16le.wav")).unwrap();
    assert_eq!(&classic[20..22], &1_u16.to_le_bytes());
    for end in [0, 12, 20, 44, 60, bytes.len() / 2, bytes.len() - 1] {
        let input = MemoryInput::new(bytes[..end].to_vec());
        let Ok(mut session) =
            MediaSession::open(Box::new(input), Some("wav"), MediaLimits::default())
        else {
            continue;
        };
        let mut output = PcmFrame::with_capacity(session.limits().max_pcm_samples_per_frame);
        let mut terminated = false;
        for _ in 0..100 {
            if !matches!(session.read_pcm(&mut output), Ok(true)) {
                terminated = true;
                break;
            }
        }
        assert!(terminated, "truncation at {end} bytes did not terminate");
    }

    let mut non_pcm_subtype = bytes;
    non_pcm_subtype[44..48].copy_from_slice(&3_u32.to_le_bytes());
    let result = MediaSession::open(
        Box::new(MemoryInput::new(non_pcm_subtype)),
        Some("wav"),
        MediaLimits::default(),
    );
    assert!(
        matches!(
            &result,
            Err(MediaError::Backend {
                operation: "probe",
                ..
            })
        ),
        "{:?}",
        result.err()
    );
}

#[test]
fn cbr_and_tagged_xing_vbr_mp3_satisfy_the_shared_contract() {
    let path = fixture("tone-mp3-vbr-id3.mp3");
    assert_pcm_conformance(&PcmConformanceCase {
        path: &path,
        container: Container::Mp3,
        codec: Codec::Mp3,
        metadata: MediaMetadata {
            title: Some("Mantle VBR Title".to_owned()),
            author: Some("Mantle VBR Artist".to_owned()),
            isrc: Some("BRMNT2600002".to_owned()),
        },
        seek_to: std::time::Duration::from_millis(500),
    });

    let cbr = fs::read(fixture("tone-mp3.mp3")).unwrap();
    assert!(!contains_ascii(&cbr, b"Xing"));
    assert!(!contains_ascii(&cbr, b"Info"));
    let vbr = fs::read(path).unwrap();
    assert_eq!(&vbr[..3], b"ID3");
    assert!(contains_ascii(&vbr, b"Xing"));
}

#[test]
fn malformed_and_truncated_mp3_terminate_within_bounds() {
    let bytes = fs::read(fixture("tone-mp3-vbr-id3.mp3")).unwrap();
    for end in [0, 3, 10, 142, bytes.len() / 2, bytes.len() - 1] {
        assert_bounded_pcm_termination(&bytes[..end], "mp3", end);
    }

    let mut oversized_id3 = bytes;
    oversized_id3[6..10].copy_from_slice(&[0x7f; 4]);
    assert!(
        MediaSession::open(
            Box::new(MemoryInput::new(oversized_id3)),
            Some("mp3"),
            MediaLimits::default(),
        )
        .is_err()
    );

    let oversized_frame = b"ID3\x03\x00\x00\x00\x00\x00\x0aTIT2\xfe\xff\xdf\xff\x00\x00";
    let result = MediaSession::open(
        Box::new(MemoryInput::new(oversized_frame.as_slice())),
        Some("mp3"),
        MediaLimits::default(),
    );
    assert!(
        matches!(
            result,
            Err(MediaError::Backend {
                operation: "metadata preflight",
                ref message,
            }) if message.contains("4278181887 bytes")
        ),
        "{:?}",
        result.err()
    );
}

#[test]
fn forward_only_tagged_mp3_uses_bounded_metadata_preflight() {
    let bytes = fs::read(fixture("tone-mp3-vbr-id3.mp3")).unwrap();
    let mut session = MediaSession::open(
        Box::new(ForwardOnlyInput::new(bytes)),
        Some("mp3"),
        MediaLimits::default(),
    )
    .unwrap();
    assert!(!session.info().seekable);
    assert_eq!(
        session.info().metadata,
        MediaMetadata {
            title: Some("Mantle VBR Title".to_owned()),
            author: Some("Mantle VBR Artist".to_owned()),
            isrc: Some("BRMNT2600002".to_owned()),
        }
    );
    let mut frame = PcmFrame::with_capacity(session.limits().max_pcm_samples_per_frame);
    assert!(session.read_pcm(&mut frame).unwrap());
}

#[test]
fn standard_and_fragmented_mp4_satisfy_the_shared_contract() {
    for (name, title, author, seek_millis) in [
        (
            "tone-aac-lc-metadata.m4a",
            "Mantle MP4 Title",
            "Mantle MP4 Artist",
            500,
        ),
        (
            "tone-aac-lc-fragmented.m4a",
            "Mantle Fragmented Title",
            "Mantle Fragmented Artist",
            2_500,
        ),
    ] {
        let path = fixture(name);
        assert_pcm_conformance(&PcmConformanceCase {
            path: &path,
            container: Container::Mp4,
            codec: Codec::AacLc,
            metadata: MediaMetadata {
                title: Some(title.to_owned()),
                author: Some(author.to_owned()),
                isrc: None,
            },
            seek_to: std::time::Duration::from_millis(seek_millis),
        });
    }

    let standard = fs::read(fixture("tone-aac-lc-metadata.m4a")).unwrap();
    assert!(!contains_ascii(&standard, b"mvex"));
    assert!(!contains_ascii(&standard, b"moof"));
    let fragmented = fs::read(fixture("tone-aac-lc-fragmented.m4a")).unwrap();
    assert!(contains_ascii(&fragmented, b"mvex"));
    assert!(contains_ascii(&fragmented, b"sidx"));
    assert_eq!(count_ascii(&fragmented, b"moof"), 8);
}

#[test]
fn malformed_and_truncated_fragmented_mp4_terminate_within_bounds() {
    let bytes = fs::read(fixture("tone-aac-lc-fragmented.m4a")).unwrap();
    let first_moof = find_ascii(&bytes, b"moof").unwrap();
    for end in [
        0,
        8,
        first_moof - 4,
        first_moof + 4,
        first_moof + 8,
        bytes.len() / 2,
        bytes.len() - 1,
    ] {
        assert_bounded_pcm_termination(&bytes[..end], "m4a", end);
    }

    let mut invalid_moof_size = bytes;
    invalid_moof_size[first_moof - 4..first_moof].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_bounded_pcm_termination(&invalid_moof_size, "m4a", usize::MAX);
}

#[test]
fn ogg_opus_flac_and_vorbis_satisfy_the_shared_contracts() {
    for (name, codec, title, author, isrc) in [
        (
            "tone-flac-tags.oga",
            Codec::Flac,
            "Mantle Ogg FLAC Title",
            "Mantle Ogg FLAC Artist",
            "BRMNT2600005",
        ),
        (
            "tone-vorbis-tags.ogg",
            Codec::Vorbis,
            "Mantle Ogg Vorbis Title",
            "Mantle Ogg Vorbis Artist",
            "BRMNT2600004",
        ),
    ] {
        let path = fixture(name);
        assert_pcm_conformance(&PcmConformanceCase {
            path: &path,
            container: Container::Ogg,
            codec,
            metadata: MediaMetadata {
                title: Some(title.to_owned()),
                author: Some(author.to_owned()),
                isrc: Some(isrc.to_owned()),
            },
            seek_to: std::time::Duration::from_millis(500),
        });
    }

    assert_opus_conformance(
        "tone-opus-tags.ogg",
        Container::Ogg,
        &MediaMetadata {
            title: Some("Mantle Ogg Opus Title".to_owned()),
            author: Some("Mantle Ogg Opus Artist".to_owned()),
            isrc: Some("BRMNT2600003".to_owned()),
        },
    );
}

fn assert_opus_conformance(name: &str, container: Container, metadata: &MediaMetadata) {
    let path = fixture(name);
    let cancellation = MediaCancellation::new();
    let mut session = MediaSession::open_file_with_cancellation(
        &path,
        MediaLimits::default(),
        cancellation.clone(),
    )
    .unwrap();
    assert_eq!(session.info().container, container);
    assert_eq!(session.info().codec, Codec::Opus);
    assert_eq!(&session.info().metadata, metadata);
    let seek = session.seek(std::time::Duration::from_millis(500)).unwrap();
    assert!(seek.actual.unwrap().abs_diff(seek.requested) <= std::time::Duration::from_millis(100));

    let mut output = EncodedPacket::with_capacity(session.limits().max_packet_bytes);
    let mut previous = None;
    let mut packets = 0;
    while session.read_encoded(&mut output).unwrap() {
        let timestamp = output.timestamp().unwrap();
        assert!(previous.is_none_or(|value| timestamp >= value));
        assert!(
            output
                .duration()
                .is_some_and(|duration| !duration.is_zero())
        );
        assert!(!output.data().is_empty());
        previous = Some(timestamp);
        packets += 1;
    }
    assert!(packets > 0);
    assert!(!session.read_encoded(&mut output).unwrap());

    let mut cancelled = MediaSession::open_file_with_cancellation(
        path,
        MediaLimits::default(),
        cancellation.clone(),
    )
    .unwrap();
    cancellation.cancel();
    assert!(matches!(
        cancelled.read_encoded(&mut output),
        Err(MediaError::Cancelled)
    ));
}

#[test]
fn malformed_and_truncated_ogg_mappings_terminate_within_bounds() {
    for (name, extension) in [
        ("tone-opus-tags.ogg", "ogg"),
        ("tone-vorbis-tags.ogg", "ogg"),
        ("tone-flac-tags.oga", "oga"),
    ] {
        let bytes = fs::read(fixture(name)).unwrap();
        assert_eq!(&bytes[..4], b"OggS");
        for end in [0, 4, 27, bytes.len() / 2, bytes.len() - 1] {
            assert_bounded_media_termination(&bytes[..end], extension, end);
        }

        let mut invalid_page = bytes;
        invalid_page[4] = u8::MAX;
        assert_bounded_media_termination(&invalid_page, extension, usize::MAX);
    }
}

#[test]
fn matroska_vorbis_and_aac_profiles_satisfy_the_shared_contract() {
    for (name, codec, metadata, seek_millis) in [
        (
            "tone-vorbis-tags.mkv",
            Codec::Vorbis,
            MediaMetadata {
                title: Some("Mantle Matroska Vorbis Title".to_owned()),
                author: Some("Mantle Matroska Vorbis Artist".to_owned()),
                isrc: Some("BRMNT2600006".to_owned()),
            },
            500,
        ),
        (
            "tone-aac-lc-tags.mkv",
            Codec::AacLc,
            MediaMetadata {
                title: Some("Mantle Matroska AAC Title".to_owned()),
                author: Some("Mantle Matroska AAC Artist".to_owned()),
                isrc: None,
            },
            500,
        ),
        (
            "tone-aac-lc-24k.mkv",
            Codec::AacLc,
            MediaMetadata::default(),
            500,
        ),
        (
            "tone-he-aac-v1.mkv",
            Codec::HeAacV1,
            MediaMetadata::default(),
            2_000,
        ),
        (
            "tone-he-aac-v2.mkv",
            Codec::HeAacV2,
            MediaMetadata::default(),
            2_000,
        ),
    ] {
        let path = fixture(name);
        assert_pcm_conformance(&PcmConformanceCase {
            path: &path,
            container: Container::Matroska,
            codec,
            metadata,
            seek_to: std::time::Duration::from_millis(seek_millis),
        });
    }

    assert_opus_conformance("tone-opus.webm", Container::WebM, &MediaMetadata::default());

    let lc_24k =
        MediaSession::open_file(fixture("tone-aac-lc-24k.mkv"), MediaLimits::default()).unwrap();
    assert_eq!(lc_24k.info().codec, Codec::AacLc);
    assert_eq!(lc_24k.info().sample_rate, 24_000);

    let mut rewound =
        MediaSession::open_file(fixture("tone-he-aac-v1.mkv"), MediaLimits::default()).unwrap();
    let mut first_frame = PcmFrame::with_capacity(rewound.limits().max_pcm_samples_per_frame);
    assert!(rewound.read_pcm(&mut first_frame).unwrap());
    assert_eq!(first_frame.timestamp(), Some(std::time::Duration::ZERO));
}

#[test]
fn malformed_and_truncated_ebml_mappings_terminate_within_bounds() {
    for (name, extension, document_type) in [
        ("tone-opus.webm", "webm", b"webm".as_slice()),
        ("tone-vorbis-tags.mkv", "mkv", b"matroska".as_slice()),
        ("tone-aac-lc-tags.mkv", "mkv", b"matroska".as_slice()),
        ("tone-aac-lc-24k.mkv", "mkv", b"matroska".as_slice()),
        ("tone-he-aac-v1.mkv", "mkv", b"matroska".as_slice()),
        ("tone-he-aac-v2.mkv", "mkv", b"matroska".as_slice()),
    ] {
        let bytes = fs::read(fixture(name)).unwrap();
        assert_eq!(&bytes[..4], &[0x1a, 0x45, 0xdf, 0xa3]);
        assert!(contains_ascii(
            &bytes[..bytes.len().min(4_096)],
            document_type
        ));
        for end in [0, 4, 16, bytes.len() / 2, bytes.len() - 1] {
            assert_bounded_media_termination(&bytes[..end], extension, end);
        }

        let mut invalid_ebml = bytes;
        invalid_ebml[4] = u8::MAX;
        assert_bounded_media_termination(&invalid_ebml, extension, usize::MAX);
    }
}

#[test]
fn raw_adts_crc_and_aac_profiles_satisfy_the_sequential_contract() {
    for (name, codec) in [
        ("tone-aac-lc.adts", Codec::AacLc),
        ("tone-aac-lc-crc.adts", Codec::AacLc),
        ("tone-he-aac-v1.adts", Codec::HeAacV1),
        ("tone-he-aac-v2.adts", Codec::HeAacV2),
    ] {
        assert_adts_conformance(name, codec);
    }
}

fn assert_adts_conformance(name: &str, codec: Codec) {
    let path = fixture(name);
    let cancellation = MediaCancellation::new();
    let mut session = MediaSession::open_file_with_cancellation(
        &path,
        MediaLimits::default(),
        cancellation.clone(),
    )
    .unwrap();
    assert_eq!(session.info().container, Container::Adts);
    assert_eq!(session.info().codec, codec);
    assert_eq!(session.info().sample_rate, 48_000);
    assert_eq!(session.info().channels, 2);
    assert_eq!(session.info().duration, None);
    assert!(!session.info().seekable);
    assert_eq!(session.info().metadata, MediaMetadata::default());
    assert!(session.seek(std::time::Duration::ZERO).is_err());

    let mut output = PcmFrame::with_capacity(session.limits().max_pcm_samples_per_frame);
    let mut previous = None;
    let mut frames = 0;
    while session.read_pcm(&mut output).unwrap() {
        let timestamp = output.timestamp().unwrap();
        assert!(previous.is_none_or(|value| timestamp >= value));
        assert_eq!(output.sample_rate(), 48_000);
        assert_eq!(output.channels(), 2);
        previous = Some(timestamp);
        frames += 1;
    }
    assert!(frames > 10);
    assert!(!session.read_pcm(&mut output).unwrap());

    let mut cancelled = MediaSession::open_file_with_cancellation(
        path,
        MediaLimits::default(),
        cancellation.clone(),
    )
    .unwrap();
    assert!(cancelled.read_pcm(&mut output).unwrap());
    cancellation.cancel();
    assert!(matches!(
        cancelled.read_pcm(&mut output),
        Err(MediaError::Cancelled)
    ));
}

#[test]
fn raw_adts_scans_crc_and_malformed_headers_within_bounds() {
    let crc = fs::read(fixture("tone-aac-lc-crc.adts")).unwrap();
    assert_eq!(&crc[..2], &[0xff, 0xf0]);
    let no_crc = fs::read(fixture("tone-aac-lc.adts")).unwrap();
    assert_eq!(&no_crc[..2], &[0xff, 0xf1]);

    let mut prefixed = vec![0x55; 31];
    prefixed.extend_from_slice(&no_crc);
    let mut scanned = MediaSession::open(
        Box::new(MemoryInput::new(prefixed)),
        Some("aac"),
        MediaLimits::default(),
    )
    .unwrap();
    assert_eq!(scanned.info().container, Container::Adts);
    let mut output = PcmFrame::with_capacity(scanned.limits().max_pcm_samples_per_frame);
    assert!(scanned.read_pcm(&mut output).unwrap());
    assert_eq!(output.timestamp(), Some(std::time::Duration::ZERO));

    for bytes in [no_crc.as_slice(), crc.as_slice()] {
        let first_length = adts_frame_length(bytes);
        for end in [
            0,
            1,
            7,
            9,
            first_length - 1,
            bytes.len() / 2,
            bytes.len() - 1,
        ] {
            assert_bounded_media_termination(&bytes[..end], "aac", end);
        }

        let mut invalid_length = bytes.to_vec();
        invalid_length[3] &= 0xfc;
        invalid_length[4] = 0;
        invalid_length[5] &= 0x1f;
        assert_bounded_media_termination(&invalid_length, "aac", usize::MAX);

        let mut multiple_blocks = bytes.to_vec();
        multiple_blocks[6] |= 1;
        assert_bounded_media_termination(&multiple_blocks, "aac", usize::MAX - 1);
    }

    let mut unsupported_main = no_crc;
    unsupported_main[2] &= 0x3f;
    let result = MediaSession::open(
        Box::new(MemoryInput::new(unsupported_main)),
        Some("aac"),
        MediaLimits::default(),
    );
    assert!(matches!(
        result,
        Err(MediaError::UnsupportedCodecProfile {
            codec: "AAC",
            profile: "ADTS audio object type is not AAC-LC",
        })
    ));
}

#[test]
fn raw_adts_profile_probe_is_bounded_and_supports_forward_only_input() {
    let bytes = fs::read(fixture("tone-he-aac-v1.adts")).unwrap();
    let limits = MediaLimits {
        max_codec_probe_bytes: 8,
        ..MediaLimits::default()
    };
    let result = MediaSession::open(
        Box::new(MemoryInput::new(bytes.clone())),
        Some("aac"),
        limits,
    );
    assert!(matches!(
        result,
        Err(MediaError::CodecProbeLimitExceeded { limit: 8, .. })
    ));

    let input = ForwardOnlyInput::new(bytes);
    let mut session = MediaSession::open(Box::new(input), Some("aac"), MediaLimits::default())
        .expect("forward-only ADTS should open without rewinding consumed profile packets");
    assert_eq!(session.info().container, Container::Adts);
    assert_eq!(session.info().codec, Codec::HeAacV1);
    assert_eq!(session.info().sample_rate, 48_000);
    assert!(!session.info().seekable);
    let mut output = PcmFrame::with_capacity(session.limits().max_pcm_samples_per_frame);
    assert!(session.read_pcm(&mut output).unwrap());
    assert_eq!(output.timestamp(), Some(std::time::Duration::ZERO));
}

#[test]
fn zero_codec_probe_limit_is_rejected() {
    let limits = MediaLimits {
        max_codec_probe_bytes: 0,
        ..MediaLimits::default()
    };
    assert!(matches!(
        MediaSession::open_file(fixture("tone-aac-lc.adts"), limits),
        Err(MediaError::InvalidLimits(
            "max_codec_probe_bytes must be non-zero"
        ))
    ));
}

fn adts_frame_length(bytes: &[u8]) -> usize {
    (usize::from(bytes[3] & 3) << 11) | (usize::from(bytes[4]) << 3) | (usize::from(bytes[5]) >> 5)
}

struct ForwardOnlyInput {
    inner: Cursor<Box<[u8]>>,
}

impl ForwardOnlyInput {
    fn new(bytes: impl Into<Box<[u8]>>) -> Self {
        Self {
            inner: Cursor::new(bytes.into()),
        }
    }
}

impl Read for ForwardOnlyInput {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buffer)
    }
}

impl Seek for ForwardOnlyInput {
    fn seek(&mut self, _position: SeekFrom) -> io::Result<u64> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "input is forward-only",
        ))
    }
}

impl MediaInput for ForwardOnlyInput {
    fn is_seekable(&self) -> bool {
        false
    }

    fn byte_len(&self) -> Option<u64> {
        None
    }
}

fn decode_pcm(name: &str) -> Vec<f32> {
    let mut session = MediaSession::open_file(fixture(name), MediaLimits::default()).unwrap();
    let mut output = PcmFrame::with_capacity(session.limits().max_pcm_samples_per_frame);
    let mut samples = Vec::new();
    while session.read_pcm(&mut output).unwrap() {
        samples.extend_from_slice(output.samples());
    }
    samples
}

fn assert_bounded_pcm_termination(bytes: &[u8], extension: &str, case: usize) {
    assert_bounded_media_termination(bytes, extension, case);
}

fn assert_bounded_media_termination(bytes: &[u8], extension: &str, case: usize) {
    let input = MemoryInput::new(bytes.to_vec());
    let Ok(mut session) =
        MediaSession::open(Box::new(input), Some(extension), MediaLimits::default())
    else {
        return;
    };
    let mut pcm = PcmFrame::with_capacity(session.limits().max_pcm_samples_per_frame);
    let mut encoded = EncodedPacket::with_capacity(session.limits().max_packet_bytes);
    for _ in 0..2_048 {
        let result = if session.info().codec == Codec::Opus {
            session.read_encoded(&mut encoded)
        } else {
            session.read_pcm(&mut pcm)
        };
        if !matches!(result, Ok(true)) {
            return;
        }
    }
    panic!("malformed case {case} did not terminate");
}

fn contains_ascii(bytes: &[u8], needle: &[u8]) -> bool {
    bytes.windows(needle.len()).any(|window| window == needle)
}

fn count_ascii(bytes: &[u8], needle: &[u8]) -> usize {
    bytes
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count()
}

fn find_ascii(bytes: &[u8], needle: &[u8]) -> Option<usize> {
    bytes
        .windows(needle.len())
        .position(|window| window == needle)
}

#[test]
fn cancellation_before_open_stops_probe() {
    let cancellation = MediaCancellation::new();
    cancellation.cancel();
    assert!(matches!(
        MediaSession::open_file_with_cancellation(
            fixture("tone-flac.flac"),
            MediaLimits::default(),
            cancellation,
        ),
        Err(MediaError::Cancelled)
    ));
}

#[test]
fn metadata_strings_obey_the_caller_limit() {
    let limits = MediaLimits {
        max_metadata_string_bytes: 8,
        ..MediaLimits::default()
    };
    let session = MediaSession::open_file(fixture("tone-metadata.flac"), limits).unwrap();
    assert_eq!(session.info().metadata, MediaMetadata::default());
}

#[test]
fn zero_metadata_limit_is_rejected() {
    let limits = MediaLimits {
        max_metadata_string_bytes: 0,
        ..MediaLimits::default()
    };
    assert!(matches!(
        MediaSession::open_file(fixture("tone-flac.flac"), limits),
        Err(MediaError::InvalidLimits(
            "max_metadata_string_bytes must be non-zero"
        ))
    ));
}
