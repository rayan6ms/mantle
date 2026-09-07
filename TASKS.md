# Current task: bounded finite-source staging for Raydio

- [x] Measure Oracle compressed input: streamed max 244.864 ms versus staged 0.957 ms; startup 0.603 versus 2.286 seconds, 3.27 MiB anonymous file.
- [x] Implement opt-in size-bounded anonymous-file staging using the existing HTTP policies and cancellation.
- [x] Verify full validation, staging cancellation/errors, seek after origin shutdown, oversized fallback, default streaming, and resource ceilings.
- [x] Run media regressions, formatting, Clippy, and supply-chain checks; record new dependency decision.
- [ ] Integrate and repeat real Raydio playback before any reliability claim.
