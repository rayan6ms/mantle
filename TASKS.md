# Current task: progressive finite-source buffering for Raydio

- [x] Measure complete-stage Oracle startup and reproduce the delayed-remainder bottleneck.
- [x] Implement opt-in prefix buffering with bounded owned workers/cache and joined cancellation.
- [x] Preserve range validation, retries, deadlines, proxy transport and cached repeat.
- [x] Reproduce and fix eager WebM metadata/tail-index waits, preserving explicit seeks atomically.
- [x] Verify exact Opus/AAC output, failures, cancellation/drop and storage/resource bounds.
- [x] Complete media release checks: 241 tests passed, eight existing manual fixtures ignored; Clippy and advisory/license/vet audits pass.
- [ ] Complete Raydio's live Oracle qualification before a live improvement claim.
