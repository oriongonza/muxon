# Event DAG — Design Insights

## 1. An edge IS an event type

Before: `DagEdge("protocol", "WorkspaceListCmd", "store")` and `struct WorkspaceListCmd`
were two separate things connected only by a string.

After: declaring the edge *is* declaring the type. Remove the struct → compile error at
the edge site. The topology becomes a fact the compiler enforces, not a comment that rots.

```rust
dag_event!(WorkspaceListCmd { reply: SyncSender<Vec<Workspace>> });

const EDGES: &[DagEdge] = edges![
    WorkspaceListCmd: protocol -> store,
];
```

---

## 2. walk(t, v, a) — every guarantee is a visitor

Termination (cycle detection), completeness (reachability), and the runtime event loop
are all the same abstraction:

```
t = topology   (the wiring, computed once)
v = visitor    (the programmer's only concern)
a = the answer
```

New structural guarantees cost nothing to add: implement `Visitor`, call `walk`.
The topology never changes.

This is the halting problem bypass — constrain the program to a DAG and termination
is structural, not reasoned per-path.

---

## 3. `Box<dyn Fn(&mut EventBus) + Send>` vs enum dispatch

The old pattern: one enum variant per event type, one match arm per variant, forever.

The insight: closures *are* the dispatch table.

```rust
emitter.emit(WorkspaceOpened { workspace_id });
// boxes: |bus| bus.emit(event)
// bus thread calls: f(&mut bus)
// TypeId dispatch happens inside rt-events — you never see it
```

Adding a new event type requires zero changes to the channel infrastructure.

---

## 4. `EventBus` is not `Send` — and that's fine

rt-events is deliberately single-threaded. The answer isn't `unsafe impl Send` or a
lock — it's a thread that *owns* the bus and accepts work over a channel. The Tokio
async world and the sync bus world are separated by one `SyncSender`.

```
handler ──SyncSender<Box<dyn Fn>>──> BUS THREAD ──> bus.emit() ──> subscribers
```

---

## 5. The wiring module as executable architecture diagram

`wiring.rs` is the only file where you can see the full system topology at once.
It's not documentation — it *runs* at startup.

- Add a node without wiring it → `check_completeness` fails at boot.
- Create a cycle → `check_termination` fails at boot.

The architecture diagram cannot drift from the implementation because it *is* the
implementation.

---

## 6. Newtypes over strings at the boundary

The protocol boundary receives strings. Past that boundary, everything uses
`WorkspaceId`, `SnapshotId` etc. `FromStr` on each newtype means the parse happens
once, at handler entry. The rest of the handler, the emitter, the bus, and
rt-events subscribers all see typed values.

The compiler enforces correctness everywhere except at the JSON envelope edge —
which is where it should be.
