# IMPLEMENTATION PLAN

## Goal

Build Muxon as a composable workspace and Resurreccion system:

- infinite logical nesting through workspaces rather than native mux session hierarchies
- per-directory reopen semantics such as `muxon --dir .`
- always-on save/restore across local and remote hosts
- first-class Resurreccion for shells, runtimes, editors, and aigents
- small internal components wired together through one monorepo and one control plane

## Design Constraints

- `Muxon` is the top-level project and UX entry point
- `Resurreccion` is the abstraction for capture and restore
- all code lives under one repository
- adapters such as Codex and Claude stay as separate crates
- the central adapter trait stays undesigned for now
- all durable state changes must flow through a single control plane instead of direct database writes
- `rt-events` is the in-process event and observer mechanism used to decouple components inside long-lived processes

## Leverage Principles

Every component is built so its interactions multiply value rather than add it. These principles are how each crate stays small while the system stays expressive.

- **Event log as the source of truth.** All durable state changes are events. Workspaces, snapshot indices, and runtime registries are projections derived from the log and rebuildable at any time. Time-travel, audit, and replication fall out for free.
- **Content addressing.** Snapshot artifacts are addressed by BLAKE3 hash. Identical state is stored once. Diffs and branches collapse into manifest comparisons rather than payload comparisons.
- **Capability negotiation.** Adapters advertise typed verbs (`capture.shell.cwd`, `restore.nvim.buffer`, `enumerate.zellij.panes`, ...). The planner composes capture and restore DAGs by matching intent to advertised capabilities. New adapters drop in without changing the planner.
- **Local/remote symmetry.** `resurreccion-remote` is a transport, not a feature. Every CLI and plugin operation works against a local socket or an SSH-tunneled remote daemon with identical semantics. Remote support is something we maintain, not something we build twice.
- **Subscribe-everything, three layers.** Events flow through three layers that share semantics: an in-process `rt-events` bus inside the daemon (sync callbacks, ~38 ns dispatch, `TypeId` as the subscription key), a durable append-only log in `resurreccion-store` (the source of truth, replayable), and a proto subscription channel that projects the in-process bus to remote observers. CLIs, plugins, adapters, replicators, and observability sinks all consume the same stream. UIs become passive renderers; replication becomes one more observer.
- **Pure planners, dirty executors.** Planning is a pure function from `(state, intent) → DAG`. Execution is the only side-effecting layer. Plans are inspectable, dry-runnable, and testable without touching real runtimes.
- **Restoration as a lattice, not a boolean.** Every artifact carries a fidelity grade (`exact`, `stateful`, `structural`, `historical`). The system reports what it could and could not restore; a degraded restore is a usable restore.
- **Composable workspaces.** Workspaces nest like trees, reference each other like graphs, and can adopt remote workspaces as children. The same navigation primitives work at every level of the hierarchy.
- **One verb, many surfaces.** Each user-visible verb (`open`, `save`, `restore`, `search`) is implemented once in the daemon and surfaced by every UI. The CLI, the Zellij plugin, future GUIs, and CI invocations all hit the same code path.

## Monorepo Layout

```text
muxon/
  Cargo.toml
  IMPLEMENTATION_PLAN.md
  README.md
  crates/
    resurreccion-core/
    resurreccion-proto/
    resurreccion-store/
    resurreccion-planner/
    resurreccion-daemon/
    resurreccion-cli/
    resurreccion-mux/
    resurreccion-zellij/
    resurreccion-shell/
    resurreccion-aigents/
    resurreccion-codex/
    resurreccion-claude/
    resurreccion-nvim/
    resurreccion-emacs/
    resurreccion-dir/
    resurreccion-remote/
  plugins/
    resurreccion-zellij-plugin/
  docs/
    architecture.md
    protocol.md
    snapshot-format.md
```

## Crate Responsibilities

### Foundation

- `resurreccion-core`
  - shared domain types
  - identifiers
  - workspace and runtime models
  - snapshot metadata and restoration-level enums

- `resurreccion-proto`
  - request and response envelopes
  - daemon transport messages
  - stable JSON shapes for CLI and plugin integration

- `resurreccion-store`
  - SQLite schema and migrations
  - event log storage
  - snapshot indexing
  - query layer for workspaces, runtimes, and Resurreccion artifacts

- `resurreccion-planner`
  - restore-plan construction
  - dependency ordering
  - orchestration logic for nested restores

### Control Plane

- `resurreccion-daemon`
  - long-lived per-host coordinator
  - Unix socket control plane
  - `rt-events` observer bus for in-process event fanout
  - autosave scheduling
  - event subscriptions
  - locking and state mutation serialization

- `resurreccion-cli`
  - main user-facing binary
  - commands such as `open`, `save`, `restore`, `tree`, and `doctor`
  - daemon client, not a direct database writer

### Runtime and Locator Adapters

- `resurreccion-mux`
  - common multiplexer interface that every backend implements
  - defines the `Mux` trait covering session discovery, attach/detach, pane enumeration, layout introspection, layout application, send-keys, and topology subscriptions
  - defines the pane-as-resource model with stable pane identity across snapshots and restarts
  - defines backend-neutral layout primitives (tabs, splits, sizes, focus)
  - defines the topology event types emitted on the rt-events bus (`PaneOpened`, `PaneClosed`, `FocusChanged`, `LayoutChanged`)
  - declares capability flags for backend-specific features (plugin embedding, copy mode, scrollback formats)
  - ships a conformance test suite that any backend must pass before being accepted

- `resurreccion-zellij`
  - the Zellij implementation of the `Mux` trait from `resurreccion-mux`
  - isolates Zellij-specific concerns (CLI invocation, IPC socket, plugin embedding) behind the trait
  - runtime discovery
  - attach, create, capture, and restore hooks

- `resurreccion-shell`
  - shell and pane process inspection
  - cwd, titles, and command metadata capture

- `resurreccion-dir`
  - canonical path resolution
  - directory and git-root binding keys
  - host-scoped workspace lookup keys

- `resurreccion-remote`
  - SSH-based delegation to a remote daemon
  - remote workspace discovery and attach flows

### Resurreccion Families

- `resurreccion-aigents`
  - shared aigent-domain logic
  - resumability classification
  - common artifact tracking for Codex-like and Claude-like tools

- `resurreccion-codex`
  - Codex-specific metadata capture and resume integration

- `resurreccion-claude`
  - Claude-specific metadata capture and resume integration

- `resurreccion-nvim`
  - Neovim-specific state capture and restore

- `resurreccion-emacs`
  - Emacs-specific state capture and restore

### UI Surface

- `resurreccion-zellij-plugin`
  - thin UI entry point for Zellij
  - delegates to the daemon for real state reads and writes
  - does not own business logic

## Per-Crate Leverage Notes

Each crate is sized to be a multiplier, not a leaf feature. The notes below describe how each crate compounds when combined with the others.

- `resurreccion-core` — owns ULID-based identifiers, the capability verb taxonomy, the fidelity lattice, the BLAKE3 content-hash type, workspace path algebra, and the canonical event type taxonomy consumed by `rt-events`. Because `rt-events` keys subscriptions on `TypeId`, defining event types in one crate keeps every observer in the system wired to the same channel. Every other crate depends on this one; every shared invariant lives here. Touching this crate has system-wide effect, so it is the single place to encode invariants once and never re-encode them.
- `resurreccion-proto` — self-describing capability advertisement on connect; streaming responses for events and large payloads; bidirectional RPC so the daemon can call back into adapter subprocesses; explicit schema versioning with declared upgrade paths; trace context threaded through every envelope. The proto is also the source for generated CLI completions and SDK clients.
- `resurreccion-store` — exposes only an append-only event API for writes; reads are served by derived projections that can be dropped and rebuilt from the log; ships a content-addressed blob store for snapshot artifacts; the workspace graph is a first-class queryable surface with parent, child, ref, and remote-link edges. Designed so the event log plus the blob store are sufficient to reconstruct the entire system on a fresh host.
- `resurreccion-planner` — pure DAG construction over capabilities; one planner serves both capture and restore (capture-DAG and restore-DAG are duals over the same capability graph); pluggable execution strategies (sequential, parallel, dry-run, replay); explicit partial-success reporting with per-node fidelity; plans are inspectable artifacts that can be diffed and reviewed before execution.
- `resurreccion-daemon` — owns the in-process `rt-events` bus (sync callbacks, ~38 ns dispatch) and projects its emissions onto the proto subscription channel for remote consumers; manages workspace leases so only one writer mutates a workspace at a time; supervises adapter subprocesses with restart and health policy; hot-reloads adapter manifests so capabilities can change without a daemon restart. The daemon is the only process with write access to the store; everything else is a client.
- `resurreccion-cli` — every command emits machine-readable JSON behind a flag; shell completions and man pages are generated from the proto schema; the same binary speaks to a local socket or an SSH-tunneled remote daemon with no code path differences; commands compose through stdout pipelines so users can build their own workflows without code changes.
- `resurreccion-mux` — the single seam between the Resurreccion world and any terminal multiplexer. Daemon, planner, snapshot store, and proto all target the `Mux` trait; no other crate names Zellij, tmux, or kitty directly. Adding a backend means writing one crate that passes the conformance suite; nothing upstream changes. Backend-neutral topology events on the rt-events bus mean a single set of subscribers covers every multiplexer.
- `resurreccion-zellij` — the first `Mux` implementation. Isolates Zellij-specific quirks (CLI invocation, IPC socket, plugin embedding) so they cannot leak into the rest of the system. Future backends (`resurreccion-tmux`, `resurreccion-kitty`, `resurreccion-wezterm`) follow the same shape and reuse the same conformance suite.
- `resurreccion-shell` — process-tree introspection with OS-specific backends; optional prompt-side instrumentation for high fidelity; cold-start heuristics when no breadcrumb exists; reusable from any runtime adapter that hosts shells, including future tmux or kitty backends.
- `resurreccion-dir` — stable binding keys composed from realpath, git remote, and worktree identity; detects repo identity drift across renames, forks, and clones; emits hint records that let the same logical workspace bind to multiple physical checkouts on the same host.
- `resurreccion-remote` — presents the local proto over an SSH-tunneled socket; caches remote projections so trees remain navigable when offline; supports `remote-as-link` so a workspace tree can adopt a remote workspace as a subtree under a single navigation surface.
- `resurreccion-aigents` — common turn-by-turn transcript schema; resumability classes (`external-id-resumable`, `transcript-replayable`, `advisory-only`); cross-aigent artifact attribution so any file or pane can be traced back to the turn that produced it; token and cost accounting normalized across providers.
- `resurreccion-codex` / `resurreccion-claude` — thin mappings into the shared aigent model; declare capabilities at handshake; carry only provider-specific resume references and quirks. Adding a new provider is a new crate of comparable size, never a planner change.
- `resurreccion-nvim` / `resurreccion-emacs` — editor-side companion plugin pattern; capture buffers, swap files, undo trees, jump lists, registers; restore into a running editor or a fresh launch through the same code path. Editor adapters are the canonical example of high-fidelity stateful restore.
- `resurreccion-zellij-plugin` — pure renderer over daemon proto; ships a reusable widget kit (workspace tree, snapshot timeline, capability matrix, transcript view) that any future TUI or GUI surface can reuse without re-implementing protocol handling.

## Roadmap

We follow Rust's pre-1.0 model: ship usable software early, iterate on the API in response to real use, freeze surfaces only once their shape is right.

Versioning follows semver. We are at **0.0.0** today: scaffold and plan, no implementation. The 0.0.x line carries pre-MVP implementation work. **0.1.0 is the first usable cut** — the version where `muxon --dir .` round-trips a Zellij workspace end to end. From there, each minor bump (0.2.0, 0.3.0, ...) lands a meaningful capability. **1.0.0 is the freeze** reached after a stabilization phase, not the launch — it ships nothing new on top of the last 0.x.0 and changes only the compat policy.

Every pre-1.0 release may rename, restructure, or reshape any surface. Each release publishes migration notes covering anything that changed. The milestones in the next section are the work units; the versions below group those units into shippable cuts.

### 0.0.x — Pre-MVP Implementation

Each 0.0.x release lands one piece of the road to MVP. The goal is to reach 0.1.0 quickly without skipping foundations that future versions cannot retrofit.

#### 0.0.0 — Scaffold (M0, current)

Repository layout, crate stubs, planning documents. No implementation. No daemon, no store, no CLI behavior beyond what `cargo build` produces.

#### 0.0.1 — Daemon Handshake (M1)

Daemon starts, exposes a Unix socket, accepts connections. CLI connects and runs `doctor`. Proto envelope shaped, error codes defined.

Exit when `muxon doctor` reports a healthy daemon over the socket.

#### 0.0.2 — Durable State (M2)

`resurreccion-store` ships SQLite schema, migrations, and basic CRUD for workspaces, runtimes, and snapshots. The daemon mediates every write; nothing else touches the database directly.

Exit when a workspace can be inserted, retrieved, and listed entirely through the daemon.

#### 0.0.3 — Directory Binding (M3)

`resurreccion-dir` produces stable binding keys from realpath, git remote, and worktree identity. `muxon --dir .` resolves a path to a workspace, creating one if absent. `last_opened_at` persisted.

Exit when running `muxon --dir .` twice in a row resolves to the same workspace ID.

#### 0.0.4 — Mux Trait and Zellij Backend (M4)

`Mux` trait shipped in `resurreccion-mux` with conformance suite. `resurreccion-zellij` implements it: discover, create, attach, basic capture. No restore yet.

Exit when `muxon --dir .` opens a Zellij session bound to the workspace and reopening attaches to the same session.

#### 0.0.5 — Bus and Autosave (M5 partial, M11 partial)

`rt-events` bus runs inside the daemon. Event types defined in `resurreccion-core`. Autosave on attach, detach, and topology changes. A subscriber writes durable events to `resurreccion-store`.

Exit when topology changes in Zellij are observable via `events tail`.

#### 0.0.6 — Structural Restore (M5 structural complete)

Layout snapshot captures tabs, panes, and splits. Restore recreates the same layout in a fresh Zellij session. The restoration fidelity lattice surfaces in CLI output.

Exit when closing a workspace and reopening it recreates the Zellij layout.

### 0.1.0 — First Usable Cut (MVP)

The smallest valuable thing. `muxon --dir .` opens a directory-bound workspace, captures, and restores Zellij + structural layout. A user can `muxon --dir .`, work in Zellij, close it, and `muxon --dir .` again to find their layout intact. CLI is the only surface.

This is the version where "MVP" is honest: a usable end-to-end workflow exists.

### 0.x.0 — Iteration Beyond the MVP

Each minor bump lands a meaningful capability. Compat may break across 0.x; migration notes per release.

#### 0.2.0 — Stateful Capture (M5 deeper, shell adapter)

Shell cwd, command, and pane title captured per pane. Restore lands the user in the same cwd with the same prompt. First proof that the planner can compose multiple capture verbs into one save.

#### 0.3.0 — First Aigent (M6 partial, Claude adapter)

`resurreccion-aigents` ships the common aigent model. `resurreccion-claude` resumes external session IDs captured at autosave time; aigent state binds to panes and shows up in `muxon tree`. First validation that the aigent abstraction is the right shape — and the first chance to discover that it is not.

#### 0.4.0 — Capability Handshake (M10)

Adapters advertise capabilities on connect. The planner consumes the capability map rather than calling adapters by name. `muxon doctor` renders a per-host capability matrix. Ad hoc adapter calls scattered through the daemon get pruned and replaced with capability lookups.

#### 0.5.0 — Content Addressing and Subscriptions (M12, M13)

BLAKE3 content addressing for snapshot artifacts (automatic dedup; manifest-only diffs). Proto subscription channel projects rt-events emissions to remote consumers; `events tail` migrates to subscriptions. First chance to find that an early type or shape needs to change before freeze.

#### 0.6.0 — Second Aigent (Codex)

`resurreccion-codex` as the second aigent backend. Anything Claude-specific that leaked into the aigent abstraction surfaces here and gets fixed. Validates that adding an adapter is genuinely a one-crate change.

#### 0.7.0 — Editors (Neovim, Emacs)

`resurreccion-nvim` and `resurreccion-emacs`. Editor-side companion plugins capture buffers, swap files, undo trees, jump lists, registers. The fourth adapter family lands; we discover whether the planner, fidelity lattice, and capability taxonomy hold up under the most stateful adapters in the system.

#### 0.8.0 — Tree TUI (M7)

Standalone TUI over the workspace tree. Navigation, jump, rebind, archive, inspect. The first interactive surface beyond the CLI; subscribe-driven updates exercised end to end.

#### 0.9.0 — Zellij Plugin UI (M8)

Plugin shell that mirrors tree navigation inside Zellij. Routes through the daemon proto. Proves the subscribe-driven UI emergent capability with a second renderer; surfaces any proto omissions that only a second client could expose.

#### 0.10.0 — Remote (M9)

SSH-tunneled daemon. `--dir` semantics across hosts. Remote state authoritative on the remote machine. The local/remote symmetry principle gets tested against real network conditions; anything in the proto that assumed locality gets fixed.

#### 0.11.0 — Time-Travel (M14)

Snapshot timeline in the tree UI. `restore --at <snapshot>` branches a workspace from any past state. Snapshot diff in the UI. Falls out of the 0.5.0 foundation; if it does not, the foundation needed work and we now know.

#### 0.12.0 — Search (M15)

Cross-artifact search index over transcripts, files, pane titles, workspace names, and snapshot manifests. `muxon search <query>` returns results addressable back to the snapshot, pane, or turn that produced them.

#### 0.13.0 — Federation (M16)

`remote-as-link` as a first-class workspace edge. One tree spans multiple hosts under one navigation surface. Cached projections keep the tree browsable when a host is offline.

#### 0.14.0 — Aigent Transcript Replay (M17)

`transcript-replayable` formalized as a fidelity tier. Replay Codex or Claude transcripts against a fresh workspace with per-turn attribution. Aigent context becomes truly first-class state.

#### 0.15.0 — Bundles and Headless (M19, M20)

Workspace bundles (export/import). Rsync-safe event log and blob store layout. Fresh-host restore from a bundle alone. Batch mode for daemon and CLI; smoke-test mode; CI-friendly verbs with structured exit codes.

#### 0.16.0+ — Additional Backends

More `Mux` backends (`resurreccion-tmux`, `resurreccion-kitty`, `resurreccion-wezterm`) implemented against the trait. Each addition tests the trait under load; anything Zellij-specific that leaked into the trait surfaces here and gets fixed before freeze. New aigent providers and editor adapters land the same way.

### 0.N.0 → 0.M.0 — Stabilization

By the time the major adapter families and emergent capabilities have shipped, the API surfaces will have absorbed real use. The stabilization phase is hardening rather than feature work.

Each release in this phase focuses on:

- closing every known issue in the freeze-candidate surfaces (proto, CLI, snapshot format, store schema, event-type taxonomy in `resurreccion-core`, `Mux` trait, capability verb taxonomy, adapter trait shape)
- migration tests on synthetic upgrade paths
- conformance suites passing on every backend
- performance budgets met for every documented verb (`open` warm < 200 ms, structural restore < 2 s, snapshot create < 500 ms for a small workspace)
- a deliberate trial period of multiple consecutive releases without breaking changes before the freeze

Exit when no surface change is planned and all conformance and migration tests have been green across multiple consecutive releases.

### 1.0.0 — Freeze

The first compatible release. Ships nothing new on top of the last 0.x.0; changes only the compat policy.

**Frozen surfaces** (no breaks until 2.0.0, if ever):

- proto envelope, verbs, fields, error codes, and subscription channel
- CLI commands, flags, exit codes, and JSON output shapes
- snapshot manifest format and the content-addressing scheme
- store schema, with declared internal migration paths
- event-type taxonomy in `resurreccion-core`
- `Mux` trait and conformance suite
- capability verb taxonomy
- adapter trait shape (when promoted from internal to public)

**Compat promise:** 1.x adds adapters, backends, verbs, fields, event types, and capabilities. 1.x will not rename, remove, or semantically change anything in a frozen surface. Editions or feature flags carry any opt-in behavior change; nothing changes silently.

### 1.x.0 — Additive Compounding

1.x ships features as independent additive slices. Each slice is reversible; users on 1.0.0 upgrade to any 1.x without code changes.

#### 1.1.0+ — New Backends, Adapters, UIs, Observers

New `Mux` backends, aigent adapters, editor adapters, UIs (web, native), and observers (Grafana, OpenTelemetry exporters, replication targets) drop in through the existing trait surfaces. Each addition is a new crate; the system grows without breaking.

#### 1.x.0 — Plugin SDK (M18)

The adapter trait, capability verb taxonomy, and event-type taxonomy are promoted to a public crate with a conformance test suite. Third-party adapters drop in as subprocesses speaking the proto. The taxonomies gain their own compat policy alongside the existing frozen surfaces.

#### 1.n — Continued Compounding

2.0.0 only happens if a frozen surface needs a hard break — not as a routine release cadence. The expectation is that 1.x continues for as long as the trait surfaces remain sound. New value compounds through new crates against unchanged interfaces.

## Milestones

### Milestone 0: Repository Scaffold

- create the monorepo layout
- add placeholder crates and plugin entry point
- write the first architecture and planning documents

### Milestone 1: Control Plane Skeleton

- define the daemon socket protocol in `resurreccion-proto`
- implement `resurreccion-daemon` startup and health checks
- use `rt-events` for in-process daemon event fanout and observer decoupling
- implement `resurreccion-cli doctor`
- make `resurreccion-cli` able to connect to the daemon

### Milestone 2: Durable State

- add SQLite schema and migrations in `resurreccion-store`
- implement workspace, binding, runtime, snapshot, and event tables
- provide basic CRUD and event append APIs
- support first-run bootstrap

### Milestone 3: Directory-First Open Flow

- implement `resurreccion-dir` canonicalization
- support `muxon --dir .` semantics in `resurreccion-cli`
- create or resolve a workspace by binding key
- persist `last_opened_at`

### Milestone 4: Multiplexer Trait and Zellij Backend

- define the `Mux` trait, pane-as-resource model, layout primitives, and topology events in `resurreccion-mux`
- ship the conformance test suite that any backend must pass
- implement the trait in `resurreccion-zellij` as the first backend
- discover existing Zellij sessions
- create and attach a session for a workspace
- capture enough metadata for an initial structural restore
- integrate with the daemon rather than ad hoc shell logic
- prove the abstraction by sketching what a `resurreccion-tmux` shell would look like, even if not implemented

### Milestone 5: Autosave and Snapshots

- add timer-based saves
- add event-based saves on attach, detach, and topology changes
- define snapshot manifests and artifact storage layout
- classify restore fidelity as exact, stateful, structural, or historical

### Milestone 6: Aigents

- add common aigent metadata model
- wire in Codex and Claude adapters
- track external session IDs and resume references when available
- bind aigent state to workspaces and panes

### Milestone 7: Standalone Tree UI

- build a standalone TUI over the workspace tree
- support arbitrary nesting and movement within the workspace graph
- expose jump, rebind, archive, and inspect flows

### Milestone 8: Zellij Plugin UI

- add the thin plugin shell
- mirror tree navigation inside Zellij
- route all actions through the daemon protocol

### Milestone 9: Remote Resurreccion

- support remote daemon discovery and invocation over SSH
- make `--dir` semantics work across hosts
- keep remote state authoritative on the remote machine

### Milestone 10: Capability Negotiation

- formalize the capability verb taxonomy in `resurreccion-core`
- add adapter handshake to the proto so adapters advertise verbs and fidelity tiers on connect
- rewrite the planner to consume advertised capabilities rather than hardcoded adapter calls
- have `doctor` render a per-host capability matrix that names what the system can and cannot do here

### Milestone 11: Event Log as Source of Truth

- refactor the store so all writes append events
- rebuild workspace, runtime, and snapshot projections from the event log
- add an `events replay` admin command that drops and rebuilds projections
- prove the system can drop any projection at any time and reconstruct it without data loss

### Milestone 12: Content-Addressed Snapshot Store

- introduce the BLAKE3-based blob store under `resurreccion-store`
- migrate snapshot artifacts to content addressing
- collapse identical artifacts across snapshots automatically
- enable snapshot diff as a manifest comparison rather than a payload comparison

### Milestone 13: Pub/Sub Bus and Subscriptions

- adopt `rt-events` (path dep at `../promethea/rt-events`) as the in-process bus inside the daemon
- define every event type in `resurreccion-core` so all observers share the same `TypeId` keys
- enforce the rt-events discipline: sync callbacks return immediately, async work is enqueued via channels or spawned tasks
- add a subscription channel to the proto that projects in-process rt-events emissions to remote consumers
- write durable events to `resurreccion-store` from a dedicated subscriber, keeping the bus and the log decoupled
- have the daemon broadcast workspace, runtime, snapshot, capability, and adapter-lifecycle mutations
- migrate `events tail` and the Zellij plugin's live updates to subscriptions
- add a replication observer that mirrors events to a secondary store as a proof of leverage

### Milestone 14: Time-Travel and Snapshot Timeline

- expose snapshot history as a navigable timeline in the tree UI
- support `restore --at <snapshot>` to branch a workspace from any past state
- show structural diffs between adjacent snapshots so users can see what changed at each step
- treat the timeline as the canonical undo surface for destructive operations

### Milestone 15: Workspace Search Index

- index transcripts, file metadata, pane titles, workspace names, and snapshot manifests
- expose `muxon search` over the index with structured filters
- make every search result addressable back to the snapshot, pane, or turn that produced it
- keep the index derivable from the event log so it can be dropped and rebuilt

### Milestone 16: Multi-Host Federation

- promote `remote-as-link` to a first-class workspace edge in the store
- let a single tree span multiple hosts under one navigation surface
- handle host availability gracefully via cached projections when offline
- support cross-host snapshot references (a local snapshot may name a remote artifact)

### Milestone 17: Aigent Transcript Replay

- formalize `transcript-replayable` as a fidelity tier in the lattice
- support replaying a Codex or Claude transcript against a fresh workspace
- attribute restored artifacts to the original turn that produced them
- expose replay as a verb usable from CI for reproducible aigent workflows

### Milestone 18: Plugin SDK and Adapter Trait

- finalize the adapter trait based on observed adapter shape after M0–M17
- publish a public crate with stable types and a conformance test suite
- accept third-party adapters as drop-in subprocesses speaking the proto
- document the capability verb taxonomy as part of the public surface

### Milestone 19: Backup, Sync, and Portability

- design event log and blob store layouts so they are safe to back up and rsync
- support exporting and re-importing a workspace as a self-contained bundle
- make a workspace bundle restorable on a fresh host with no prior state
- treat bundles as the unit of cross-team workspace sharing

### Milestone 20: Headless and CI Integration

- support running the daemon and CLI in batch contexts without a TTY
- expose snapshot creation, restoration, and replay as CI-friendly verbs
- add a smoke-test mode that restores a snapshot, runs a check, captures the result, and tears down
- provide structured exit codes and machine output for every long-running operation

## Initial Command Surface

### User Commands

- `muxon open --dir .`
- `muxon open --workspace path/to/node`
- `muxon save`
- `muxon restore`
- `muxon tree`
- `muxon doctor`

### Control Commands

- `resurreccion-cli workspace ensure`
- `resurreccion-cli workspace bind`
- `resurreccion-cli runtime ensure`
- `resurreccion-cli runtime attach`
- `resurreccion-cli snapshot create`
- `resurreccion-cli snapshot restore`
- `resurreccion-cli events tail`

### Phase II Command Surface

These verbs are unlocked by milestones M10–M20. Each is implemented once in the daemon and surfaced uniformly by the CLI, the Zellij plugin, and any future UI.

- `muxon search <query>` — cross-artifact search over transcripts, files, pane titles, and snapshot manifests
- `muxon restore --at <snapshot>` — branch a workspace from any past snapshot
- `muxon diff <snapshot-a> <snapshot-b>` — structural diff between two snapshots
- `muxon replay <transcript> --into <workspace>` — replay an aigent transcript against a workspace
- `muxon bundle export <workspace>` / `muxon bundle import <path>` — portable workspace bundles
- `muxon subscribe <selector>` — long-lived subscription over the daemon bus
- `muxon capability list` — render the live capability matrix for this host
- `muxon federation add <host>` / `muxon federation list` — manage `remote-as-link` edges
- `resurreccion-cli events replay` — drop and rebuild projections from the event log
- `resurreccion-cli adapter list` / `resurreccion-cli adapter reload` — inspect and hot-reload adapter manifests
- `resurreccion-cli plan show <intent>` — render the planner DAG for an intent without executing it

## Emergent Capabilities

These are the system-level behaviors the project exists to produce. None is owned by a single crate; each one falls out of the principles, the milestones, and the way the components compose. Naming them gives every contributor a target larger than their crate.

- **Time-travel restore.** Event log plus content-addressed snapshots plus the planner make it trivial to branch a workspace from any past state, with no special "undo" subsystem.
- **Workspace templates.** Any past snapshot becomes a seed for a new workspace. Templates are not a separate construct — they are a usage of snapshots and the planner.
- **Cross-host workspace graph.** Local/remote symmetry plus `remote-as-link` produce a single navigable tree that spans hosts. The user never has to think about where a workspace lives.
- **Aigent context as first-class state.** The shared aigent model plus snapshots plus the bus mean a Claude or Codex session can be lifted into a new workspace, attributed to specific turns, replayed, or diffed against another session.
- **Live capability map.** Capability negotiation plus `doctor` produce an always-current view of what the system can do on this host with these adapters at this fidelity.
- **Cross-artifact search.** One query surfaces every related artifact across transcripts, files, panes, workspaces, and history — because the search index spans every projection of the event log.
- **Subscribe-driven UIs.** Any UI built on the proto gets realtime updates for free. New UIs do not need bespoke wiring; they just subscribe.
- **Replayable workflows.** A snapshot plus a transcript becomes a reproducible build of an interactive session, usable from CI without changes.
- **Workspace diff.** Snapshot manifests plus content addressing reduce structural diff to set arithmetic over hashes.
- **Headless workflows.** The same control plane that serves humans serves cron, CI, and remote agents. Every interactive verb has a batch mode.
- **Self-healing restore.** Partial-success reporting plus the fidelity lattice mean a degraded restore is a usable restore. The system tells the user what it could not do rather than refusing the operation.
- **Federated observability.** A replication observer plus the bus produce a cross-host event stream that any monitoring or audit tool can consume without bespoke integration.
- **Hot-swappable adapters.** Capability negotiation plus adapter subprocess supervision plus hot reload mean adapters can be added, upgraded, or replaced while the daemon is running.
- **Multiplexer portability.** The same Resurreccion logic backs Zellij today, tmux and kitty tomorrow, with no daemon, planner, or proto changes — because every multiplexer enters the system through the `resurreccion-mux` trait.

## Cross-Cutting Concerns

These constraints are owned by no single crate but bind every crate. They are how the system stays coherent as it grows.

- **Identifier strategy.** ULIDs everywhere — time-sortable, dedupable, and safe to propagate across hosts without coordination.
- **Schema evolution.** Store migrations and proto versioning are first-class. Every release declares its upgrade path and ships forward and backward compatibility tests.
- **Telemetry.** Structured events with trace IDs are threaded through every proto call. The same bus that drives subscriptions drives observability — there is no separate telemetry plane.
- **Security model.** Per-workspace capability scoping; secret redaction at snapshot time; explicit allowlists for capture surfaces (env vars, shell history, transcripts); SSH delegation rather than embedded credentials.
- **Failure isolation.** Adapter subprocesses can crash without taking the daemon down. The daemon can restart without losing event-log integrity. No single failure cascades.
- **Backwards compatibility.** Old snapshots and old event streams must remain readable. Structured deprecation lives in the proto, not in code paths sprinkled through adapters.
- **Documentation as a deliverable.** Every milestone updates the relevant document under `docs/`. The architecture, protocol, and snapshot-format documents are first-class outputs, not afterthoughts.
- **Testability.** Pure planners, content-addressed artifacts, and event-sourced state mean almost every behavior can be tested without spinning up a real runtime. End-to-end tests exist; they are the minority, not the majority.
- **Performance budget.** Each verb has a stated latency budget (e.g., `open` under 200 ms warm, `restore` under 2 s for a structural restore). Budgets are tracked and regressions are blocking.

## Non-Goals For The First Pass

- designing the final adapter trait
- full fidelity restoration for every application family
- cross-host global consensus
- a rich public plugin API beyond the internal control plane

## Acceptance Criteria For The Scaffold

- the repository is a valid Cargo workspace
- every planned crate exists at the expected path
- the planning documents explain the intended architecture
- there is a single, explicit sequence of milestones from scaffold to usable MVP
