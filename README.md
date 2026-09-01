# Mantle

Mantle is a Rust-native replacement for [Lavaplayer](https://github.com/lavalink-devs/lavaplayer).
It combines a native Rust audio and media engine with a generated JVM compatibility layer that
forwards Lavaplayer-compatible calls through JNI.

> [!WARNING]
> Mantle is pre-release software. Its public artifacts are not yet available from Maven Central,
> and it should not be treated as production-ready until the remaining release gates are complete.

## Goals

- Provide a pure Rust API without a JVM dependency.
- Support existing Java and Kotlin consumers through a Lavaplayer 2.2.6-compatible JVM surface.
- Preserve observable behavior and serialized-track compatibility where explicitly verified.
- Keep resource use bounded across media loading, decoding, audio processing, and native lifetimes.
- Ship first-class YouTube support as part of Mantle 1.0.

Compatibility is measured separately across JVM ABI, source, behavior, extensions, serialization,
packaging, audio, and media sources. See [COMPATIBILITY.md](COMPATIBILITY.md) for the current,
evidence-scoped status; Mantle does not make a blanket 100% compatibility claim.

## Repository layout

- `crates/mantle-core` — player state, lifecycle, timing, and serialization primitives.
- `crates/mantle-media` — media inputs, formats, playlists, and remote sources.
- `crates/mantle-audio` — bounded PCM processing and frame delivery.
- `crates/mantle-opus` — Opus encoding boundary.
- `crates/mantle-xaac` — native libxaac integration.
- `crates/mantle-jvm` — JNI runtime and generated JVM compatibility boundary.
- `tools/` — compatibility, oracle, and benchmark tooling.

## Building

Mantle currently requires Rust 1.97.1. Clone its pinned native dependency and run the workspace
checks with:

```sh
git submodule update --init --recursive
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
```

Some JVM compatibility and media gates additionally require a JDK, Maven, Deno, or platform-native
toolchains. The continuous-integration workflow records the exact supported test matrix.

## Security

Please report vulnerabilities through GitHub's private security-advisory mechanism as described in
[SECURITY.md](SECURITY.md). Do not open a public issue containing credentials or vulnerability
details.

## License

Mantle is licensed under the [Apache License 2.0](LICENSE).
