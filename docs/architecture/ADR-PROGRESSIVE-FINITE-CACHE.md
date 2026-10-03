# Progressive finite compressed cache (2026-10-02)

Raydio's complete staging prevents mid-track source stalls, but its 4.17 MB
Oracle input spent 4.716 seconds downloading after response setup in the latest
baseline. Playback could not begin until that completed.

Add an opt-in progressive prefix to the existing finite anonymous-file cache.
Library defaults remain zero (complete staging). Prefixes are 16 KiB–1 MiB and
must fit the existing ceiling, which remains at most 64 MiB. Larger/unknown-size
objects retain their previous streaming path. No codec change.

One owned downloader copies with a 64 KiB buffer and publishes written bytes
with release/acquire ordering. There are at most eight downloaders globally;
each has a 256 KiB stack. Readers have independent offsets and use `read_at`,
never sparse holes. Reads/seeks wait for actual bytes. Last-owner drop cancels
and joins the worker, and completion releases its concurrency slot. Existing
range identity, ETag, length, destination/proxy policy and retry checks remain.
The complete download deadline applies across recovery. Body waits poll cancellation
every 100 ms without reopening the response; connect/TLS/reconnect waits retain
their existing bounded timeout. Complete loop playback reuses the same cache.

A real WebM regression initially failed despite early HTTP open: the optional
EBML metadata scan read a fixed 128 KiB window, and the demuxer sought tail cues.
Inspection now uses the already cached prefix. During a growing WebM probe,
optional tail metadata seeks are deferred. The first explicit seek finishes the
same bounded cache and opens an indexed reader; it replaces the active session
only after seeking succeeds. Filters/encoder reset follow the ordinary seek
path. Required container metadata, including a trailing MP4 `moov`, can still
require later bytes. The feature promises early availability when the container
permits it, not a universal startup deadline.

Evidence: the controlled 128 KiB source with a gated 400 ms remainder opened in
404.266 ms with complete staging versus 2.154 ms with a prefix, with identical
bytes. The real WebM fixture delivers an identical first Opus frame while its
remainder is held, then preserves every packet/timestamp, rejected-seek state
and backward seek. Other regressions cover stalled-body cancellation/drop,
truncation, inactivity/total deadlines, exact-offset recovery, changed validators,
independent cached replay and real AAC/Opus repeat with fresh cancellation.
Live startup/receiver qualification belongs to Raydio's progressive audit.

Tradeoff: if download throughput falls below consumption after the initial
prefix, playback may wait. Error/cancellation is explicit rather than synthesized
silence or clean EOF. Deployment may select prefix zero to restore complete
staging. Revisit the prefix/cache bound if live evidence shows waits after
playback starts or material memory/CPU growth. Do not compensate by changing
audio quality, trimming media, or sending catch-up bursts.

The proxy audit reproduced another blocker: ureq 3.4.x's SOCKS helper uses a
scoped thread around unbounded socket I/O, so receiving a timeout still waits
for that thread. A mock proxy closed after 700 ms; the old 200 ms connection
deadline returned after 701.061 ms. A bounded connector now negotiates SOCKS on
ureq's existing TCP transport, preserving its pool and Rustls wrapping. Every
setup step consumes the original remaining deadline; reads poll cancellation
without a helper thread. SOCKS4/4A/5, remote DNS, IPv4/IPv6, password negotiation,
fragmented replies and repeated pooled responses have local regressions.
The unnecessary upstream SOCKS feature/dependency and Windows import-library
closure are removed. CONNECT proxy transport is unchanged. Proxy setup errors
are constant credential-free messages. Rustls 0.23.45 also closes the advisory
RUSTSEC-2026-0285 found in the release audit; the locked WebPKI patch is 0.103.15.

## Progressive transfer deadlines (2026-10-03)

Production progressive downloads that advanced about 32 KiB/s were cut off by
the ordinary 30-second request deadline. Three successive objects cached only
about 934–950 KiB of 12.6–14.1 MB and failed when playback exhausted those bytes.
A paced local origin reproduced a healthy transfer failing at 49,152 bytes.

Progressive caching now has a separate total download budget, default 30 minutes
and configurable from nonzero through one hour. One absolute deadline starts
when the eligible full response opens (or the worker starts after a length probe)
and survives body recovery. Short DNS/send/header budgets and the ordinary
request timeout as a body-inactivity limit prevent hung setup or stalled reads.
Cancellation continues to poll every 100 ms. Ordinary ranges and complete
staging keep their previous short request policy. Bounds and encoded bytes do
not change; there is no transcoding, dropping, quality reduction or new worker.

Ureq 3.4.0 also applied completed-phase deadlines during body reads, so extending
only our body budget still failed the regression. Pin 3.4.2, which includes
3.4.1's phase-budget fix. Its protocol dependency locks to 0.6.4. Reviewed source
deltas cover timeout start/expiry, pool isolation and unread input, TLS handshake
deadlines, address fallback, content-length/list parsing, invalid status codes,
redirect authorities and bounded array helpers. Both packages remain pure Rust
with no build script or new unsafe implementation. Local delta audits are recorded
in supply-chain/audits.toml; advisory/license/vet checks remain release gates.

The healthy 128 KiB origin delivers 16 KiB every 100 ms with a 250 ms short
timeout: it now completes in roughly 0.7 seconds with exact bytes. Regressions
also prove continuing progress and ETag-validated recovery cannot reset the total
deadline, an inactive body fails promptly, and complete staging retains its
short deadline. Existing cancellation, truncation, validator, range and replay
tests remain required. Live Oracle qualification is recorded by Raydio.
