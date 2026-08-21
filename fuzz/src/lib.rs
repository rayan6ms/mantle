use std::time::Duration;

use mantle_media::{
    Codec, Container, EncodedPacket, MediaLimits, MediaSession, MemoryInput, PcmFrame,
};

const MAX_FUZZ_INPUT_BYTES: usize = 256 * 1024;
const MAX_OUTPUT_CALLS: usize = 16;

#[derive(Clone, Copy)]
pub enum LocalBoundary {
    Wave,
    Matroska,
    Mp4,
    Flac,
    Ogg,
    Mp3,
    Adts,
}

impl LocalBoundary {
    fn extension(self) -> &'static str {
        match self {
            Self::Wave => "wav",
            Self::Matroska => "mkv",
            Self::Mp4 => "m4a",
            Self::Flac => "flac",
            Self::Ogg => "ogg",
            Self::Mp3 => "mp3",
            Self::Adts => "aac",
        }
    }

    fn accepts(self, container: Container) -> bool {
        match self {
            Self::Wave => container == Container::Wave,
            Self::Matroska => matches!(container, Container::Matroska | Container::WebM),
            Self::Mp4 => container == Container::Mp4,
            Self::Flac => container == Container::Flac,
            Self::Ogg => container == Container::Ogg,
            Self::Mp3 => container == Container::Mp3,
            Self::Adts => container == Container::Adts,
        }
    }
}

pub fn exercise_local_boundary(data: &[u8], boundary: LocalBoundary) {
    if data.len() > MAX_FUZZ_INPUT_BYTES {
        return;
    }
    let limits = MediaLimits {
        max_probe_bytes: MAX_FUZZ_INPUT_BYTES as u64,
        max_metadata_string_bytes: 32 * 1024,
        max_codec_probe_bytes: MAX_FUZZ_INPUT_BYTES,
        max_packet_bytes: MAX_FUZZ_INPUT_BYTES,
        max_pcm_samples_per_frame: 64 * 1024,
        max_consecutive_decode_errors: 4,
        ..MediaLimits::default()
    };
    let Ok(mut session) = MediaSession::open(
        Box::new(MemoryInput::new(data.to_vec())),
        Some(boundary.extension()),
        limits,
    ) else {
        return;
    };
    if !boundary.accepts(session.info().container) {
        return;
    }

    let seekable = session.info().seekable;
    exercise_output(&mut session, MAX_OUTPUT_CALLS / 2);
    if seekable {
        let millis = data
            .get(..4)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u32::from_le_bytes)
            .map_or(0, |value| value % 10_000);
        if session
            .seek(Duration::from_millis(u64::from(millis)))
            .is_ok()
        {
            exercise_output(&mut session, MAX_OUTPUT_CALLS / 2);
        }
    }
}

fn exercise_output(session: &mut MediaSession, calls: usize) {
    if session.info().codec == Codec::Opus {
        let mut output = EncodedPacket::with_capacity(session.limits().max_packet_bytes);
        for _ in 0..calls {
            if !matches!(session.read_encoded(&mut output), Ok(true)) {
                break;
            }
        }
    } else {
        let mut output = PcmFrame::with_capacity(session.limits().max_pcm_samples_per_frame);
        for _ in 0..calls {
            if !matches!(session.read_pcm(&mut output), Ok(true)) {
                break;
            }
        }
    }
}
