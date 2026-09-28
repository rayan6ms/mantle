# Mantle YouTube source versus old LavaPlayer source

Mantle is not a line-for-line port of LavaPlayer's original Java YouTube source.
It is a Rust source manager with the same public source responsibilities: route
video/search/playlist identifiers, query YouTube metadata, select a playable
format, resolve signatures, and stream bounded media ranges.

Mantle currently goes further than the old built-in source in several areas:

- ordered InnerTube client fallback (`Music`, `AndroidVr`, `Web`, embedded Web,
  TV, and VisionOS);
- explicit OAuth refresh-token handling and optional browser cookies;
- optional isolated Deno/EJS signature resolution;
- bounded response, URL, cipher, playlist, retry, and media-range limits;
- cookie-authenticated watch-page playback fallback;
- explicit Opus passthrough and bounded staging/read-ahead;
- route-policy hooks that can bind source and media requests to selected local
  addresses.

The old built-in source has a simpler Java HTTP/context-filter design and does
not include the current maintained YouTube client matrix, OAuth/PoToken policy,
or Mantle's bounded Rust media handoff. The maintained Lavalink `youtube-source`
plugin is the more relevant comparison: it also uses multiple InnerTube clients,
OAuth, PoToken support, cipher handling, and optional IP rotation.

The important limitation is shared by both designs: YouTube can reject playback
from a datacenter egress even when search metadata and browser watch pages work.
Changing a search result into a direct URL does not change that playback request.
