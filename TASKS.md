# TASKS — Parallel Agentic Execution Plan

Optimized replacement of the serial per-release decomposition. Collapses
0.0.1 → 0.1.0 into three sprints so Phase 0 ships as a single parallel push.

## TL;DR

One serial **Interface Lockdown** sprint (≈1 operator-day) publishes every
type, trait, proto envelope, and event definition the rest of the system
composes against. Once locked, **10 independent lanes** run as parallel
agents (≈2–3 wall-clock days of the slowest lane). A final **Integration**
sprint merges, smoke-tests, and tags 0.1.0. Estimated wall-clock:
**≈1 week** vs. the 6–9 weeks the serial plan budgeted.

## The bet

The leverage principle that makes this tractable is already in the plan:
**one seam per concern** (the `Mux` trait, the proto envelope, the event
taxonomy, the fidelity lattice, the store API). If every seam is specified
before implementation starts, every implementation lane is a pure function
from "the seam" to "a crate". Lanes don't need to talk to each other.

If the seams leak, the parallelism collapses. Sprint 0 is the only place
that decides what a seam's shape is. Once Sprint 1 starts, seam changes
are stop-the-world events, not lane-local decisions.

## Dep swaps from the naive plan

| Concern | Naive | Picked | Why |
|---|---|---|---|
| SQL | rusqlite + refinery | **rusqlite (bundled) + hand-rolled migrations** | one file, 30 LoC, no dep churn |
| RPC framing | custom bytes | **`tokio-util::LengthDelimitedCodec` + `serde_json`** | drops protocol-reinvention |
| RPC dispatch | custom verb table | **keep** (10 verbs; tarpc is overkill) | smaller surface for agents |
| IDs | string | **`ulid` crate** | time-sortable, host-safe |
| Hash | — | **`blake3`** (added now) | cheap now, load-bearing at 0.5.0 |
| XDG paths | manual env chain | **`directories`** | delete 30 LoC |
| Paths | `std::path` | **`camino::Utf8Path`** | UTF-8 by construction, better UX |
| Git detection | shell out | **`git2`** | robust, no process spawn |
| Errors | ad hoc | **`thiserror` (libs) + `anyhow` (bins)** | standard split |
| Tracing | retrofit later | **`tracing` + `tracing-subscriber` from day 0** | free trace IDs, no retrofit cost |
| Test runner | `cargo test` | **`cargo nextest`** | 2–3× faster, parallel-safe |
| CLI completions | hand-written | **`clap_complete`** | free from clap schema |

All added to `Cargo.toml` in Sprint 0, not per lane. Lanes never touch
workspace deps.

## Sprint 0 — Interface Lockdown

**Operator:** 1 human (or one primary Claude session). Serial.
**Budget:** ≈1 day. Shorter if decisions are already cached.

This sprint ships **no business logic**. Every deliverable is either a type,
a trait signature, a constant, or a Cargo.toml edit.

### 0.1 — Workspace tooling

- `Cargo.toml`: add every dep from the table above as a `workspace.dependencies`
  entry with pinned versions.
- `rust-toolchain.toml` pinning a stable version.
- `rustfmt.toml` + `.cargo/config.toml` with `target-dir`.
- `clippy` denies listed in `[workspace.lints]`.
- CI jobs (all required for merge): `cargo fmt --check`, `cargo clippy -- -D warnings`,
  `cargo nextest run --workspace`, `cargo build --workspace`.
- Branch protection on `main`: require 1 approval + all CI jobs green; disallow
  direct pushes; enable auto-merge.
- Repo labels `lane:A`..`lane:I` for lane tagging.

### 0.2 — `resurreccion-core` types

- ULID newtypes: `WorkspaceId`, `RuntimeId`, `SnapshotId`, `PaneId`,
  `SessionId`, `TabId`, `EventId`, `BlobId`.
- `BindingKey` newtype (BLAKE3 of canonical path + optional git identity).
- `ErrorCode` enum: `NotReady`, `NotFound`, `Conflict`, `Internal`,
  `Unsupported`, `VersionMismatch`, `Timeout`, `Busy`.
- `RestoreFidelity` enum + `PartialRestore` struct.
- Event taxonomy as structs (one type = one `TypeId` = one rt-events key):
  `WorkspaceOpened`, `WorkspaceClosed`, `RuntimeAttached`, `RuntimeDetached`,
  `PaneOpened`, `PaneClosed`, `FocusChanged`, `LayoutChanged`,
  `SnapshotCreated`, `SnapshotRestored`.
- Marker trait `DaemonEvent` implemented by every event.

### 0.3 — `resurreccion-proto` envelope

- `Envelope { id, verb, trace_id, body }` with `#[serde(tag="ok")]` success/
  error split consistent with the existing `docs/protocol.md` shape.
- `PROTO_VERSION` constant, bumped via const assert on each schema change.
- Verb-name constants for every Sprint-1 verb (single source of truth for
  Lane B and Lane C).
- `Client` skeleton (connect + call + stream-call) — signatures only; impl
  is Lane B1.
- Request/response types for every verb, serde-derived.

### 0.4 — `resurreccion-mux` trait

- Trait `Mux` with signatures only: `discover`, `create`, `attach`,
  `capture`, `apply_layout`, `send_keys`, `subscribe_topology`.
- Types: `LayoutSpec`, `LayoutCapture`, `TopologyEvent`, `Capability` flags.
- `MuxError` with retry/fatal discrimination.
- Conformance test module `resurreccion-mux::conformance::run<M: Mux>` —
  test bodies can be stubs that panic "not implemented"; Lane D fills them in.

### 0.5 — `resurreccion-store` API

- `Store` struct (no impl yet) + trait-shaped method signatures for each
  CRUD surface. Lane A fills in bodies.
- `migrations/001_initial.sql` text agreed and committed (but not yet
  executed by code).

### 0.6 — `resurreccion-planner` types

- `Plan`, `PlanNode`, `NodeId` types + `execute` signature.
- Capability-verb string constants consumed by plan nodes.

### 0.7 — Second-backend sketch (the leverage gate)

- `docs/mux-trait-sketch.md`: map every `Mux` method to a hypothetical
  tmux implementation. If anything is awkward, revise the trait **now**,
  not after Lane D ships. This is the single most load-bearing hour of
  Sprint 0.

### Sprint 0 exit

- `cargo build --workspace` green.
- `cargo nextest run --workspace` green (all tests are compile-check stubs).
- CI pipeline green.
- Every Sprint-1 lane description below compiles against the locked API.

---

## Sprint 1 — Parallel lanes

**Agents:** 5–10 independent Claude/human sessions.
**Budget:** ≈2–3 days wall-clock (≈1–2 focused days per lane).

Each lane below is a **drop-in prompt** for `Agent(subagent_type="general-purpose")`
given Sprint 0 is complete. Lanes do not share branches; each PRs into `main`
when done. Order of merge does not matter for A/C/D/E/F/G; B1 should land
before B2/B3 to shorten their review context.

### Lane workflow (per agent)

Every lane runs through the same four-stage pipeline. No human review in the
middle; the only human touch-points are Sprint 0 seam decisions and Sprint 2
final merge.

**1. Implementation agent** — `Agent(subagent_type="general-purpose", isolation="worktree")`

- Prompt: the lane spec below, verbatim, plus "the PR must also pass Lane
  Review (see `TASKS.md`)".
- Works in an isolated worktree (the Agent tool creates and returns it).
- Commits to branch `lane/<ID>-<slug>` (e.g., `lane/A-store`,
  `lane/B1-daemon-runtime`).
- Opens a PR with `gh pr create`, tagging the `lane:<ID>` label and linking
  the `TASKS.md` anchor.
- Enables auto-merge on the PR: `gh pr merge <num> --auto --squash`.

**2. Review agent** — fresh `Agent(subagent_type="general-purpose")`, no
worktree, no shared memory with (1)

- Prompt: the lane spec + `gh pr diff <num>` + the repo's seam files
  (`resurreccion-core`, `resurreccion-proto`, `resurreccion-mux`,
  `resurreccion-store` public APIs).
- Checks against the lane spec's **Deliverables**, **Done when**, and
  **Out of scope** bullets; flags seam violations (imports into crates the
  lane does not own), missing tests, lane-scope creep, obvious bugs,
  rt-events callback discipline, unwrap/panic in non-test paths.
- Emits verdict as JSON: `{ "verdict": "approve" | "request_changes",
  "comments": [ ... ] }`.
- On `approve`: `gh pr review <num> --approve`.
- On `request_changes`: `gh pr review <num> --request-changes --body <comments>`;
  orchestrator re-spawns (1) with the comments appended to its prompt.

The review agent is a fresh model instance with no context from (1) — same
model, independent conversation. This is the cheapest practical substitute
for human review; the review is judgment against the lane spec, not a
rubber stamp of the implementer's intent.

**3. CI gate**

- GitHub Actions runs `cargo fmt --check`, `cargo clippy -- -D warnings`,
  `cargo nextest run --workspace`, `cargo build --workspace`.
- Every job is required by branch protection on `main`.

**4. Merge**

- Branch protection: `main` requires 1 approval **and** all CI jobs green.
- Auto-merge (enabled at PR creation) squash-merges the branch the moment
  both gates are satisfied. No human in the loop.

**Failure loop**

- CI red → orchestrator re-spawns (1) with the failing job's log excerpt
  in the prompt; worktree is reused.
- Review `request_changes` → orchestrator re-spawns (1) with the comments.
- Iteration budget: **3 loops per lane** before escalating to a human.
  A lane that exceeds this is a signal the Sprint 0 seams were wrong for
  this lane's problem — not a signal to keep iterating.

**Orchestrator**

- One top-level session (human or a dedicated orchestrator agent) that:
  spawns implementation + review agents per lane, watches PR statuses
  (`gh pr view <num> --json reviewDecision,statusCheckRollup`), re-spawns
  on failure, reports lane status to the Sprint 1 dashboard.
- Orchestrator runs the lanes in parallel by issuing multiple `Agent`
  calls in one message; it does not serialize unless the DAG says so.

### Lane A — Store implementation

- **Owns:** `crates/resurreccion-store/`.
- **Input:** `Store` signatures from Sprint 0.5; `migrations/001_initial.sql`.
- **Deliverables:**
  - `Store::open(path)` runs migration, returns `Arc<Store>`.
  - Every CRUD method implemented against `rusqlite::Connection` behind
    a `Mutex`.
  - `events.append(kind, workspace_id, payload_json)` + `events.tail_from(id)`.
  - Unit tests per table: insert → get → list → update.
- **Done when:** `cargo nextest run -p resurreccion-store` green.
- **Out of scope:** daemon wiring, event semantics, content addressing.

### Lane B1 — Daemon runtime

- **Owns:** `crates/resurreccion-daemon/src/{main,runtime,dispatch}.rs`.
- **Input:** proto envelope + verb constants.
- **Deliverables:**
  - Unix socket bind at XDG path via `directories`.
  - Accept loop spawning a tokio task per connection.
  - Verb dispatch `HashMap<&'static str, Arc<dyn Handler>>` with registration
    helper.
  - `handshake` and `doctor.ping` handlers.
  - `SIGTERM`/`SIGINT` graceful shutdown (2s drain + socket unlink).
  - Single-instance guard (connect before bind).
- **Done when:** launching the binary + running `muxon doctor` from Lane C
  returns exit 0.
- **Out of scope:** any verb beyond handshake/ping (B2/B3 own those).

### Lane B2 — Workspace verbs

- **Owns:** `crates/resurreccion-daemon/src/verbs/workspace.rs`.
- **Input:** B1 dispatch helper + Lane A store + Lane E dir binding.
- **Deliverables:** handlers for `workspace.create`, `workspace.get`,
  `workspace.list`, `workspace.resolve_or_create`, `workspace.open`
  (attach-or-create via `Arc<dyn Mux>`).
- **Done when:** integration test in Lane I exercises each verb end-to-end.
- **Out of scope:** snapshot logic, event emission (G subscribes but B2 just
  calls `daemon.emit`).

### Lane B3 — Snapshot + events.tail verbs

- **Owns:** `crates/resurreccion-daemon/src/verbs/{snapshot,events}.rs`.
- **Input:** B1 + A + F planner + D Zellij + G bus.
- **Deliverables:** `snapshot.create`, `snapshot.restore`, `events.tail`
  (streaming response — writes each event as a framed envelope until client
  disconnects).
- **Done when:** Lane I test saves → kills → restores a Zellij layout via
  these verbs.

### Lane C — CLI

- **Owns:** `crates/resurreccion-cli/`.
- **Input:** proto envelope + verb constants + `Client` skeleton.
- **Deliverables:**
  - `muxon` binary with `clap` derive.
  - Subcommands: `doctor`, `save`, `restore`, `tree` (stub), `events tail`.
  - Top-level `--dir <path>` (not a subcommand) → calls
    `workspace.resolve_or_create` + `workspace.open`.
  - `workspace {create,get,list}` subcommands.
  - `--json` global flag switches every output.
  - `clap_complete` generation in `build.rs`.
  - Exit codes: 0 ok, 1 timeout, 2 version mismatch, 3 socket missing.
- **Done when:** `muxon doctor` against a running daemon exits 0; all
  subcommands compile and respond sensibly.
- **Out of scope:** server-side verb logic.

### Lane D — Zellij backend

- **Owns:** `crates/resurreccion-zellij/`.
- **Input:** `Mux` trait + conformance suite.
- **Deliverables:**
  - `impl Mux for ZellijMux` via `zellij` CLI invocation.
  - `discover`: parse `zellij list-sessions`.
  - `create`: generate a layout KDL, `zellij --layout`.
  - `attach`: `zellij attach`.
  - `capture`: parse `zellij action dump-layout`.
  - `subscribe_topology`: 1s polling task emitting deltas as
    `TopologyEvent`.
  - Capability flags advertised: `PluginEmbedding=false` for 0.1.0.
  - Conformance suite passes.
- **Done when:** `cargo nextest run -p resurreccion-zellij` green, including
  conformance suite.
- **Out of scope:** plugin UI, IPC socket (deferred to ≥0.5.0).

### Lane E — Directory binding

- **Owns:** `crates/resurreccion-dir/`.
- **Input:** `BindingKey` type.
- **Deliverables:**
  - `canonicalize(path) -> Utf8PathBuf`.
  - `detect_git(path) -> Option<{ remote, worktree_name }>` via `git2`.
  - `compose_binding_key(canonical, git, scope) -> BindingKey` (BLAKE3).
  - `scope`: enum `PathScoped | RepoScoped`; default `RepoScoped` when inside
    a git repo.
  - Decision note in `docs/architecture.md`.
  - Unit tests: symlink identity, distinct paths, repo clone identity.
- **Done when:** `cargo nextest run -p resurreccion-dir` green.

### Lane F — Planner

- **Owns:** `crates/resurreccion-planner/`.
- **Input:** `Plan` types + capability-verb constants + `Mux` trait.
- **Deliverables:**
  - `plan_capture(workspace_state, capabilities) -> Plan`.
  - `plan_restore(manifest, capabilities) -> Plan`.
  - `execute(plan, mux, store) -> PartialRestore`.
  - For 0.1.0 the plan is one node (`layout` verb) but plumbing is full-DAG
    shaped — 0.4.0's capability handshake drops in without planner rewrite.
  - Unit tests: empty state → empty plan; one pane → one-node capture plan;
    one manifest → one-node restore plan; dry-run mode returns the plan
    without executing.
- **Done when:** `cargo nextest run -p resurreccion-planner` green.

### Lane G — Bus + subscribers

- **Owns:** `crates/resurreccion-daemon/src/bus.rs` + `src/subscribers/`.
- **Input:** core event taxonomy + Lane A store.
- **Deliverables:**
  - `rt_events::Bus` held by `Daemon`.
  - `daemon.emit(event)` helper.
  - Durable subscriber: every event type → channel → tokio task →
    `store.events.append`.
  - Autosave subscriber: on `RuntimeAttached`/`Detached`/`LayoutChanged` →
    debounced (5s per workspace) → calls `snapshot.create` via internal verb.
  - `proto::SubscriptionHub` re-broadcasting bus events to streaming
    `events.tail` consumers (B3 plugs in here).
  - Compile-time assertion / lint comment: callbacks must not block
    (channel-send only).
- **Done when:** tests in Lane I observe events flowing end-to-end.

### Lane I — Integration, smoke, docs, release

- **Owns:** `tests/` at the workspace root + `docs/` + `scripts/`.
- **Input:** all other lanes merged.
- **Deliverables:**
  - `tests/doctor.rs`: spawn daemon + `muxon doctor` → exit 0.
  - `tests/round_trip.rs`: workspace create → list → resolve identity.
  - `tests/restore.rs`: `muxon --dir` → open Zellij → save → kill → `muxon --dir` →
    restore → assert layout.
  - `tests/events.rs`: open a pane while `events tail` is running → event
    observed within 2s.
  - `scripts/smoke.sh`: the above as a user-runnable shell walkthrough.
  - `docs/{architecture,protocol,snapshot-format}.md` refreshed to describe
    what shipped.
  - Workspace version bump to `0.1.0`; release tag; short migration note.
- **Done when:** `cargo nextest run --workspace` green locally and in CI;
  smoke script runs clean on a fresh host.

---

## Sprint 2 — Integration and 0.1.0 release

**Operator:** 1 human. Serial. Budget: ≈1 day.

- Merge order: Sprint 0 → A, E, F, D, G (independent) → B1 → B2, B3 → C → I.
- Resolve any cross-lane rustfmt/clippy bikesheds (Sprint 0 lints should
  have caught these already).
- Run Lane I smoke on a fresh container.
- Tag `v0.1.0`. Publish release notes.

## Dependency DAG (for scheduling agents)

```
Sprint 0
  └── (unlocks all lanes)
         A ─┬──────────┐
         E ─┤          │
         F ─┼──► B3 ───┤
         D ─┤          │
         G ─┘          │
                        │
         B1 ──► B2 ────┤
                        ├──► I ──► Sprint 2 ──► 0.1.0
         C ─────────────┘
```

Critical path: Sprint 0 → B1 → B3 → I → Sprint 2 (≈5 days).
Every other lane fits inside that window.

## Risk register

- **Seam drift mid-Sprint-1.** Any lane that discovers a needed extension
  raises a "seam change" PR that pauses dependent lanes. Budget 1 such event.
- **Zellij CLI brittleness.** `zellij action dump-layout` output shape may
  change between Zellij versions. Pin a specific Zellij release for 0.1.0
  and lock it in `docs/architecture.md`.
- **rt-events discipline.** Every subscriber callback must be non-blocking.
  Lane G ships a lint/comment; every lane emitting events must pass review.
- **Merge conflict storm.** Mitigated by: disjoint crate ownership per lane,
  all workspace deps pinned in Sprint 0, agents never touching shared files
  outside their lane.
- **Integration test flakiness on Zellij spawn.** Tests run in a dedicated
  temp HOME + XDG dirs; daemon socket paths per test process. Lane I owns
  the harness.
- **Review agent false-approves a broken PR.** Mitigations: CI catches
  compile/test/format issues regardless of the review verdict; branch
  protection requires both gates; the 3-iteration budget escalates
  persistent cases to a human. Review quality is upper-bounded by the
  specificity of the lane spec — vague Done-when criteria produce vague
  reviews.
- **Worktrees accumulate.** The Agent tool cleans worktrees with no
  changes; others persist with the lane branch. Orchestrator runs
  `git worktree prune` after successful merge; stale worktrees on failed
  lanes are evidence for human review.

## Speedup estimate

| Profile | Serial plan | Parallel plan |
|---|---|---|
| Solo no AI | 4–7 months | not applicable (no parallel agents) |
| Team of 3–5 | 2–3 months | 2–3 weeks |
| Observed velocity | 6–9 weeks | **≈1 week wall-clock** |

The parallel plan's speedup is a function of how many lanes are run
concurrently and how clean the seams are. Best case with 10 agents and
zero seam drift: 4–5 days. Realistic with one seam revision and integration
bugs: 7–10 days.

## Depth policy (updated)

Sprint 0 + Sprint 1 collectively decompose **all** of Phase 0. Beyond 0.1.0,
the same pattern applies per phase:

1. **Phase sprint 0 (serial):** define the seams that phase changes or adds
   — event types, proto verbs, capabilities, trait extensions.
2. **Phase sprint 1 (parallel):** 5–10 lanes against the seams.
3. **Phase sprint 2 (serial):** integrate, smoke, release.

Decompose only the phase currently entering sprint 0. Phases further out
stay at milestone granularity in `IMPLEMENTATION_PLAN.md`.
