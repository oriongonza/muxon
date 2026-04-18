# Changelog

All notable changes are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning: [Semantic Versioning](https://semver.org/) pre-1.0 model (see IMPLEMENTATION_PLAN.md).

## [0.1.0] - 2026-04-18

### Added
- Integration tests (`crates/resurreccion-daemon/tests/integration.rs`) with daemon subprocess spawning, socket communication, and doctor.ping + workspace.list verification.
- `resurreccion-daemon` CLI argument parsing with `serve --socket <path>` support for custom socket paths (testing).
- Full documentation pass with `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` compliance across all public crates.

### Fixed
- Fixed rustdoc lint deprecation in `resurreccion-cli` (changed `missing_crate_level_docs` to `rustdoc::missing_crate_level_docs`).
- Resolved clippy warnings in `resurreccion-daemon` handlers (redundant clone, or_fun_call, expect_fun_call).

## [Unreleased]

### Added
- `resurreccion-claude`: ClaudeAigent implementing Aigent trait via Anthropic Messages API.
- `resurreccion-proto`: shell.capture, shell.restore, aigent.generate, aigent.list verb constants.
- `resurreccion-shell`: ShellCapture type, ShellAdapter trait, ProcShellAdapter stub.
- `resurreccion-shell`: ProcShellAdapter captures cwd, cmdline, env from /proc filesystem.
- `resurreccion-aigents`: Aigent trait (model_id, generate, capabilities), Message/Role types, AigentCapability bitflags (STREAMING, FUNCTION_CALLING, IMAGE_INPUT), and conformance test suite.
- Workspace-level dependencies, lints, build profiles, toolchain pin, formatting config.
- `resurreccion-planner`: Core planning layer types (`NodeId`, `PlanNode`, `Plan`, `PlanResult`, `NodeResult`).
- `resurreccion-planner`: Capability verb constants module (`capture.layout`, `restore.layout`, `capture.shell`, `restore.shell`, `capture.aigent`, `restore.aigent`, `capture.editor`, `restore.editor`).
- `resurreccion-planner`: `plan_capture(capabilities: &Capability) -> Plan` — captures a single `CAPTURE_LAYOUT` node plan.
- `resurreccion-planner`: `plan_restore(manifest: &SnapshotManifest, capabilities: &Capability) -> Plan` — restores from a single `RESTORE_LAYOUT` node plan.
- `resurreccion-planner`: `execute(plan: &Plan, mux: &dyn Mux, store: &Store) -> anyhow::Result<PlanResult>` — executes plan nodes in DAG topological order, with dry-run mode support.
- `resurreccion-store`: SQLite CRUD implementation for workspaces, runtimes, snapshots, events.
- `resurreccion-daemon`: Tokio async runtime with Envelope protocol support, verb dispatch system, single-instance guard, and graceful SIGTERM/SIGINT shutdown with 2s drain timeout.
- `resurreccion-daemon`: Event bus integration via `rt-events` with non-blocking channel-based subscriber pattern for durable event persistence to store.
- `resurreccion-daemon`: Snapshot verb handlers (create, restore, list, get) capturing and restoring layout state via planner and Mux backends.
- `resurreccion-daemon`: Events verb handlers (tail) streaming stored events to clients.
- `resurreccion-proto`: Verb constants for snapshot operations (SNAPSHOT_CREATE, SNAPSHOT_RESTORE, SNAPSHOT_LIST, SNAPSHOT_GET) and event streaming (EVENTS_TAIL).
- `resurreccion-cli`: muxon binary with clap subcommands (doctor, workspace, save, restore, tree, events), shell completions (bash, zsh, fish).
- `resurreccion-zellij`: `ZellijMux` implementing `Mux` trait via zellij CLI (discover, create, attach, capture, subscribe_topology).
- `resurreccion-dir`: Path canonicalization (`canonicalize`), git detection (`detect_git`), and binding key composition (`compose_binding_key`) with `PathScoped` and `RepoScoped` scopes.
