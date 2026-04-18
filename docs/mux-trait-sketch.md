# Mux Trait: Second-Backend Sketch (tmux)

Maps every [`Mux`](../crates/resurreccion-mux/src/lib.rs) trait method to a hypothetical
tmux backend implementation. This sketch documents how tmux CLI semantics could satisfy
the trait contract, and flags where the fit is awkward.

## Overview

The Mux trait defines a common interface for multiplexer backends to support Resurreccion
workflow orchestration:

1. **discover()** — list all sessions  
2. **create()** — spawn a new session with an initial layout  
3. **attach()** — foreground an existing session  
4. **capture()** — snapshot the current layout  
5. **apply_layout()** — restructure a session to match a layout spec  
6. **send_keys()** — send keystrokes to the focused pane  
7. **subscribe_topology()** — stream topology change events  
8. **capabilities()** — advertise backend feature flags  

This sketch assumes tmux ≥ 3.0 and uses `tmux` CLI exclusively (no direct socket access).

---

## 1. discover() → Vec\<String\>

**Trait contract:**  
List all currently running sessions managed by this backend.

**tmux mapping:**

```sh
tmux list-sessions -F '#{session_name}'
```

Parses stdout, one session name per line. Strip trailing whitespace.

**Edge cases & awkward fits:**

- **Empty state**: tmux returns exit code 1 if no sessions exist. Treat as `Ok(vec![])`.
- **No error granularity**: tmux doesn't distinguish "backend not running" from "no sessions."
  If `tmux list-sessions` fails entirely, assume the tmux server is dead → return
  `MuxError::NotAvailable("tmux server not running")`.
- **Session namespace collisions**: tmux's session namespace is global per server.
  Muxon cannot "own" a subset of sessions — any session on the server is visible.
  Future: might need a prefix convention (e.g., "muxon:*") to filter.

**Implementation notes:**

```rust
fn discover(&self) -> Result<Vec<String>, MuxError> {
    let output = Command::new("tmux")
        .args(&["list-sessions", "-F", "#{session_name}"])
        .output()?;
    
    if !output.status.success() && output.stderr.contains("no server running") {
        return Err(MuxError::NotAvailable("tmux server not running".into()));
    }
    
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.trim().to_string())
        .collect())
}
```

---

## 2. create(session_name, layout) → Result\<(), MuxError\>

**Trait contract:**  
Create a new session with the given name and apply an initial layout (panes & tabs).

**tmux mapping:**

1. **Create the base session:**
   ```sh
   tmux new-session -d -s <session_name> -x 120 -y 40
   ```
   `-d` for detached (don't attach), `-x/-y` to set initial window size.

2. **For each pane in layout.panes:**
   - First pane: already exists in the default window. Configure it:
     ```sh
     tmux send-keys -t <session>:0 'cd <cwd> && <cmd>' Enter
     ```
   - Subsequent panes: split the window:
     ```sh
     tmux split-window -t <session>:0 -h  # or -v for vertical
     tmux send-keys -t <session>:0.<pane_idx> 'cd <cwd> && <cmd>' Enter
     ```

3. **For each tab in layout.tabs:**
   Create new windows:
   ```sh
   tmux new-window -t <session> -n <tab_name>
   ```

**Edge cases & awkward fits:**

- **CRITICAL: LayoutSpec is underspecified.**  
  The trait provides `panes: Vec<PaneSpec>` and `tabs: Vec<String>` but:
  - No geometric layout info (pane positions, sizes, split directions)
  - No mapping between panes and tabs (which pane lives in which tab?)
  - No command to execute (PaneSpec has no "command" field; only cwd + title)
  
  **Current assumption**: panes are sequential, layout is left-to-right splits in tab 0,
  and subsequent tabs are empty. This is a leveraging constraint: a second backend must
  either accept this limitation or expand LayoutSpec.

- **Window size negotiation**: tmux windows have a fixed size. The sketch picks 120x40
  arbitrarily. A real implementation must negotiate size with the terminal or query it
  from the calling environment.

- **No pane focus specification**: tmux doesn't let you set focus during session creation.
  The focused pane will be the last one created (or first, depending on split order).
  If PaneSpec needs a "focused" flag, we'd split in a specific order.

- **Tab = Window mismatch**: tmux calls them "windows," not "tabs." The naming is a
  Zellij-ism that leaks into the trait. Works fine in practice.

- **Initial command execution**: If PaneSpec.cwd is set, we send `cd <cwd>` as a keystroke.
  But PaneSpec has no "command" field — what should execute? The sketch assumes the
  user's shell default (no command sent), just the working directory.

**Implementation notes:**

```rust
fn create(&self, session_name: &str, layout: &LayoutSpec) -> Result<(), MuxError> {
    // Check if session exists
    let exists = Command::new("tmux")
        .args(&["has-session", "-t", session_name])
        .status()?
        .success();
    if exists {
        return Err(MuxError::SessionExists(session_name.into()));
    }
    
    // Create detached session
    Command::new("tmux")
        .args(&["new-session", "-d", "-s", session_name])
        .status()?;
    
    // Configure first pane
    if let Some(first_pane) = layout.panes.first() {
        if let Some(cwd) = &first_pane.cwd {
            Command::new("tmux")
                .args(&["send-keys", "-t", &format!("{}:0", session_name),
                        &format!("cd {}", shell_escape(cwd)), "Enter"])
                .status()?;
        }
    }
    
    // Create additional panes as splits
    for (idx, pane) in layout.panes.iter().enumerate().skip(1) {
        Command::new("tmux")
            .args(&["split-window", "-t", &format!("{}:0", session_name), "-h"])
            .status()?;
        
        if let Some(cwd) = &pane.cwd {
            Command::new("tmux")
                .args(&["send-keys", "-t", &format!("{}:0.{}", session_name, idx),
                        &format!("cd {}", shell_escape(cwd)), "Enter"])
                .status()?;
        }
    }
    
    // Create tabs (windows)
    for (idx, tab_name) in layout.tabs.iter().enumerate() {
        Command::new("tmux")
            .args(&["new-window", "-t", session_name, "-n", tab_name])
            .status()?;
    }
    
    Ok(())
}
```

---

## 3. attach(session_name) → Result\<(), MuxError\>

**Trait contract:**  
Attach the current process to an existing session.

**tmux mapping:**

```sh
tmux attach-session -t <session_name>
```

This replaces the current terminal with the tmux client, blocking until the user detaches.

**Edge cases & awkward fits:**

- **Blocking operation**: `attach` is fundamentally synchronous and doesn't return until
  the user detaches. From a CLI perspective, this is correct. But if the caller is a
  daemon, this blocks the event loop. Zellij's CLI also blocks, so the trait matches
  expected behavior.

- **Terminal state**: tmux modifies the terminal (raw mode, alternate screen). The calling
  process must handle cleanup if attach fails mid-operation.

- **No pane selection**: `attach` always focuses the last-focused pane. If we need to
  attach with a specific pane in focus, we'd send a keystroke immediately after attach
  starts, which is not reliable. This is a trait limitation, not a tmux one.

**Implementation notes:**

```rust
fn attach(&self, session_name: &str) -> Result<(), MuxError> {
    let status = Command::new("tmux")
        .args(&["attach-session", "-t", session_name])
        .status()?;
    
    if !status.success() {
        return Err(MuxError::SessionNotFound(session_name.into()));
    }
    
    Ok(())
}
```

---

## 4. capture(session_name) → Result\<LayoutCapture, MuxError\>

**Trait contract:**  
Snapshot the current layout: session name, list of panes, list of tabs, capabilities.

**tmux mapping:**

1. **List windows (tabs):**
   ```sh
   tmux list-windows -t <session> -F '#{window_name}'
   ```

2. **List panes:**
   ```sh
   tmux list-panes -t <session> -F '#{pane_id}|#{pane_current_path}|#{pane_title}'
   ```
   Output: `%0|/home/user|zsh` (one per line)

3. **Detect capabilities:**
   - `PLUGIN_EMBEDDING`: tmux does not support embedding external UIs → false
   - `COPY_MODE`: tmux has copy mode → true
   - `SCROLLBACK_TEXT`: scrollback is accessible via `tmux capture-pane` → true

**Edge cases & awkward fits:**

- **Pane title ambiguity**: `#{pane_title}` is a user-settable string, not necessarily
  the command running in the pane. To get the actual command, we'd parse `/proc/<pid>`
  on Linux, which is not portable. The sketch captures the title as-is.

- **CWD availability**: `#{pane_current_path}` works on macOS and Linux but relies on
  the shell reporting PWD to tmux (not guaranteed). If unavailable, fallback to empty string.

- **Pane ordering**: tmux list-panes returns panes in creation order, not visual order.
  The capture doesn't preserve geometric layout, only pane list. This is acceptable for
  Resurreccion's use case (diffing state, not rendering UI).

- **Tab count**: window count is not the same as "tab count" in Zellij. tmux windows are
  more like split panes. But for our purposes, list-windows ≈ tabs.

**Implementation notes:**

```rust
fn capture(&self, session_name: &str) -> Result<LayoutCapture, MuxError> {
    // Get tabs (windows)
    let tabs_output = Command::new("tmux")
        .args(&["list-windows", "-t", session_name, "-F", "#{window_name}"])
        .output()?;
    
    if !tabs_output.status.success() {
        return Err(MuxError::SessionNotFound(session_name.into()));
    }
    
    let tabs: Vec<String> = String::from_utf8_lossy(&tabs_output.stdout)
        .lines()
        .map(|s| s.trim().to_string())
        .collect();
    
    // Get panes
    let panes_output = Command::new("tmux")
        .args(&["list-panes", "-t", session_name, 
                "-F", "#{pane_id}|#{pane_current_path}|#{pane_title}"])
        .output()?;
    
    let panes: Vec<PaneSpec> = String::from_utf8_lossy(&panes_output.stdout)
        .lines()
        .map(|line| {
            let parts: Vec<&str> = line.split('|').collect();
            PaneSpec {
                id: parts.get(0).unwrap_or(&"").to_string(),
                cwd: parts.get(1).map(|s| s.to_string()),
                title: parts.get(2).map(|s| s.to_string()),
            }
        })
        .collect();
    
    Ok(LayoutCapture {
        session_name: session_name.to_string(),
        panes,
        tabs,
        capabilities: self.capabilities(),
    })
}
```

---

## 5. apply_layout(session_name, layout) → Result\<(), MuxError\>

**Trait contract:**  
Restructure an existing session to match a new layout spec.

**tmux mapping:**

This is the most complex operation. High-level approach:

1. **Destroy all existing panes/windows** (except the initial window):
   ```sh
   tmux kill-window -t <session>:1
   tmux kill-window -t <session>:2
   # ... repeat for all windows
   ```

2. **Reconfigure the first window** with new panes (same as step 2 of `create()`).

3. **Create new windows** for additional tabs.

Alternatively, **non-destructive approach:**

1. Diff the current layout (from `capture()`) against the desired layout.
2. Create/destroy panes and windows as needed.
3. Rearrange splits if geometric info were available (currently not).

**Edge cases & awkward fits:**

- **Destructive reshape**: Tmux doesn't support "morphing" an existing window layout.
  We must kill and recreate. Any running commands or pane state in the old layout is
  lost. This is a significant constraint: Resurreccion cannot apply a layout without
  losing work.
  
  **Potential mitigation**: Add a "soft apply" that only creates new panes, never
  destroys. But then the layout diverges from the spec. This is a trait limitation,
  not tmux's fault — the trait should specify "destructive" semantics.

- **No pane replacement**: If we have pane A and the new layout calls for pane B at
  the same position, tmux doesn't let us "replace" A with B. We must kill A and
  create B, losing its state.

- **Window/tab ambiguity**: If layout.tabs differs from current window count, we
  add/remove windows. But which windows correspond to which tabs? Without stable IDs,
  this is best-effort.

**Implementation notes:**

```rust
fn apply_layout(&self, session_name: &str, layout: &LayoutSpec) -> Result<(), MuxError> {
    // Kill all windows except :0
    let windows_output = Command::new("tmux")
        .args(&["list-windows", "-t", session_name, "-F", "#{window_index}"])
        .output()?;
    
    let window_indices: Vec<usize> = String::from_utf8_lossy(&windows_output.stdout)
        .lines()
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    
    for idx in window_indices {
        if idx > 0 {
            let _ = Command::new("tmux")
                .args(&["kill-window", "-t", &format!("{}:{}", session_name, idx)])
                .status();
        }
    }
    
    // Kill all panes in window 0 except the first
    let panes_output = Command::new("tmux")
        .args(&["list-panes", "-t", &format!("{}:0", session_name), "-F", "#{pane_index}"])
        .output()?;
    
    let pane_indices: Vec<usize> = String::from_utf8_lossy(&panes_output.stdout)
        .lines()
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    
    for idx in pane_indices.iter().rev() {
        if *idx > 0 {
            let _ = Command::new("tmux")
                .args(&["kill-pane", "-t", &format!("{}:0.{}", session_name, idx)])
                .status();
        }
    }
    
    // Recreate panes as in create() (split logic)
    // ... (same as create, but skipping the initial session creation)
    
    // Recreate tabs (windows)
    // ... (same as create)
    
    Ok(())
}
```

---

## 6. send_keys(session_name, keys) → Result\<(), MuxError\>

**Trait contract:**  
Send a key sequence to the focused pane in a session.

**tmux mapping:**

```sh
tmux send-keys -t <session> <keys> [Enter]
```

The `keys` string is passed as-is. If the caller wants Enter pressed, they must include
`Enter` (or `C-m`) in the string explicitly.

**Edge cases & awkward fits:**

- **No pane targeting**: send-keys sends to the focused pane only. If you need to send
  to a specific pane, you must include the pane ID: `-t <session>:0.1`. The trait only
  takes a session name, so it always targets the focused pane.
  
  **Future enhancement**: If PaneSpec had an ID field and send_keys took a pane ID,
  we could target specific panes. But then send_keys must not send Enter implicitly
  (else it would execute immediately).

- **Special key names**: tmux interprets `C-c`, `M-x`, `Enter`, etc. But arbitrary
  Unicode is passed literally. The caller must know which keys are special.

- **No echo**: send-keys doesn't echo back to the caller; it just sends to the pane.
  If you want confirmation, you'd need to read pane output separately (via capture-pane).

**Implementation notes:**

```rust
fn send_keys(&self, session_name: &str, keys: &str) -> Result<(), MuxError> {
    let status = Command::new("tmux")
        .args(&["send-keys", "-t", session_name, keys])
        .status()?;
    
    if !status.success() {
        return Err(MuxError::SessionNotFound(session_name.into()));
    }
    
    Ok(())
}
```

---

## 7. subscribe_topology(session_name) → Result\<Receiver\<TopologyEvent\>, MuxError\>

**Trait contract:**  
Return a channel receiver that emits `TopologyEvent`s (pane open/close, focus change, layout change)
as the session layout changes.

**tmux mapping:**

Tmux does not have a native event stream API. Implementation options:

### Option A: Polling (simplest)

Spawn a background task that:
1. Captures current layout periodically (every 100ms).
2. Diffs against the previous capture.
3. Emits TopologyEvent for changes.

```rust
fn subscribe_topology(&self, session_name: &str) 
    -> Result<Receiver<TopologyEvent>, MuxError> 
{
    let (tx, rx) = mpsc::channel();
    let session = session_name.to_string();
    let mux = Arc::clone(&self.0); // Assuming self wraps Arc<TmuxImpl>
    
    std::thread::spawn(move || {
        let mut last_panes = Vec::new();
        loop {
            match mux.capture(&session) {
                Ok(capture) => {
                    // Diff capture.panes against last_panes
                    for pane in &capture.panes {
                        if !last_panes.iter().any(|p: &PaneSpec| p.id == pane.id) {
                            let _ = tx.send(TopologyEvent::PaneOpened {
                                pane_id: pane.id.clone(),
                            });
                        }
                    }
                    for pane in &last_panes {
                        if !capture.panes.iter().any(|p| p.id == pane.id) {
                            let _ = tx.send(TopologyEvent::PaneClosed {
                                pane_id: pane.id.clone(),
                            });
                        }
                    }
                    last_panes = capture.panes;
                }
                Err(_) => break, // Session died
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });
    
    Ok(rx)
}
```

**Pros:** Simple, no tmux API enhancements needed.  
**Cons:** High latency (100ms poll interval), CPU usage.

### Option B: inotify / fs watcher (more efficient)

On Linux, tmux stores session state in `/tmp/tmux-<uid>/<session>`. Watch the directory
for changes using `inotify`, then capture on change.

**Pros:** Event-driven, lower latency.  
**Cons:** Linux-only, fragile (implementation detail of tmux).

### Option C: Hook into tmux config (advanced)

Tmux supports hooks in its config, but they're not accessible via CLI. This requires
patching tmux or running a custom build.

**Not viable for a general backend.**

### Option D: Use tmux event hook extension (if available in future tmux)

Tmux ≥ 3.4 has experimental event subscriptions. If that stabilizes, we could use it.
For now, option A is the fallback.

**Edge cases & awkward fits:**

- **CRITICAL: No native event API.**  
  The trait assumes topology events are observable, but tmux has no built-in way to
  watch a session. Polling is O(n) per sample (must capture entire layout each time).
  For large sessions, this is expensive.
  
  Zellij likely has a native event API, so this asymmetry is notable. A production tmux
  backend must either accept polling overhead or document it as a capability limitation.

- **Focus change detection**: Detecting which pane has focus requires comparing captures.
  The focused pane is implicit in `capture()` output (order or some flag), not explicit.
  **Current assumption**: We track which pane was the last to receive send_keys() as a
  heuristic for focus.

- **LayoutChanged granularity**: Should we emit LayoutChanged when pane count changes?
  Pane size changes? Pane order changes? The trait doesn't define it. **Current assumption**:
  emit if the pane count or tab count changes.

- **Channel close semantics**: If the session dies, the background task exits and the
  channel closes. The caller detects this as a clean close. Good.

**Implementation notes:**

See Option A above.

---

## 8. capabilities() → Capability

**Trait contract:**  
Return a bitflags struct indicating which features this backend supports.

**tmux flags:**

```rust
fn capabilities(&self) -> Capability {
    let mut cap = Capability::empty();
    
    // PLUGIN_EMBEDDING: tmux does NOT support embedding external UIs
    // cap |= Capability::PLUGIN_EMBEDDING; // ← false
    
    // COPY_MODE: tmux has copy mode (prefix + [)
    cap |= Capability::COPY_MODE;
    
    // SCROLLBACK_TEXT: tmux scrollback is accessible via capture-pane
    cap |= Capability::SCROLLBACK_TEXT;
    
    cap
}
```

**Edge cases & awkward fits:**

- **PLUGIN_EMBEDDING mismatch**: Zellij supports embedding plugin UIs inside panes.
  Tmux does not. If Resurreccion relies on plugin embedding for its UI, a tmux backend
  cannot deliver that feature. This is a hard architectural gap.
  
  **Mitigation**: Ensure Resurreccion is usable with PLUGIN_EMBEDDING=false (e.g., use
  stdio/terminal UI instead).

- **SCROLLBACK_TEXT portability**: Scrollback text via `capture-pane -p -S -<num>` works,
  but the scrollback buffer size is configurable and limited. Large scrollbacks may be
  truncated. Document the limit clearly.

---

## Summary: Leverage Gate Analysis

The Mux trait has good coverage for basic session/pane management. Mapping to tmux CLI reveals:

### Clean Fits
- **discover()**: tmux list-sessions works directly.
- **attach()**: tmux attach-session works directly.
- **capture()**: tmux list-panes/list-windows works.
- **send_keys()**: tmux send-keys works.
- **capabilities()**: Easy to flag what tmux supports.

### Awkward Fits

1. **LayoutSpec underspecification** (affects `create()` and `apply_layout()`)
   - No geometry (pane positions, sizes, split directions).
   - No pane-to-tab mapping.
   - No command to execute (only cwd).
   
   **Resolution**: Zellij LayoutSpec likely mirrors Zellij's internal format. For leverage,
   accept this constraint in Sprint 0.4. A second backend must encode geometry somehow
   (e.g., nested tree of panes), or Resurreccion must not preserve exact geometry.

2. **Destructive layout application** (`apply_layout()`)
   - Tmux cannot morph a layout in-place; it must kill and recreate.
   - Running commands in old panes are lost.
   
   **Resolution**: Document `apply_layout()` as destructive. If Resurreccion needs non-destructive
   updates, it must buffer commands and restart them. Or: design an incremental layout diff API.

3. **No native event API** (`subscribe_topology()`)
   - Tmux has no pub/sub event stream. Polling is the fallback.
   - High latency, CPU overhead for large sessions.
   
   **Resolution**: Accept polling in a tmux backend. Optimize interval based on workload.
   Or: document this as a performance limitation in the trait design.

4. **Plugin embedding unavailable** (`capabilities()`)
   - Zellij supports embedding plugin UIs; tmux does not.
   - Hard architectural gap.
   
   **Resolution**: Ensure Resurreccion is usable without PLUGIN_EMBEDDING. Use stdio
   or terminal UI for the second backend.

5. **Pane focus control** (affects `create()` and `send_keys()`)
   - Trait has no way to set initial focus or target a specific pane.
   - Tmux focuses by pane ID, not by index.
   
   **Resolution**: Add an optional `focused_pane_id` to LayoutSpec, or accept that
   focus is unpredictable at creation time.

### Verdict

The Mux trait **successfully gates on key differences** between Zellij and tmux:
- LayoutSpec geometry (Zellij → tmux: awkward but workable).
- Layout destructiveness (tmux limitation, not a trait problem).
- Event API (tmux limitation, polling fallback acceptable).
- Plugin embedding (tmux missing, can work around if Resurreccion doesn't require it).

A tmux backend is **feasible** with the current trait, provided:
1. Resurreccion accepts LayoutSpec as-is (no geometry preservation).
2. Resurreccion documents layout changes as destructive.
3. Resurreccion's UI does not depend on plugin embedding.
4. Resurreccion tolerates ~100ms topology event latency.

**Recommendation for Sprint 0.5+:** Expand the trait (or create a `LayoutSpec` v2) to
support geometry and non-destructive updates. This will make a third backend (e.g., kitty)
easier to implement.
