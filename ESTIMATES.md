# Muxon — Initial Effort Estimates

**Locked:** 2026-04-18
**Status:** Baseline. Do not edit the Estimate columns after this date — add actuals below instead.

## Scope baseline (what all profiles are estimating)

Rust workspace and Resurreccion system: capture and restore shells, runtimes, editors, and aigents across local and remote hosts, with `muxon --dir .` as the canonical entry point.

**In scope for 1.0:**
- Pre-MVP foundations: daemon, store, directory binding, `Mux` trait, Zellij backend, rt-events bus, autosave, structural restore
- Stateful capture (shell cwd, command, prompt)
- Aigent family: shared model + Claude + Codex
- Editor family: Neovim + Emacs
- Capability negotiation (verb taxonomy + handshake + planner consuming the capability map)
- BLAKE3 content-addressed snapshot store
- Proto subscription channel projecting rt-events to remote consumers
- UI surfaces: standalone tree TUI + Zellij plugin renderer
- SSH-tunneled remote daemon with local/remote symmetry
- Higher-order: time-travel, search, federation (`remote-as-link`), transcript replay, workspace bundles, headless/CI mode
- Stabilization phase culminating in 1.0.0 freeze of proto, CLI, snapshot manifest, store schema, event taxonomy, `Mux` trait, capability verb taxonomy

**Explicitly out of scope for 1.0:**
- Additional `Mux` backends beyond Zellij (tmux, kitty, wezterm land in 1.x via the existing trait)
- Public plugin SDK for third-party adapters (M18 lands in 1.x once the trait shape is proven by the in-tree adapters)
- Web or native GUI surfaces
- Cross-host transactions or distributed consensus
- Federation conflict resolution beyond cached projections during host outages

## Phase plan

| # | Scope | Versions |
|---|---|---|
| 0 | Pre-MVP foundations: daemon handshake, durable state, directory binding, `Mux` trait + Zellij backend, rt-events bus, autosave, structural restore | 0.0.1 → 0.1.0 |
| 1 | Stateful capture (shell adapter) and first aigent (shared model + Claude) | 0.2.0, 0.3.0 |
| 2 | Leverage infrastructure: capability handshake, content addressing, proto subscription channel | 0.4.0, 0.5.0 |
| 3 | Adapter compounding: second aigent (Codex), editor family (Neovim, Emacs) | 0.6.0, 0.7.0 |
| 4 | UI surfaces and remote: tree TUI, Zellij plugin UI, SSH-tunneled remote | 0.8.0 → 0.10.0 |
| 5 | Higher-order capabilities: time-travel, search, federation, replay, bundles, headless | 0.11.0 → 0.15.0 |
| 6 | Stabilization and 1.0.0 freeze | 0.N.0 → 1.0.0 |

## Methodology

How these numbers were produced. Apply the same process when re-estimating remaining phases after actuals land.

### 1. Scope lock
Enumerate what's in and out of 1.0 before any estimate is written. Anything added later is a new entry in the Actuals table, not a retrofit to the Initial columns.

### 2. Phase decomposition
- Each phase has a testable exit criterion.
- No phase depends on a later phase to validate.
- 5–7 phases for a project of this size. More hides complexity, fewer hides risk.

### 3. Three profiles, defined

**Normal solo (no AI).** Anchor to comparable real-world projects and decompose by subsystem complexity. Reference points used here: Atuin (~1–2y to maturity, very small team), Zellij itself (~2y to 0.x stability, team), tmux-resurrect / tmuxinator / sesh (years of incremental work, single-author). Solo-no-AI for our scope = 1.5–2.5y total, dominated by the adapter long tail and the stabilization gate.

**Team of 3–5.** Apply Brooks discount — realistic gain vs. solo is 2–3×, not 5×. Coordination overhead eats 30–50%. Identify parallel splits along natural interface boundaries (storage/proto, mux + Zellij backend, planner + capabilities, adapters split across people, UI + remote).

**You (observed velocity).** Reused from the graphon estimate locked the same day; same operator, same calendar window. See §4.

### 4. Velocity measurement (observed-velocity profile)

Reused verbatim from `~/repos/graphon/ESTIMATES.md` (locked 2026-04-18, same operator, same window — re-measuring would produce the same numbers). Source: `git log --all` over the nous2 repo, 62 calendar days (2026-02-16 → 2026-04-18).

- **2709 commits** total across all refs (HEAD-only shows 121; most work lives on branches/worktrees).
- **45/62 active days, 17 zero-commit days (27% dropout).**
- Commits per active day: **p10=2, p25=20, p50=52, p75=70, p90=147, max=388.**
- Spread ~75× between light and sprint days; lumpy weekly pattern.
- LoC signal distorted by generated imports — commit count is the reliable proxy.

Derived multipliers:
- **Focused velocity factor** = 5–10× typical solo pace.
- **Calendar stretch factor** ≈ 1.3–1.5× (focused-time → calendar).

### 5. Compute the numbers

1. Decompose each phase into subsystem **focused-time** estimates.
2. Multiply by calendar stretch factor → **calendar estimate**.
3. Sum into MVP (Phase 0 alone) and Full (Phases 0–6) totals.
4. State each cell as a range, not a point; widen ranges for phases that contain risk compounders (see §6).

### 6. Risk compounders (do not scale)

Anything gated by **understanding** rather than **throughput** does not scale with velocity or team size. Flag explicitly and budget as flat costs. For Muxon:

- `Mux` trait shape under multi-backend load — one backend cannot validate the trait. Only tmux/kitty (post-1.0) prove it. Mitigated in Phase 0 by sketching a hypothetical `resurreccion-tmux` against the trait before declaring M4 done.
- Aigent model under multi-provider load — same shape risk. Phase 1 ships Claude alone; Phase 3 (Codex) is the validation.
- rt-events sync-callback discipline — async work in callbacks deadlocks the daemon. Vigilance cost across every adapter author, not a one-time fix.
- Proto and snapshot format lock — on-disk and on-wire format migrations are expensive. Lock both before Phase 2 ships content addressing and the subscription channel.
- SSH-tunneled remote symmetry — the network breaks assumptions that hold over a Unix socket. Bytes flow differently than calls; Phase 4 will surface anything in the proto that assumed locality.
- Stabilization gate (Phase 6) — gated by *time on the surface*, not throughput. Cannot be parallelized or AI'd faster. Multiple consecutive releases without breaks is a wall-clock requirement.
- Dropout variance — zero-commit days cluster. Long quiet stretches on a half-finished store layer are how invariants silently drift. Mitigate by keeping WIP small and rebaseable.

### 7. Update cadence

- After each phase completes, log actual calendar time in the Actuals table.
- Compute variance vs. all three profiles.
- A profile is "trusted" once variance stays within ±20% for two consecutive phases.
- If the observed-velocity profile misses by ±30% on two consecutive phases, re-measure velocity from fresh `git log --all` data and re-stretch remaining estimates.
- **Never edit the Initial estimate columns** — they are a locked baseline. All learning goes into the Calibration log and the remaining-phases forecasts.

## Initial estimates — three profiles

Calendar time, includes slack for debugging, review, off-days.

| Phase | Normal solo (no AI) | Team of 3–5 | You (observed velocity) |
|---|---|---|---|
| 0 | 4–7 months | 2–3 months | 6–9 weeks |
| 1 | 1–2 months | 3–5 weeks | 3–5 weeks |
| 2 | 2–3 months | 1–2 months | 4–6 weeks |
| 3 | 2–3 months | 1–2 months | 4–6 weeks |
| 4 | 2–4 months | 1–2 months | 4–7 weeks |
| 5 | 4–6 months | 2–3 months | 7–11 weeks |
| 6 | 3–6 months | 2–4 months | 8–12 weeks |
| **MVP (Phase 0)** | **4–7 months** | **2–3 months** | **6–9 weeks** |
| **Full (0–6)** | **18–31 months** | **10–17 months** | **8–13 months** |

## Reasoning per profile

### Normal solo (no AI)
Anchor points: Atuin reached 1.0 in roughly 18 months with a very small team. Zellij took ~2 years to reach a stable 0.x with a team. tmuxinator and sesh — single-author, similar in scope to Phase 0 alone — were years of incremental evolution. A competent solo Rust engineer with no AI assistance, building this scope (multiple adapter families, content-addressed store, capability negotiation, remote, UI surfaces, stabilization to freeze) is a 1.5–2.5 year project. Phase 0 alone is 4–7 months because daemon + store + dir binding + Mux trait + bus + autosave + structural restore is six non-trivial subsystems that must compose cleanly. Phase 5 is wide because the higher-order capabilities (time-travel, search, federation, replay) each touch every layer. Phase 6 is dominated by wall-clock — even solo, the freeze gate is "time on the surface".

### Team of 3–5
Brooks discount applies: 5 people ≠ 5× a solo engineer. Realistic gain is 2–3×. Good parallel split: one on storage/proto/daemon, one on mux/Zellij + remote, one on planner + capabilities + content addressing, one on adapters (rotating across families), one on UI surfaces + tests/ops. Coordination overhead eats 30–50%. Phase 0 shrinks to 2–3 months because storage/mux/daemon proceed in parallel against stable interfaces. Phase 5 stays wide because the higher-order capabilities have cross-cutting dependencies that resist parallelization. Phase 6 barely shrinks — the stabilization gate is wall-clock, not throughput.

### You (observed velocity)
Focused velocity is 5–10× typical solo pace; calendar stretch 1.3–1.5×. Phase 0 lands in 6–9 weeks: M1–M5 are well-understood subsystems with clear interfaces and you have written variants of most of them before. Phases 1–4 stack on the same foundation; each is 4–7 weeks of focused work spread across 4–7 weeks calendar. Phase 5 widens to 7–11 weeks because each higher-order capability requires the right foundation already to be in place — a missing primitive in Phase 2 surfaces here as a refactor. Phase 6 is the longest: the freeze gate requires multiple consecutive releases without breaks, and that is a wall-clock requirement that no velocity multiplier shortens.

## Risk compounders (do not scale with velocity or team size)

1. **`Mux` trait shape under one backend** — Zellij alone cannot validate the trait. The trait will be wrong in ways that only show up when tmux or kitty land (post-1.0). Mitigation: M4 includes sketching a hypothetical `resurreccion-tmux` implementation against the trait before declaring it done. Expect ~1 week of trait revisions during Phase 0 and another round of revisions when the second backend lands in 1.x.
2. **Aigent model under one provider** — same shape. The Claude integration in Phase 1 will encode Claude-specific assumptions; the Codex integration in Phase 3 is the test. Budget ~1 week of model rework when the leak surfaces.
3. **rt-events sync-callback discipline** — every adapter author is one mistake away from deadlocking the daemon. Cannot be solved once; must be guarded continuously through code review, lints, or a wrapper that enforces non-blocking callbacks. Flat ongoing cost.
4. **Proto and snapshot format lock** — on-disk and on-wire format migrations are expensive. The proto envelope must be right by Phase 2 (capability handshake) and the snapshot manifest must be right by Phase 2 (content addressing). Slipping this turns Phase 5 into a migration project. Budget ~1–2 weeks of format lockdown work in Phase 2.
5. **SSH-tunneled remote symmetry** — local/remote symmetry is a principle, not a property; the proto needs to actually preserve semantics over a degraded link. Phase 4 will expose every place the proto assumed locality. Expect ~1–2 weeks of careful debugging against partition and latency scenarios.
6. **Stabilization gate (Phase 6)** — the freeze requires multiple consecutive releases without breaking changes against real users. This is gated by *time on the surface*, not by throughput. No team size or AI assistance shortens it below the time it takes for surfaces to absorb real use. Budget the calendar minimum even if focused-time looks small.
7. **Dropout variance** — zero-commit days cluster. Long quiet stretches on half-finished store, planner, or proto layers are how invariants silently drift. Mitigate by keeping WIP small and rebaseable.

## Actuals (fill as phases complete)

| Phase | Est. focused | Est. calendar | Started | Finished | Actual calendar | Variance | Notes |
|---|---|---|---|---|---|---|---|
| 0 | 4–6 weeks | 6–9 weeks | | | | | |
| 1 | 2–4 weeks | 3–5 weeks | | | | | |
| 2 | 3–5 weeks | 4–6 weeks | | | | | |
| 3 | 3–5 weeks | 4–6 weeks | | | | | |
| 4 | 3–5 weeks | 4–7 weeks | | | | | |
| 5 | 5–8 weeks | 7–11 weeks | | | | | |
| 6 | 4–6 weeks (focused) + wall-clock | 8–12 weeks | | | | | |

## Calibration log

After each phase completes, add an entry here with:
- Actual vs. each of the three estimate profiles
- What surprised us (over or under)
- Whether remaining phase estimates need adjustment

(First entry lands after Phase 0.)
