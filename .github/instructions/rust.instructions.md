---
applyTo: "crates/**/*.rs,plugins/**/*.rs"
---

# Rust Review Guidelines

Use these instructions for Rust code review in this repository.

- Review behavior before style. Prioritize correctness, protocol safety, restore fidelity, resource lifetime, and compatibility over formatting or naming nits.
- Be strict around `unsafe`, raw pointers, FFI, `static mut`, process-wide globals, and interior mutability. Ask whether the same result can be achieved without widening the unsafety surface.
- In daemon and observer code, keep `rt-events` callbacks synchronous and non-blocking. If work might block, spawn or enqueue it elsewhere instead of doing it inline.
- Treat proto, snapshot, store, and event-taxonomy changes as compatibility-sensitive. Flag silent wire-format, on-disk, or replay-breaking changes.
- In long-lived processes, be skeptical of `unwrap`, `expect`, `panic!`, and silent error swallowing. Prefer explicit error propagation unless crashing is intentional and documented.
- Check cleanup paths carefully: socket replacement, stale file removal, child-process supervision, subscription lifetimes, and lock release on error.
- For concurrency, review lock ordering, backpressure, cancellation, and `Send`/`Sync` assumptions. Flag code that can deadlock, livelock, or block the daemon event loop.
- Be skeptical of new dependencies. Prefer standard library or existing workspace crates unless the new dependency clearly earns its cost in maintenance, security, and licensing.
- Prefer targeted tests for high-risk changes: protocol compatibility, event ordering, stale-socket behavior, idempotent restore, and failure-path cleanup.
- Avoid speculative micro-optimizations unless the code is already on a measured hot path or the patch includes evidence.
