# ADR-0030: Keep playback transitions and routed sockets inside Mantle

## Status

Accepted on 2026-09-02.

## Context

Crust, a native Lavalink-style consumer, needs to seek and replace filters while a selected
YouTube Opus object is playing. It also needs Lavalink RoutePlanner-style outbound address
selection for YouTube control requests and media connections. Before this decision,
`YoutubePlaybackSession` exposed only frame delivery, and Mantle's HTTP policy did not select a
local source address.

Decoding Opus or rebuilding filter/encoder state in Crust would duplicate Mantle-owned media
behavior. Selecting an address without binding the socket, or pooling only by remote origin, would
allow a connection selected for route A to escape through or be reused after selecting route B.

## Evidence

- Mantle's pinned `opus-head-sys` 0.3.1/libopus revision already exposes the decoder API. A narrow
  safe decoder adapter requires no new codec dependency.
- ureq 3.4.0 exposes an unversioned connector boundary, but its private idle-pool key is fixed to
  scheme, authority, and proxy. Mantle cannot safely add its opaque route identity to that key.
- Rust's standard networking API does not provide portable bind-before-connect sockets.
  `socket2` 0.6.5 is current as of this decision, supports Rust 1.70+, is MIT OR Apache-2.0, is
  actively maintained at `rust-lang/socket2`, and supplies the required portable socket calls.
  Cargo Vet accepts its publisher under the existing Mozilla and Bytecode Alliance trust imports;
  Cargo Deny accepts its license, source, and graph.

Regression evidence is in `mantle-opus` decoder tests,
`phase12_youtube::finite_opus_session_seeks_filters_and_returns_safely_to_passthrough`, and
`phase12_remote_http::routed_client_binds_each_selected_ip_and_reports_connection_outcomes`.

## Decision

`YoutubePlaybackSession` owns seek and live filter replacement. Opus input uses direct packet
delivery only with an empty filter chain. An active chain selects Mantle-owned Opus decode →
canonical PCM filter → Opus encode. Seek resets demux/decoder, filter, resampler where applicable,
encoder, partial-frame, timestamp, and passthrough-transition state. Factory installation remains
atomic; removing filters resets state before direct delivery resumes.

Mantle exposes `OutboundRoutePolicy`, a credential-safe origin context, an opaque route identity,
the local IP to bind, and connection outcome reporting. The same policy can be installed on
`YoutubeAudioSourceManager` control requests and passed to finite or live playback openings.
Finite range connections, HLS manifests/reloads, and HLS segments all use the routed connector.

Because ureq's pool key cannot carry route identity, every routed agent has zero idle connections
and its routed transport reports itself closed after the request. This is equivalent to an isolated
one-connection pool per route selection and makes cross-route reuse impossible. Reusing direct
connections is preferable only if a future HTTP backend exposes a pool key that includes the exact
route identity.

`socket2` remains private to `mantle-media`, uses no optional features, and is pinned exactly at
0.6.5.

## Consequences

- Crust does not own codec, filter-transition, or socket-binding implementation.
- Routed HTTP pays a connection/TLS setup cost per request. Correct RoutePlanner semantics take
  priority; normal non-routed HTTP retains ureq pooling.
- The policy receives scheme and authority only, never signed paths, query parameters, headers,
  credentials, or response bodies.
- The current outcome contract reports connection establishment, timeout, and transport failure.
  Higher-level HTTP status classification remains available through Mantle's existing source
  errors and responses.

## Revisit triggers

- A maintained HTTP backend supports a caller-defined route-aware pool key.
- Routed request volume shows connection setup to be a measured bottleneck.
- `socket2` becomes unmaintained, changes licensing, or fails a supported target.
- Crust demonstrates a required route outcome that cannot be derived from the current connection
  and source error contracts.
