# YouTube PoToken audit

Mantle accepts a static `poToken` and matching `visitorData`, and places them in
`serviceIntegrityDimensions.poToken` plus the Web/WebEmbedded client context.
The pair is validated as a pair and is never logged.

Oracle testing generated a session pair through Invidious Companion, but its
validator could not obtain playable formats for several videos from the Oracle
egress. Those values were discarded and are not installed in production.

The current Companion flow also mints per-video content tokens after the session
token. A static pair is therefore not automatically equivalent to the complete
Companion flow. Before installing another pair, validate the exact request path
against a representative video and confirm whether the selected Web client
needs the per-video token in addition to session `poToken` and `visitorData`.

Mantle now supports an optional Companion player endpoint. When configured, it
sends the video ID to Companion with its bearer secret; Companion performs the
BotGuard/session work and mints the per-video token. Mantle parses the returned
player response and retains its normal bounded media and fallback pipeline.
This is preferred over persisting a static pair, but it requires a separately
managed Companion process on the same egress.

Current Oracle state: OAuth refresh token and browser cookies are present; a
working PoToken/visitor-data pair is absent. This is intentional because a token
that fails validation is worse than an explicit playback failure and must not be
treated as authentication success.
