use mantle_media::{MediaError, MediaLimits, MediaSession, MemoryInput};

const HOSTED_FAILURE_SAMPLE_SIZE: u32 = 3_959_423_310;

fn atom(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
    let size = u32::try_from(payload.len() + 8).expect("small deterministic fixture");
    let mut bytes = Vec::with_capacity(payload.len() + 8);
    bytes.extend_from_slice(&size.to_be_bytes());
    bytes.extend_from_slice(&kind);
    bytes.extend_from_slice(payload);
    bytes
}

fn nested_sample_size_box(default_size: u32, entries: &[u32]) -> Vec<u8> {
    let mut payload = vec![0; 4];
    payload.extend_from_slice(&default_size.to_be_bytes());
    let sample_count = if default_size == 0 {
        u32::try_from(entries.len()).expect("small deterministic fixture")
    } else {
        1
    };
    payload.extend_from_slice(&sample_count.to_be_bytes());
    if default_size == 0 {
        for entry in entries {
            payload.extend_from_slice(&entry.to_be_bytes());
        }
    }
    let stsz = atom(*b"stsz", &payload);
    let stbl = atom(*b"stbl", &stsz);
    let minf = atom(*b"minf", &stbl);
    let mdia = atom(*b"mdia", &minf);
    let trak = atom(*b"trak", &mdia);
    atom(*b"moov", &trak)
}

fn assert_packet_preflight_rejects(bytes: Vec<u8>) {
    let limits = MediaLimits {
        max_probe_bytes: 64 * 1024,
        max_packet_bytes: 256 * 1024,
        ..MediaLimits::default()
    };
    let result = MediaSession::open(Box::new(MemoryInput::new(bytes)), Some("m4a"), limits);
    assert!(matches!(
        result,
        Err(MediaError::Backend {
            operation: "container preflight",
            message,
        }) if message == format!(
            "MP4 sample declares {HOSTED_FAILURE_SAMPLE_SIZE} bytes; limit is {}",
            limits.max_packet_bytes
        )
    ));
}

#[test]
fn rejects_oversized_constant_sample_size_before_backend_probe() {
    assert_packet_preflight_rejects(nested_sample_size_box(HOSTED_FAILURE_SAMPLE_SIZE, &[]));
}

#[test]
fn rejects_oversized_per_sample_size_before_backend_probe() {
    assert_packet_preflight_rejects(nested_sample_size_box(0, &[HOSTED_FAILURE_SAMPLE_SIZE]));
}
