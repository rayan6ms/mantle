# Bounded source recovery and post-EOF seek

HTTP response acquisition retries did not cover truncated bodies. A range body
may now resume at the exact consumed byte under an unchanged strong ETag. Missing
or weak identity, changed total size, or wrong offsets fail before splicing bytes.
The configured retry count is shared by all body failures for an input, including
failures separated by successful reads; a recovery episode shares one request
deadline and cannot multiply nested retries. Explicit seek ends that episode
but does not replenish its lifetime retry allowance. No extra media buffer is
introduced. Local HTTP regressions compare every recovered byte and exercise
identity rejection, cancellation, exhaustion and deadline behavior.

Finite EOF retains the seekable media session through final DSP drain. Seeking
clears pending packets and reconstructs the decoder from its bounded codec
parameters, then resets resampling, filters and encoding. This also resets AAC
noise-substitution history left intact by the backend's reset method. Decoder
reconstruction occurs on seek, never in the frame hot path. The input and codec
parameters remain bounded by the existing media limits.

This exposed Symphonia 0.6.1 MP4 iterator state loss at EOF. The narrowly renamed
vendored demuxer is documented in
`third_party/symphonia-format-isomp4/MANTLE-PATCH.md`. Its license remains MPL-2.0.
The extra maintenance is justified by ordinary and fragmented MP4 regressions;
remove the fork after an upstream fix passes them. No receiver/network improvement
is inferred from source-side tests.

`AudioFrameError::InvalidFilterConfiguration` lets consumers reject invalid DSP
parameters without reporting an unrelated resampler failure.

Exact HE-AAC replay exposed a native priming artifact: with error concealment
enabled, libxaac defers the first frame's synthesis but reports bytes in its
scratch-aliased output buffer as PCM. Those bytes contained channel pointers.
The tracked native patch now clears that priming output when consuming
`first_frame`; normal synthesized audio is unchanged. Regressions cover both
HE-AAC profiles and verify exact replay rather than tolerating varying samples.
