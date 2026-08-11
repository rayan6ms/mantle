use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use mantle_media::{
    Codec, Container, EncodedPacket, MediaError, MediaLimits, MediaSession, MemoryInput, PcmFrame,
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/media/fixtures")
        .join(name)
}

#[test]
fn probes_primary_frozen_formats() {
    let cases = [
        ("tone-pcm-s16le.wav", Container::Wave, Codec::PcmS16Le),
        ("tone-mp3.mp3", Container::Mp3, Codec::Mp3),
        ("tone-aac-lc.m4a", Container::Mp4, Codec::AacLc),
        ("tone-he-aac-v1.m4a", Container::Mp4, Codec::HeAacV1),
        ("tone-he-aac-v2.m4a", Container::Mp4, Codec::HeAacV2),
        ("tone-opus.webm", Container::WebM, Codec::Opus),
    ];
    for (name, container, codec) in cases {
        let session = MediaSession::open_file(fixture(name), MediaLimits::default()).unwrap();
        let info = session.info();
        assert_eq!(info.container, container, "{name}");
        assert_eq!(info.codec, codec, "{name}");
        assert_eq!(info.sample_rate, 48_000, "{name}");
        assert_eq!(info.channels, 2, "{name}");
        assert!(info.seekable, "{name}");
        assert!(
            info.duration
                .is_some_and(|value| value > Duration::from_secs(5))
        );
    }
}

#[test]
fn decodes_pcm_without_growing_caller_storage() {
    for name in ["tone-pcm-s16le.wav", "tone-mp3.mp3", "tone-aac-lc.m4a"] {
        let mut session = MediaSession::open_file(fixture(name), MediaLimits::default()).unwrap();
        let mut frame = PcmFrame::with_capacity(session.limits().max_pcm_samples_per_frame);
        let storage = frame.samples().as_ptr();
        let mut frames = 0_usize;
        let mut samples = 0_usize;
        let mut signal = 0.0_f64;
        while session.read_pcm(&mut frame).unwrap() {
            assert_eq!(frame.samples().as_ptr(), storage, "{name}");
            assert_eq!(frame.sample_rate(), 48_000, "{name}");
            assert_eq!(frame.channels(), 2, "{name}");
            frames += 1;
            samples += frame.samples().len();
            signal += frame
                .samples()
                .iter()
                .map(|sample| f64::from(sample.abs()))
                .sum::<f64>();
        }
        assert!(frames > 10, "{name}");
        assert!(samples > 500_000, "{name}");
        assert!(signal > 1.0, "{name}");
    }
}

#[test]
fn extracts_webm_opus_packets_without_growing_caller_storage() {
    let mut session =
        MediaSession::open_file(fixture("tone-opus.webm"), MediaLimits::default()).unwrap();
    let mut packet = EncodedPacket::with_capacity(session.limits().max_packet_bytes);
    let storage = packet.data().as_ptr();
    let mut packets = 0_usize;
    let mut bytes = 0_usize;
    while session.read_encoded(&mut packet).unwrap() {
        assert_eq!(packet.data().as_ptr(), storage);
        assert!(!packet.data().is_empty());
        assert!(packet.timestamp().is_some());
        assert!(packet.duration().is_some_and(|value| !value.is_zero()));
        packets += 1;
        bytes += packet.data().len();
    }
    assert!(packets > 250);
    assert!(bytes > 50_000);
}

#[test]
fn seeks_each_primary_local_path() {
    for name in [
        "tone-pcm-s16le.wav",
        "tone-mp3.mp3",
        "tone-aac-lc.m4a",
        "tone-he-aac-v1.m4a",
        "tone-he-aac-v2.m4a",
        "tone-opus.webm",
    ] {
        let mut session = MediaSession::open_file(fixture(name), MediaLimits::default()).unwrap();
        let result = session.seek(Duration::from_secs(3)).unwrap();
        assert_eq!(result.requested, Duration::from_secs(3));
        assert!(
            result.actual.is_some_and(|actual| {
                actual.abs_diff(result.requested) <= Duration::from_millis(100)
            }),
            "{name}: {result:?}"
        );
        if session.info().codec == Codec::Opus {
            let mut packet = EncodedPacket::with_capacity(session.limits().max_packet_bytes);
            assert!(session.read_encoded(&mut packet).unwrap());
            assert!(
                packet
                    .timestamp()
                    .is_some_and(|value| value >= Duration::from_secs(2)),
                "{name}"
            );
        } else {
            let mut frame = PcmFrame::with_capacity(session.limits().max_pcm_samples_per_frame);
            assert!(session.read_pcm(&mut frame).unwrap());
            assert!(
                frame
                    .timestamp()
                    .is_some_and(|value| value >= Duration::from_secs(2)),
                "{name}"
            );
        }
    }
}

#[test]
fn decodes_he_aac_profiles_with_bounded_reusable_storage() {
    for (name, codec, expected_samples) in [
        ("tone-he-aac-v1.m4a", Codec::HeAacV1, 585_728_usize),
        ("tone-he-aac-v2.m4a", Codec::HeAacV2, 589_824_usize),
    ] {
        let mut session = MediaSession::open_file(fixture(name), MediaLimits::default()).unwrap();
        assert_eq!(session.info().codec, codec, "{name}");
        let mut output = PcmFrame::with_capacity(session.limits().max_pcm_samples_per_frame);
        let storage = output.samples().as_ptr();
        let mut samples = 0_usize;
        while session.read_pcm(&mut output).unwrap() {
            assert_eq!(output.samples().as_ptr(), storage, "{name}");
            assert_eq!(output.sample_rate(), 48_000, "{name}");
            assert_eq!(output.channels(), 2, "{name}");
            samples += output.samples().len();
        }
        assert_eq!(samples, expected_samples, "{name}");
    }
}

#[test]
fn enforces_probe_packet_pcm_and_output_bounds() {
    let bytes = fs::read(fixture("tone-mp3.mp3")).unwrap();
    let invalid_buffer = MediaLimits {
        input_buffer_bytes: 256,
        ..MediaLimits::default()
    };
    assert!(matches!(
        MediaSession::open(
            Box::new(MemoryInput::new(bytes.clone())),
            Some("mp3"),
            invalid_buffer
        ),
        Err(MediaError::InvalidLimits(_))
    ));

    let tiny_probe = MediaLimits {
        max_probe_bytes: 8,
        ..MediaLimits::default()
    };
    assert!(matches!(
        MediaSession::open(
            Box::new(MemoryInput::new(bytes.clone())),
            Some("mp3"),
            tiny_probe
        ),
        Err(MediaError::ProbeLimitExceeded { limit: 8 })
    ));

    let packet_limited = MediaLimits {
        max_packet_bytes: 8,
        ..MediaLimits::default()
    };
    let mut session = MediaSession::open(
        Box::new(MemoryInput::new(bytes.clone())),
        Some("mp3"),
        packet_limited,
    )
    .unwrap();
    let mut frame = PcmFrame::with_capacity(packet_limited.max_pcm_samples_per_frame);
    assert!(matches!(
        session.read_pcm(&mut frame),
        Err(MediaError::PacketTooLarge { limit: 8, .. })
    ));

    let pcm_limited = MediaLimits {
        max_pcm_samples_per_frame: 8,
        ..MediaLimits::default()
    };
    let mut session = MediaSession::open(
        Box::new(MemoryInput::new(bytes.clone())),
        Some("mp3"),
        pcm_limited,
    )
    .unwrap();
    let mut frame = PcmFrame::with_capacity(8);
    assert!(matches!(
        session.read_pcm(&mut frame),
        Err(MediaError::PcmFrameTooLarge { limit: 8, .. })
    ));

    let mut session = MediaSession::open(
        Box::new(MemoryInput::new(bytes)),
        Some("mp3"),
        MediaLimits::default(),
    )
    .unwrap();
    let mut frame = PcmFrame::with_capacity(1);
    assert!(matches!(
        session.read_pcm(&mut frame),
        Err(MediaError::OutputBufferTooSmall { capacity: 1, .. })
    ));
}

#[test]
fn enforces_native_aac_configuration_memory_pcm_and_output_bounds() {
    let he_bytes = fs::read(fixture("tone-he-aac-v1.m4a")).unwrap();
    let tiny_config = MediaLimits {
        max_codec_config_bytes: 1,
        ..MediaLimits::default()
    };
    assert!(matches!(
        MediaSession::open(
            Box::new(MemoryInput::new(he_bytes.clone())),
            Some("m4a"),
            tiny_config,
        ),
        Err(MediaError::CodecConfigTooLarge { limit: 1, .. })
    ));

    let tiny_native_memory = MediaLimits {
        max_native_decoder_bytes: 1,
        ..MediaLimits::default()
    };
    assert!(matches!(
        MediaSession::open(
            Box::new(MemoryInput::new(he_bytes.clone())),
            Some("m4a"),
            tiny_native_memory,
        ),
        Err(MediaError::Backend {
            operation: "decoder creation",
            ..
        })
    ));

    let tiny_native_pcm = MediaLimits {
        max_pcm_samples_per_frame: 8,
        ..MediaLimits::default()
    };
    let mut session = MediaSession::open(
        Box::new(MemoryInput::new(he_bytes.clone())),
        Some("m4a"),
        tiny_native_pcm,
    )
    .unwrap();
    let mut frame = PcmFrame::with_capacity(8);
    assert!(matches!(
        session.read_pcm(&mut frame),
        Err(MediaError::PcmFrameTooLarge { limit: 8, .. })
    ));

    let mut session = MediaSession::open(
        Box::new(MemoryInput::new(he_bytes)),
        Some("m4a"),
        MediaLimits::default(),
    )
    .unwrap();
    let mut frame = PcmFrame::with_capacity(1);
    assert!(matches!(
        session.read_pcm(&mut frame),
        Err(MediaError::OutputBufferTooSmall { capacity: 1, .. })
    ));
}

#[test]
fn malformed_inputs_fail_without_producing_unbounded_output() {
    for bytes in [Vec::new(), b"not media".to_vec(), vec![0xff; 4_096]] {
        let result = MediaSession::open(
            Box::new(MemoryInput::new(bytes)),
            None,
            MediaLimits {
                max_probe_bytes: 4_096,
                input_buffer_bytes: 64 * 1024,
                ..MediaLimits::default()
            },
        );
        assert!(result.is_err());
    }
}

#[test]
fn corrupted_he_aac_payloads_terminate_within_packet_and_output_bounds() {
    for name in ["tone-he-aac-v1.m4a", "tone-he-aac-v2.m4a"] {
        let mut bytes = fs::read(fixture(name)).unwrap();
        corrupt_first_mdat_payload(&mut bytes);
        let limits = MediaLimits::default();
        let Ok(mut session) =
            MediaSession::open(Box::new(MemoryInput::new(bytes)), Some("m4a"), limits)
        else {
            continue;
        };
        let mut frame = PcmFrame::with_capacity(limits.max_pcm_samples_per_frame);
        let mut terminated = false;
        for _ in 0..400 {
            if matches!(session.read_pcm(&mut frame), Ok(true)) {
                assert!(frame.samples().len() <= limits.max_pcm_samples_per_frame);
            } else {
                terminated = true;
                break;
            }
        }
        assert!(terminated, "{name}");
    }
}

fn corrupt_first_mdat_payload(bytes: &mut [u8]) {
    let mut offset = 0_usize;
    while offset.checked_add(8).is_some_and(|end| end <= bytes.len()) {
        let size = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        if size < 8 || offset.checked_add(size).is_none_or(|end| end > bytes.len()) {
            break;
        }
        if &bytes[offset + 4..offset + 8] == b"mdat" {
            let payload_start = offset + 8;
            let payload_end = (payload_start + 64).min(offset + size);
            for byte in &mut bytes[payload_start..payload_end] {
                *byte ^= 0xff;
            }
            return;
        }
        offset += size;
    }
    panic!("fixture has no bounded top-level mdat payload");
}
