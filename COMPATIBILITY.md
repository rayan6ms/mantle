# Mantle compatibility status

Mantle currently makes no release compatibility claim. Phase 1 freezes the Lavaplayer 2.2.6 contract. Phase 2 now has a deliberately narrow feasibility slice, but symbol-by-symbol classification remains unassessed until broader behavior evidence exists.

The authoritative structural input is `reference/lavaplayer-2.2.6-inventory.json`. The initial matrix at `compatibility/lavaplayer-2.2.6-classification.json` contains all 2,762 exported class/member symbols and deliberately marks every one `UNASSESSED`. An unassessed symbol has no `A_EXACT`, `B_SOURCE`, `C_SEMANTIC`, `D_LEGACY`, or `X_UNSUPPORTED` classification until a later phase supplies evidence and tests.

The classification schema is `compatibility/classification.schema.json`. A symbol may receive a compatibility classification only when its assessment becomes `CLASSIFIED`; tests and a concrete note must remain attached to that record. Difficult symbols must not be silently omitted or pre-labelled unsupported.

## Current evidence

- Reference artifact: `dev.arbjerg:lavaplayer:2.2.6`, JAR SHA-256 `84aba896d988e12ea24c25f87f2e88eca4be7adac31893eacabf93401da1282d`.
- Contract scope: 399 public/protected classes, 407 fields, and 1,956 methods.
- Artifact scope: manifest plus three non-class resources; no service-provider file is present.
- Public signature closure: 35 non-JDK/non-Lavaplayer external types are recorded.
- Built-in source order: ten remote managers followed separately by the local manager, extracted from the published `AudioSourceManagers.java` source.

Run `scripts/generate-reference-contract.sh` to reacquire, hash-check, regenerate, and byte-compare the frozen evidence.

## Gate A feasibility scope

The Rust emitter preserves a 24-class closure around manager/player/track/frame, event/load callbacks, userdata, markers, and futures. In this slice, Mantle's structural diff and Revapi report no API difference from the corresponding reference slice, and unchanged source compiled against the official JAR runs against Mantle alone on native Ubuntu, macOS, and Windows with JDK 11/25/26. The complete matrix passed in [CI run 31468365958](https://github.com/rayan6ms/mantle-gate-a-ci/actions/runs/31468365958) at commit `7c75f543bf97a2ae0fef12265ae134c2c0d7e0f2`.

This is not an `A_EXACT` classification for those symbols. The behavior is synthetic Gate A proof: no real loader/source, scheduling engine, audio decoder, serializer, or complete enum behavior exists. The checked-in classification matrix therefore remains `INITIAL_UNASSESSED`.

JDK 25/26 native-loading warnings remain tracked as `S-001` in `PROJECT_LEDGER.md` for the later packaging spike.

## Differential-oracle scope

Phase 3 freezes a shared 17-action synthetic scenario and normalized reference/Mantle goldens under `tests/oracle/`. It covers lifecycle, configuration defaults, loading/cancellation, metadata, user data, markers, seeking, track-detail bytes, play/pause/stop events, deterministic frame observations, and shutdown. `scripts/run-differential-oracle.sh` proves repeat determinism and writes a structured difference report.

The initial baseline has 8 equal observations and 9 expected differences. These differences are implementation inputs for Phase 4 and later phases, not compatibility classifications. In particular, reference `BYPASSED` marker behavior is tracked as `C-001`; the classification matrix remains `INITIAL_UNASSESSED` until implementation and evidence justify symbol-level claims.
