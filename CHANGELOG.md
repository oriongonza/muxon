# Changelog

All notable changes are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning: [Semantic Versioning](https://semver.org/) pre-1.0 model (see IMPLEMENTATION_PLAN.md).

## [Unreleased]

### Added
- Workspace-level dependencies, lints, build profiles, toolchain pin, formatting config.
- `resurreccion-planner`: Core planning layer types (`NodeId`, `PlanNode`, `Plan`, `PlanResult`, `NodeResult`).
- `resurreccion-planner`: Capability verb constants module (`capture.layout`, `restore.layout`, `capture.shell`, `restore.shell`, `capture.aigent`, `restore.aigent`, `capture.editor`, `restore.editor`).
- `resurreccion-planner`: `plan_capture(capabilities: &Capability) -> Plan` — captures a single `CAPTURE_LAYOUT` node plan.
- `resurreccion-planner`: `plan_restore(manifest: &SnapshotManifest, capabilities: &Capability) -> Plan` — restores from a single `RESTORE_LAYOUT` node plan.
- `resurreccion-planner`: `execute(plan: &Plan, mux: &dyn Mux, store: &Store) -> anyhow::Result<PlanResult>` — executes plan nodes in DAG topological order, with dry-run mode support.
- `resurreccion-store`: SQLite CRUD implementation for workspaces, runtimes, snapshots, events.
- `resurreccion-daemon`: Tokio async runtime with Envelope protocol support, verb dispatch system, single-instance guard, and graceful SIGTERM/SIGINT shutdown with 2s drain timeout.
- `resurreccion-cli`: muxon binary with clap subcommands (doctor, workspace, save, restore, tree, events), shell completions (bash, zsh, fish).
- `resurreccion-zellij`: `ZellijMux` implementing `Mux` trait via zellij CLI (discover, create, attach, capture, subscribe_topology).
- `resurreccion-dir`: Path canonicalization (`canonicalize`), git detection (`detect_git`), and binding key composition (`compose_binding_key`) with `PathScoped` and `RepoScoped` scopes.
