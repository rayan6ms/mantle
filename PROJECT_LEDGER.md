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

# Companion URL provenance (2026-10-02)

Installed Invidious Companion revision `bb3b37ff40c69475e45785d16eb7da8876b80089`
deciphers every media URL and removes signatureCipher before returning it
(`src/lib/helpers/youtubePlayerHandling.ts`). Oracle's authenticated response
contained direct signed Opus URLs with `n` already present and no cipher.
Mantle incorrectly treated that value as a fresh Web challenge, reproducing
InvalidResponse before media I/O and causing an avoidable fallback. An explicit
private provenance flag now preserves URLs only from the configured authenticated
Companion endpoint. Raw InnerTube/watch responses retain normal deciphering;
unresolved Companion signatures are rejected and fall through to another client.
The regression failed before the fix; it now returns the exact URL without any
player-script request. Existing raw-cipher and media handoff tests still pass.

# Metadata-length staging (2026-10-02)

After the Companion correction, Oracle spent 10.455 seconds opening a 4.17 MB
Opus object. A bounded same-route curl probe attributed 1.775 seconds to its
preliminary 256 KiB request. Finite YouTube metadata already supplies the size.
`HttpRangeOptions::expected_source_bytes` now validates that size against the
server's Content-Range and lets stageable objects use one full range immediately.
Missing lengths retain the probe, oversized objects retain windowed streaming,
and the existing anonymous file, 64 KiB copy buffer, deadlines and recovery remain.

The regression failed before optimization with two requests / 70.886 ms. An
isolated 4 MiB, 20 ms/request comparison measured two requests / 71.390 ms versus
one / 50.263 ms, with exact staged bytes available after the origin shuts down.
Tests also cover stale/invalid metadata, exact-offset truncated-body recovery,
changed validators, cancellation, oversized streaming and real Opus/AAC replay.
No encoder, bitrate, pacing or read-ahead setting changes. Live qualification is
recorded by Raydio's startup audit; controlled latency is not an Oracle promise.
