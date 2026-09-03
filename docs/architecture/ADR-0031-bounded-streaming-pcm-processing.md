# ADR-0031: Extend the owned PCM chain with one bounded streaming stage

## Status

Accepted on 2026-09-02.

## Context

Mantle's `PcmFilter::process(&mut PcmFrame)` contract is intentionally allocation-stable but only
models one-input-block to one-output-block transforms. Lavalink 4.2.2's lavadsp 0.7.8 timescale is
streaming: it can retain latency, produce a variable number of samples, change paced duration, and
emit a terminal tail. Keeping the old 1,920-to-1,920 contract would reproduce pitch changes but not
speed, duration, EOF, position, or track-end semantics. Moving decode, encode, source pulling, or
output pacing into Crust would duplicate Mantle's playback pipeline.

## Decision

Extend `FilterPipeline` with one `StreamingPcmProcessor` between bounded fixed-filter chains.
Factories insert it with `FilterChainBuilder::push_streaming`; fixed filters retain their insertion
order on either side. One streaming stage is sufficient for Lavalink's core chain and avoids a
general graph scheduler.

The processor receives canonical interleaved 48 kHz stereo slices and reports explicit consumed
and produced sample counts. Mantle validates counts and channel alignment, rejects a no-progress
call with pending input, and supplies at most one canonical frame of output capacity per call. The
processor owns explicitly bounded algorithm state and reports
`StreamingProcessorCapacityExceeded` when it cannot proceed within that bound.

Mantle owns fixed-capacity pending-input, processor-output, surplus, and final-frame storage. Its
pull loop requests more source blocks when output is incomplete, repeatedly drains available
processor output, and encodes only exact 1,920-sample blocks. At EOF it repeatedly calls `finish`
until drained, then emits at most one zero-padded frame. Processor or post-filter errors clear the
prepared output scratch so retry cannot expose zero-filled scratch as valid audio.

Output timestamps use paced time from the first input timestamp in 20 ms increments. The separate
source position advances from the same base only by processor-reported consumed input frames. If
the first timestamp is absent, both clocks remain absent for that stream. Reset clears both clocks.

Replacement is constructed before the active graph changes. Playback resets its owned codec and
transition state before committing the already-built graph; a factory failure therefore preserves
the prior graph and mode. Assignment drops and resets the old graph. Seek resets the active graph,
removal installs an empty graph, and session destruction resets before releasing all processors.

## Evidence

`mantle-audio` regressions cover 2.0× and 0.5× paced duration, monotonic dual clocks, missing
timestamp policy, terminal flush before padding, retained surplus, fixed-stage order, reset/drop,
atomic factory failure, deterministic capacity failure, and allocation-free steady-state assembly.
`mantle-media` regressions cover the real PCM transcoder's changed frame counts and the finite Opus
path's multi-input pull, seek reset against a clean sought session, replacement/reset, immediate
passthrough restoration, and shutdown cleanup.

## Consequences

- Crust implements only the bounded lavadsp processor and factory ordering; Mantle retains source,
  codec, framing, timing, EOF, and transition ownership.
- Streaming processors are currently restricted to canonical 48 kHz stereo and one stage per
  chain. A second stage requires evidence that Lavalink-compatible ordering cannot be expressed
  with fixed pre/post filters.
- A processor that retains data must size and enforce its own storage bound. Mantle never creates
  an unbounded queue or allocates per output frame.

## Revisit triggers

- A required Lavalink filter graph contains more than one variable-rate stage.
- A measured processor requires larger transfer blocks to make bounded forward progress.
- A new playback path cannot use the shared Mantle-owned pull and flush loop.
