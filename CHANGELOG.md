# Changelog

All notable changes are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning: [Semantic Versioning](https://semver.org/) pre-1.0 model (see IMPLEMENTATION_PLAN.md).

## [Unreleased]

### Added
- Workspace-level dependencies, lints, build profiles, toolchain pin, formatting config.
- `resurreccion-planner`: Core planning layer types (`NodeId`, `PlanNode`, `Plan`, `PlanResult`, `NodeResult`).
- `resurreccion-planner`: Capability verb constants module (`capture.layout`, `restore.layout`, `capture.shell`, `restore.shell`, `capture.aigent`, `restore.aigent`, `capture.editor`, `restore.editor`).
- `resurreccion-planner`: `execute()` stub signature (implementation by Lane F).
- `resurreccion-store`: SQLite CRUD implementation for workspaces, runtimes, snapshots, events.
- `resurreccion-daemon`: Tokio async runtime with Envelope protocol support, verb dispatch system, single-instance guard, and graceful SIGTERM/SIGINT shutdown with 2s drain timeout.
