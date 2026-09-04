# Crust integration contract

Mantle owns the playback, processing, and routed-network boundaries Crust needs.

## Integrated YouTube playback

For deterministic offline benchmarks, open content-addressed fixtures with
`MediaSession::open_file(...)`, then transfer ownership with
`YoutubePlaybackSession::from_media_session(session, expected_kind)`. This path validates the
probed container and codec against `YoutubePlaybackFormatKind` exactly like selected finite
YouTube playback. Because YouTube's format enum has no MP3 or FLAC variants, use
`YoutubePlaybackSession::from_probed_media_session(session)` for already probed local MP3 and FLAC
fixtures. Both constructors enter the same Mantle-owned finite playback implementation; neither
performs discovery or network access.

The offline session produces complete 20 ms Opus frames through `read_frame`. Compatible 48 kHz
stereo Opus begins in `OpusPassthrough`; MP3, AAC, FLAC, and other decoded inputs begin and remain
in `Transcode`. The existing `seek`, `set_filter_factory`, `mode`, and `source_media_position`
operations apply unchanged. Crust must not decode, resample, assemble, filter, or encode around
this seam.

Use `YoutubePlaybackSession::seek(Duration)` to seek. Use
`YoutubePlaybackSession::set_filter_factory(Some(factory))` to install or replace filters and pass
`None` to remove them. `mode()` reports `Transcode` while an effective filter chain is active and
returns to `OpusPassthrough` when an Opus input has no filters. Live HLS is always transcoded and
exposes the same `set_filter_factory` operation on `YoutubeLivePlaybackSession`.

Crust retains its `PcmFilterFactory`; Mantle creates and owns the per-session filter instances.
Mantle owns Opus decoding, canonical PCM filtering, encoding, partial-frame state, and reset after
seek or replacement. A failed factory build preserves the previous chain.

## Streaming PCM processing

Implement `StreamingPcmProcessor` for the lavadsp timescale state and add it at the correct point
in `PcmFilterFactory::build` with `FilterChainBuilder::push_streaming`. Counts in
`StreamingPcmProgress` are interleaved sample counts and must be stereo-channel aligned.
`process(input, output)` may consume part or all of `input` and produce zero, part, or all of the
provided output slice. It may be called with empty input to drain immediately available output.
The processor must keep any algorithm latency or surplus in its own explicitly bounded storage;
when that bound is insufficient it returns `AudioFrameError::StreamingProcessorCapacityExceeded`.

Call `builder.push(...)` for fixed `PcmFilter` stages. Fixed stages pushed before
`push_streaming(...)` run before timescale; stages pushed afterward run on each produced block.
The chain permits one streaming stage and at most 32 total nodes. This preserves Lavalink's fixed
pre-filters → timescale → fixed post-filters order without exposing playback orchestration to
Crust. Existing fixed-only factories and `FilterPipeline::process` remain supported.

`StreamingPcmProcessor::finish` drains terminal latency. Mantle calls it repeatedly after source
EOF, retains surplus in fixed-capacity storage, assembles exact 1,920-sample 48 kHz stereo blocks,
and only then zero-pads at most one final block. `reset` must discard every processor-owned sample;
Mantle invokes it on seek, successful factory replacement/removal, and session destruction. New
sessions own fresh processors, so track replacement cannot inherit prior state.

Install the factory through `YoutubePlaybackSession::set_filter_factory(Some(factory))` (or the
live-session equivalent), not by driving `FilterPipeline` from Crust. A failed replacement leaves
the old graph and `mode()` unchanged. Passing `None` removes processing; finite compatible Opus
input returns to `YoutubePlaybackMode::OpusPassthrough`, while Mantle continues to own the safe
decode/process/encode transition.

Each emitted PCM/Opus frame has a paced timestamp of `first_input_timestamp + output_index * 20ms`.
`YoutubePlaybackSession::source_media_position` and
`YoutubeLivePlaybackSession::source_media_position` instead report
`first_input_timestamp + consumed_source_frames / 48_000`. Thus output time and source-media time
remain monotonic but intentionally diverge at non-unit speed. If the first input timestamp is
absent, both processor-derived clocks remain absent until reset. Seek resets both clocks, and the
next decoded block establishes the new base at the actual media position.

The lower-level public pull primitives used by Mantle's media crate are
`FilterPipeline::{replacement, commit_replacement, submit_input, read_output, finish_input,
source_position, has_streaming_processor}` and `StreamingPcmPoll::{Frame, NeedInput, Finished}`.
Crust should not call these for YouTube playback; the session APIs already pull more source input
when a processor initially emits nothing and drain all output before EOF.

## Routed HTTP

Implement `OutboundRoutePolicy` in Crust. `select_route` receives only the destination scheme and
authority and returns `OutboundRoute { local_ip, identity }`. `report_outcome` receives connection
establishment or a credential-safe connection failure.

Create the YouTube manager with
`YoutubeAudioSourceManager::with_route_policy(options, authentication, policy.clone())` so all
control requests are routed. Open finite selected media with
`open_selected_playback_routed(..., policy.clone())`, or live HLS with
`open_selected_live_playback_routed(..., policy.clone())`. Pass the same shared RoutePlanner state
at both boundaries.

Mantle binds the selected local IP before connecting. Routed requests do not retain idle pooled
connections, so selecting B cannot reuse a socket previously bound to A. Ordinary non-routed HTTP
retains pooling.

Crust should map each route entry to a stable `u64` identity, reject an address family that cannot
reach the destination, and combine the connection outcome with Mantle's existing source
error/status classification when updating RoutePlanner health.
