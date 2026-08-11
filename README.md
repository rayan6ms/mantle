# Mantle implementation specification

Mantle is a Rust-native replacement for Lavaplayer.

This directory is the implementation specification for an autonomous coding agent. It intentionally separates permanent rules from phase-specific work so the agent does not carry thousands of unrelated instructions into every task.

## Read order

1. `AGENTS.md` — permanent non-negotiable rules.
2. `PLAN.md` — project phases, kill-gates, and release gates.
3. Read only the specification relevant to the current phase:
   - `docs/spec/COMPATIBILITY.md`
   - `docs/spec/JVM.md`
   - `docs/spec/MEDIA.md`
   - `docs/spec/SOURCES.md`
   - `docs/spec/TESTING_PERFORMANCE.md`
   - `docs/spec/DEPENDENCIES_SECURITY.md`
4. `docs/spec/PROCESS.md` — how the agent records evidence, bugs, performance, inconsistencies, and architectural changes.
5. `TASKS.md` — the only active implementation backlog. Keep this short and rewrite it at each milestone.
6. `PROJECT_LEDGER.md` — concise active engineering memory; update only when a finding matters.

## Product statement

Mantle must provide:

- a pure Rust native API with no JVM dependency;
- a Lavaplayer 2.2.6-compatible JVM surface implemented by generated JVM bytecode forwarding to Rust through JNI;
- no committed or shipped Java/Kotlin implementation source;
- first-class YouTube support as part of Mantle 1.0;
- behavior and track-serialization compatibility measured against Lavaplayer rather than assumed;
- bounded resource use and production-grade performance/reliability.

## Compatibility promise

"Drop-in" means:

- Java/Kotlin application source should remain unchanged for APIs declared source-compatible;
- already-compiled consumers should run when Mantle replaces Lavaplayer on the runtime classpath for APIs declared binary-compatible;
- the dependency coordinate will change to Mantle's own artifact coordinate;
- Mantle must not impersonate the `dev.arbjerg` Maven namespace.

Compatibility has separate dimensions:

1. JVM binary ABI.
2. JVM source compatibility.
3. behavioral semantics.
4. extension/SPI compatibility.
5. serialized track compatibility.
6. artifact/POM/resource compatibility.
7. audio behavior.
8. source behavior.

Never collapse these into one unsupported "100% compatible" claim.

## Core principle

> Preserve Lavaplayer at the boundary. Build the inside as a modern Rust system.

Mantle core must not know about JNI, JVM class names, Maven, or Java object semantics. Those belong to the compatibility boundary.

## Agent memory discipline

The agent must preserve important discoveries without turning the repository into a diary.

Use `PROJECT_LEDGER.md` for active bugs, incompatibilities, performance findings, resource issues, open questions, and intentional debt. Move durable architectural rationale into ADRs and remove resolved ledger items after their permanent test/documentation exists.

The written plan is not sacred: reproducible evidence may reveal that an assumption is wrong. When that happens, update the specification rather than forcing the implementation to match a bad plan.
