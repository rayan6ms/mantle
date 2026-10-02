# Filter continuity audit resolved (2026-09-10)

Replacing a filter graph no longer resets decoded/resampled PCM, encoder state,
source clocks, or true EOF. A single bounded pending graph drains already
accepted streaming input and latency, retaining partial frames across the
transition. Returning to Opus passthrough waits for an empty boundary; reentering
processing resets stale codec history and advances the source clock past packets
that bypassed decoding. A finite unexpected NeedInput is a typed error, not a panic.

Regression evidence: repeated identity changes preserve every encoded byte and
timestamp for FLAC, MP3, AAC 48 kHz, resampled AAC 24 kHz and mono PCM 8 kHz;
three post-EOF changes remain EOF. Rate transitions retain partial samples
without inserting an intermediate padded frame. Repeated Opus bypass changes
preserve source packet order/clocks. Audio 34, media unit 23 (one environment
exclusion), YouTube 39 (three exclusions), allocation 3 tests pass; scoped Clippy
passes. The existing allocation bounds remain unchanged. No live receiver or
cloud performance improvement is inferred from these deterministic tests.

# Active integration findings

Raydio isolated source pull on Oracle reproduced a 3.94-second request stall.
Independent libopus decoding found no malformed durations, mid-song silence,
clipping, or nonfinite PCM. Two socket traces did not reproduce that long wait.
A separate matched compressed-input experiment measured six reads over 20 ms
(max 244.864 ms) when streamed, versus zero (max 0.957 ms) after anonymous-file
staging. Startup increased from 0.603 to 2.286 seconds for 3,433,755 bytes.
This justifies an opt-in bounded staging implementation; larger/live sources
remain streaming and the bot still requires downstream and endurance evidence.
Evidence is in Raydio evidence/ORACLE-ENDURANCE.md and the source audit JSON.

Implementation: opt-in Unix anonymous-file staging, default disabled, 64 MiB
ceiling and one temporary 64 KiB copy buffer. Media HTTP (16), media unit
(20 plus anonymous-file lifecycle), YouTube (38), remote HTTP (10), formatting,
Clippy, cargo audit, cargo deny, and cargo vet (171 audited) pass. Existing
live/environment test exclusions remain unchanged. No dependency changes.

Replay must rebuild the media pipeline from the retained compressed file:
WebM seek after EOF can fail in the demuxer, and the PCM transcoder releases
its consumed media session. StagedPlaybackInput consumes the old playback
session and retains one anonymous file handle, then opens fresh decoder/DSP
state with independent cancellation. Exact Opus and AAC packets repeat after
both source servers are dropped; old cancellation cannot poison the replay.
YouTube suite now passes 39 tests with the same 3 environment/live exclusions.

# Configured source-proxy policy fix (2026-10-01)

`HttpNetworkAccess::PublicInternetOnly` was also applied to the loopback address
of an explicitly configured SOCKS source proxy. The proxy handshake was therefore
classified as `DestinationDenied`, so YouTube search and playlist control requests
failed while Companion player requests (which bypass the proxy) still worked.
The resolver now permits only the configured proxy authority as a private exception;
directly resolved destination addresses remain subject to the public-address filter.
Regression coverage verifies the authority match. Oracle probes now load search and
playlist inputs successfully through the home egress.

# Staged startup round trips (2026-10-02)

Raydio took 26.2 seconds from command receipt to track start while encrypted
voice became ready in 2.4 seconds. Finite staging unnecessarily fetched every
256 KiB window separately. A controlled 4 MiB fixture with 20 ms fixed request
latency measured 16 requests / 372.728 ms before, versus 2 requests / 72.658 ms
after. Once the first bounded probe establishes a stageable object's length,
the next range consumes its remainder through the existing 64 KiB copy buffer.
No new dependency, heap-sized media buffer, or streaming-default change.
Interrupted-body recovery retains the original response bounds; changed object
validators still fail. Exact bytes, cancellation, oversized fallback and replay
tests pass. These simulated timings do not establish the live Oracle gain.
The complete media suite, all-target media Clippy with warnings denied, and
formatting of changed Rust files pass. Existing proxy/Companion documentation
lint findings were corrected, with checked watch-document size conversions and
an explicitly non-exhaustive credential-redacting Debug implementation.
