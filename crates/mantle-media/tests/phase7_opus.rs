use mantle_audio::{
    EncodedFrameSlot, OpusModeTransition, OpusPassthrough, OpusPipelineMode, PcmFormat,
};
use mantle_media::{EncodedPacket, MediaLimits, MediaSession};

#[test]
fn routes_real_compatible_webm_packets_without_growing_output_storage() {
    let mut session = MediaSession::open_file(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/media/fixtures/tone-opus.webm"
        ),
        MediaLimits::default(),
    )
    .unwrap();
    let info = session.info();
    let format = PcmFormat::new(info.sample_rate, info.channels).unwrap();
    let mut router = OpusPassthrough::new(format);
    let mut packet = EncodedPacket::with_capacity(session.limits().max_packet_bytes);
    let mut output = EncodedFrameSlot::new();
    let output_storage = output.data().as_ptr();
    let mut packets = 0_usize;

    while session.read_encoded(&mut packet).unwrap() {
        let route = router
            .route_packet(packet.data(), packet.timestamp(), &mut output)
            .unwrap();
        assert_eq!(route.mode, OpusPipelineMode::Passthrough);
        assert_eq!(output.data(), packet.data());
        assert_eq!(output.timestamp(), packet.timestamp());
        assert_eq!(output.data().as_ptr(), output_storage);
        if packets == 0 {
            assert_eq!(
                route.transition,
                Some(OpusModeTransition::EnabledPassthrough)
            );
        } else {
            assert_eq!(route.transition, None);
        }
        packets += 1;
    }

    assert!(packets > 250);
}
