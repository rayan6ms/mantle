# Crust integration contract

Mantle now owns the two capabilities Crust needed before its next phase.

## Integrated YouTube playback

Use `YoutubePlaybackSession::seek(Duration)` to seek. Use
`YoutubePlaybackSession::set_filter_factory(Some(factory))` to install or replace filters and pass
`None` to remove them. `mode()` reports `Transcode` while an effective filter chain is active and
returns to `OpusPassthrough` when an Opus input has no filters. Live HLS is always transcoded and
exposes the same `set_filter_factory` operation on `YoutubeLivePlaybackSession`.

Crust retains its `PcmFilterFactory`; Mantle creates and owns the per-session filter instances.
Mantle owns Opus decoding, canonical PCM filtering, encoding, partial-frame state, and reset after
seek or replacement. A failed factory build preserves the previous chain.

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
