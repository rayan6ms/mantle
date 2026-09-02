# Mantle compatibility status

Mantle 1.0.0 has completed its compatibility, hardening, and publication gates and is published.
Kill-gate D passes for the pinned Phase 14 real-consumer scope, and Phase 15 passes with an explicit
reduced-duration network-soak claim. Phase 13 completed the synthetic and structural compatibility
expansion against the fixed `dev.arbjerg:lavaplayer:2.2.6` baseline: all 2,762 exported symbols are
classified as 2,564
`A_EXACT`, 133 `C_SEMANTIC`, and 65 `D_LEGACY`, with none unassessed. The Rust emitter covers all
402 reference classes and adds twenty-three Mantle runtime classes.

The publishable JVM artifact is `io.github.rayan6ms:mantle-lavaplayer:1.0.0`. Its contract records
the four deliberate resources, eleven retained JVM dependencies, replacement of
`lavaplayer-natives` by platform-classified Mantle native artifacts, all 35 external public types,
the automatic module name, and the application-owned JDK 24+ native-access grant. The generated
artifact passes Mantle's structural diff, JVM verification and behavioral oracles, and the recorded
independent Revapi gate. Symbol classifications remain authoritative in
`compatibility/lavaplayer-2.2.6-classification.json`; intentional native and packaging boundaries
are recorded in ADR-0024 and ADR-0025.

## Phase 14 pinned real consumers

`reference/phase14-real-consumer-inventory.json` pins four upstream repositories without committing
their Java or Kotlin source. Each dependency is first compiled unchanged against the frozen 2.2.6
reference where its original dependency is older, then compiled with only the Mantle migration
overlay. Exact upstream files and licenses are locked by SHA-256 and can be reacquired with
`scripts/check-phase14-consumer-inventory.sh --verify-upstream`.

| Consumer | Revision | Role | Important boundary |
|---|---|---|---|
| `lavalink` | `3d24006d1eed2bd9b4f5916298cf87ab34408b6f` | Audio server and plugin API | Exact 2.2.6 consumer; scheduler, reusable frame provider, listeners, markers, serialization, sources, and transitive plugin types. |
| `jmusicbot` | `859e5c5862decf433f8face5eaca3372d7d27b22` | JDA music bot | JDA frame provider, ordered loading, typed user data, per-guild scheduling, transformed sources, and explicit Beam/Getyarn legacy registration. |
| `simplevoicechat_music` | `f21305f4deafc4c5869a060e8dcfbbf24d73c82b` | Minecraft voice-chat mod | Reusable PCM `MutableAudioFrame`, listener-driven queue, ordered loading, equalizer, and non-Discord transport. |
| `youtube_source` | `f45bbb7aebfcbc1c553769e04af6cd43afa8b7c3` | Third-party source extension | `AudioSourceManager`, `DelegatedAudioTrack`, HTTP façade, source serialization, executor, and container SPI usage. |

### Completed Lavalink consumer slice

The pinned Lavalink revision `3d24006d1eed2bd9b4f5916298cf87ab34408b6f` compiles unchanged against
the frozen `dev.arbjerg:lavaplayer:2.2.6` artifact. The same `plugin-api` and `Lavalink-Server`
Kotlin sources compile against `io.github.rayan6ms:mantle-lavaplayer:1.0.0` using only an ephemeral
Gradle init overlay. The overlay selects `mantle-native:1.0.0:linux-x86_64`, disables the
filename-based `lavaplayerNativesJar`, and verifies an absolute extracted native-library path.
Mantle's JAR, POM, selected native classifier, overlay, logs, and machine-readable result are
emitted under `target/phase14/lavalink-source-compatibility/`.

Regression gate: `scripts/check-phase14-lavalink-source-compatibility.sh`, including a
`NativeLoader.load(path)` smoke against the selected native classifier.

### Completed JMusicBot consumer slice

The pinned JMusicBot revision `859e5c5862decf433f8face5eaca3372d7d27b22` compiles unchanged against
the frozen Lavaplayer 2.2.6 artifact and against Mantle after only the Lavaplayer coordinate is
replaced. Its JDA `AudioSendHandler`, player scheduler/listeners, ordered loading, typed track user
data, source registration, and `YoutubeAudioSourceManager` subclass all link successfully. The
upstream POM's unavailable `com.jagrosh:jda-utilities:3.0.5` parent artifact is normalized only in
the generated target consumer using pinned JDA Utilities `jda_v4` sources; this is recorded as build
normalization rather than a Mantle compatibility change. Beam and Getyarn classes load for linkage
only, with no claim that their retired services operate.

Regression gate: `scripts/check-phase14-jmusicbot-source-compatibility.sh`.

### Completed Simple Voice Chat Music consumer slice

The pinned Simple Voice Chat Music revision `f21305f4deafc4c5869a060e8dcfbbf24d73c82b` compiles
unchanged against the frozen Lavaplayer 2.2.6 artifact and against Mantle after only its Lavaplayer
coordinate is replaced. Its reusable `MutableAudioFrame`, listener-driven queue, ordered loading,
equalizer, player state, and non-Discord transport touchpoints link successfully. A matching JVM
smoke runs against both artifacts with an explicit native load for Mantle; the generated consumer
keeps all upstream Java source unchanged. Gradle 8.6 is run with a JDK 17/21-compatible toolchain
because the workspace JDK 25 is newer than that pinned Gradle release.

Regression gate: `scripts/check-phase14-simplevoicechat-source-compatibility.sh`. Evidence is
retained under `target/phase14/simplevoicechat-source-compatibility/`.

### Completed youtube-source SPI consumer slice

The pinned `youtube-source` revision `f45bbb7aebfcbc1c553769e04af6cd43afa8b7c3` compiles unchanged
for `common` and `v2` against the frozen Lavaplayer 2.2.6 artifact and against Mantle after only
the `v2` compile-only/test coordinates are replaced. The generated gate preserves `common` on its
original compile-only dependency, verifies the actual resolved reference JAR hash and isolated
Mantle artifact, and runs matching JVM smoke for source-manager configuration and routing,
`DelegatedAudioTrack` inheritance and user data, HTTP context acquisition, source serialization,
executor factory dispatch, and container SPI linkage. The consumer targets JVM 8 bytecode, so
resolution metadata is raised to JVM 11 for Lavaplayer 2.2.6 selection while compilation remains
targeted to 8; Gradle 8.10 runs under JDK 21/17 rather than the workspace JDK 25.

Regression gate: `scripts/check-phase14-youtube-source-spi-compatibility.sh`. Evidence is retained
under `target/phase14/youtube-source-spi-compatibility/`.

### Completed deterministic real-consumer behavior slice

`compatibility/phase14-real-consumer-behavior.json` maps deterministic local scenarios to all nine
required behaviors and every pinned consumer. The reference and Mantle runs execute unchanged
JMusicBot `QueuedTrack`, `RequestMetadata`, and transformative-source class linkage plus unchanged
`youtube-source` manager and delegated-track classes. The same harness reproduces the exact
Lavalink and Simple Voice Chat scheduler, listener, immutable/reusable-frame, marker,
MessageInput/MessageOutput, ordered-loading, and source-configuration interaction shapes.

The first Mantle run exposed a real boundary: full serialization of the Java-owned
`youtube-source` track incorrectly entered the native-proxy encoder. The emitted manager now
dispatches Java-owned track details through their `AudioSourceManager` and retains the native path
for Mantle-owned proxy tracks. The complete normalized output—including the SHA-256 of the full
serialized track—is byte-for-byte equal to Lavaplayer 2.2.6. Regression gate:
`scripts/check-phase14-real-consumer-behavior.sh`; evidence is retained under
`target/phase14/real-consumer-behavior/` and the finding is recorded as ledger `C-008`.

The inventory covers every required Phase 14 behavior: `normal_player_scheduler`,
`jda_style_frame_provider`, `listeners`, `ordered_loading`, `user_data`, `markers`,
`serialized_tracks`, `source_configuration`, and `custom_source_or_subclass`.

### Phase 14 aggregate exit

`compatibility/phase14-real-consumer-exit.json` is the machine-readable Kill-gate D decision. The
aggregate gate reruns the pinned inventory, all four unchanged-consumer source/SPI gates, and the
deterministic behavior gate, then validates source, binary, semantic, SPI, serialization, artifact,
native-loading, and behavior evidence. It requires every one of the four consumers and all nine
required behaviors to map to passing evidence, and requires each migration boundary in ledger
`C-004` through `C-008` to name an explicit resolution and regression gate.

`scripts/check-phase14-real-consumer-exit.sh` records `Kill-gate D PASS`. The claim is deliberately
scoped: Beam and Getyarn remain linkage-only `D_LEGACY` classes because their retired live services
are not operationally claimed. The next phase is `phase15-hardening`.

### Phase 15 hardening exit

`compatibility/phase15-hardening-exit.json` records Phase 15 as
`PASS_WITH_REDUCED_DURATION`. The aggregate gate reruns sanitizer/fuzz, concurrency/lifecycle,
dependency, RealtimeSanitizer, native-soak, and replay/fault checks across ten evidence dimensions.
The native campaign completed its required 24 hours. The replay/fault campaign instead retains a
39h 39m uninterrupted observation with 28,096 exact nine-scenario cycles and an explicit claim
reduction: 72-hour endurance was not demonstrated and is not part of the compatibility claim.

No replay/fault service remains active, and the runner is smoke-only with a five-minute ceiling.
The later publication dependency gate replaced all 140 exact-version Cargo Vet exemptions with
local audits; `D-001` is resolved.

### Publication preflight

Publication staging is allowlist-based rather than a repository export. No Cargo package or
repository snapshot is published, so agent instructions, private development documentation, tests,
scripts, plans, local evidence, and repository metadata remain outside the artifact channel.

The five-platform native classifier matrix and aggregate verifier pass. The Central preflight stages
the compatibility JAR/POM, truthful NOTICE-only source/Javadoc companion JARs, a classifier-only
native `pom` coordinate, and all five native JARs. Correct public repository, SCM, developer, and
Apache-2.0 metadata is enforced. Each of the ten deployables has MD5, SHA-1, SHA-256, SHA-512, and a
detached PGP signature, yielding an exact 60-file Portal bundle. Regression gates reject missing
signatures, stale metadata, and Java/Kotlin source in placeholder archives. This offline
preflight produced the exact bundle. The CycloneDX/SLSA gate passes, and Cargo
Vet now reports 171 fully audited packages with zero exemptions. Central identity also passes:
`io.github.rayan6ms` is verified, the primary signing key independently round-trips through a
Central-supported keyserver, and the protected Portal token authenticates without being retained in
the repository. The release-key-signed bundle was uploaded as `USER_MANAGED`; deployment
`9205c170-5232-4817-980f-0ff92e581ee9` reached `VALIDATED`. Its deployment-scoped hosted gate
downloaded and matched all ten deployables, resolved the JVM and native Maven coordinates, verified
the emitted classes, and loaded the released Linux native library. Only after that evidence was
committed, the separate immutable publish action advanced the deployment to `PUBLISHED`. All 60
files on the public Maven repository match the signed staged repository exactly, and public
unauthenticated Maven/JVM/native consumer verification passes locally and in hosted run
`33601586857`. Mantle 1.0.0 publication is complete. The release-only Portal token was then revoked
and its GitHub Actions secret removed.

Three migration constraints are already explicit. Lavalink's build finds and repackages
`lavaplayer-natives` by filename, so its Mantle overlay must select a `mantle-native` classifier and
configure the verified library path instead. JMusicBot imports the `D_LEGACY` Beam and Getyarn
managers; Phase 14 tests their linkage and deterministic legacy disposition without claiming those
retired services operate. Neither result is hidden inside a generic source-compatibility claim.
The `youtube-source` v2 build targets JVM 8 while Lavaplayer 2.2.6 publishes JVM 11 metadata, so
its overlay raises only dependency-resolution attributes and keeps emitted bytecode at Java 8.

## Existing behavioral evidence

Gate A covers reference-equal JVM behavior, callbacks and reentrancy, one-million explicit wrapper
releases, GC cleanup, classloader collection, clean JVM exit, and immediate ABI mismatch rejection.
The differential oracle covers lifecycle, configuration defaults, loading and cancellation,
metadata, user data, markers, seeking, serialization bytes, event order, deterministic frames, and
shutdown. Phase 14 now determines whether those proven boundaries survive unchanged application and
extension source.

## Intentional serialization validity divergence

Java strings can contain unpaired UTF-16 surrogate code units, while Rust `String` represents well-formed Unicode scalar values. Mantle rejects serialized metadata containing unpaired surrogates. Valid Unicode, including supplementary characters encoded as Java modified UTF-8 surrogate pairs, interoperates exactly. The bounded malformed-input tests in `mantle-core` and `docs/compatibility/TRACK_SERIALIZATION.md` record this decision.
