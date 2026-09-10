# Mantle MP4 EOF seek repair

Source: `symphonia-format-isomp4` 0.6.1, the crates.io release of
<https://github.com/pdeljanov/Symphonia>. Original copyright notices and MPL-2.0
license are retained. The package/library is renamed `mantle-symphonia-isomp4` /
`mantle_symphonia_isomp4` to make the downstream modification explicit. It is
registered by Mantle's private probe instead of the upstream MP4 feature.

The sole demuxer behavior change retains one previously parsed `mdat` header as
a resynchronization point. `next_packet` consumes that header while discovering
EOF; seeking cached samples afterward otherwise fails with `no atom pending
read`. A successful seek restores this point only if no atom is pending. It
does not rewind fragment discovery, duplicate fragment tables, alter raw sample
offsets, or bypass atom/packet bounds. Storage is one optional atom header per
demuxer, independent of track duration.

Regressions live in `mantle-media::youtube_playback::tests`: ordinary and
fragmented MP4 can drain, seek to zero and reproduce the exact decoded PCM.
Encoded replay also covers AAC, HE-AAC, FLAC, MP3, Vorbis and resampled PCM.

Upstream formatting is retained to keep the patch inspectable. Revisit/remove
this vendored crate when an upstream release fixes seek-after-EOF and passes
these regressions. This is a demuxer repair, not a new codec implementation.
