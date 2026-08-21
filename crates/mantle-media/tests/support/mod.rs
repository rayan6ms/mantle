use std::path::Path;
use std::time::Duration;

use mantle_media::{
    Codec, Container, MediaCancellation, MediaError, MediaLimits, MediaMetadata, MediaSession,
    PcmFrame,
};

pub struct PcmConformanceCase<'a> {
    pub path: &'a Path,
    pub container: Container,
    pub codec: Codec,
    pub metadata: MediaMetadata,
    pub seek_to: Duration,
}

pub fn assert_pcm_conformance(case: &PcmConformanceCase<'_>) {
    let mut session = MediaSession::open_file(case.path, MediaLimits::default())
        .expect("conformance fixture should open");
    assert_eq!(session.info().container, case.container);
    assert_eq!(session.info().codec, case.codec);
    assert_eq!(session.info().metadata, case.metadata);

    let seek = session.seek(case.seek_to).expect("fixture should seek");
    assert_eq!(seek.requested, case.seek_to);
    let actual = seek
        .actual
        .expect("seek should report its actual packet time");
    assert!(
        actual.abs_diff(case.seek_to) <= Duration::from_millis(100),
        "seek to {:?} resolved to {actual:?}",
        case.seek_to
    );

    let mut output = PcmFrame::with_capacity(session.limits().max_pcm_samples_per_frame);
    let mut previous_timestamp = None;
    let mut frames = 0_usize;
    while session
        .read_pcm(&mut output)
        .expect("fixture should decode")
    {
        let timestamp = output.timestamp().expect("decoded frame should be timed");
        if previous_timestamp.is_none() {
            assert!(timestamp >= actual);
        }
        assert!(previous_timestamp.is_none_or(|previous| timestamp >= previous));
        previous_timestamp = Some(timestamp);
        frames += 1;
    }
    assert!(frames > 0);
    assert!(!session.read_pcm(&mut output).expect("EOF should be stable"));

    let cancellation = MediaCancellation::new();
    let mut session = MediaSession::open_file_with_cancellation(
        case.path,
        MediaLimits::default(),
        cancellation.clone(),
    )
    .expect("cancellation fixture should open");
    assert!(
        session
            .read_pcm(&mut output)
            .expect("one frame should decode before cancellation")
    );
    cancellation.cancel();
    assert!(matches!(
        session.read_pcm(&mut output),
        Err(MediaError::Cancelled)
    ));
}
