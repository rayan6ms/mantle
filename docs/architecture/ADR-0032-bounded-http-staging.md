# ADR-0032: Optional bounded compressed HTTP staging

Status: accepted for consumer evaluation; live audio qualification remains external.

Raydio's isolated Oracle source pull waited 3.94 seconds for one frame. The
output independently decoded without mid-track silence or invalid durations.
A compressed-input comparison on the same free VM reduced max read time from
244.864 ms to 0.957 ms (six reads over 20 ms to zero) by staging a 3,433,755-byte
object. Startup increased from 0.603 to 2.286 seconds. See Raydio's retained
source and HTTP staging evidence; this does not qualify Discord delivery.

HttpRangeOptions now has an opt-in staging byte ceiling (default zero, max
64 MiB). Eligible finite objects are read through the same bounded, validated,
routed/cancellable HTTP path into one anonymous file before playback opens.
Larger objects remain streaming. One 64 KiB heap buffer is dropped after copy;
the operating system may retain reclaimable file cache. This is not zero-cost
memory and no total-process memory improvement is claimed.

Unix exclusive create_new with mode 0600 rejects existing names and symlinks.
The pathname is unlinked before any source bytes are written, and the File
owner releases storage on failure, close, or process exit. The name is not a
security boundary; collisions have a bounded retry policy. Other platforms
reject nonzero staging options while default behavior remains unchanged.

No new production dependency or unsafe code is added. tempfile 3.27.0 was
evaluated (current release verified against crates.io) but its additional
production dependency audit closure was disproportionate to this Unix-only
exclusive-create/unlink operation. The standard library supplies the required
atomic file semantics. No publisher-trust or gate exception was introduced.

Staging checks cancellation and a request-timeout total budget between reads.
A single in-flight synchronous socket read remains bounded by the existing
per-request timeout; this is not immediate asynchronous cancellation. Failure
is explicit rather than silently accepting incomplete media.

Revisit when startup time, eligible-file coverage, disk footprint, multi-player
resource budgets, or live receiver evidence contradict the benefit. Disk
staging does not address downstream loss or receiver scheduling.
