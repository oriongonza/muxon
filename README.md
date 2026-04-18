# Muxon

Muxon is a workspace system built around **Resurreccion**: a general model for
capturing and restoring terminal runtimes, editors, aigents, and nested remote
contexts.

This repository is intentionally a monorepo:

- `crates/` contains the Rust workspace crates
- `plugins/` contains UI/plugin entry points
- `docs/` contains the living architecture documents
- `IMPLEMENTATION_PLAN.md` is the delivery plan for the first serious build

Control-plane split:

- Unix sockets carry inter-process control-plane traffic
- `rt-events` provides the in-process observer bus for daemon-side decoupling

Current state:

- the repo layout is scaffolded
- the crate names are fixed
- the adapter trait is intentionally left undesigned for now
- the next step is implementing the control plane and persistence model
