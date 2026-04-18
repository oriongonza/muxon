# Architecture

Muxon is organized around four high-level responsibilities:

- locating workspaces
- coordinating runtime control
- persisting Resurreccion state
- restoring nested contexts in dependency order

At this stage, the control plane is intentionally centralized:

- the daemon owns orchestration
- the CLI and plugins are clients
- adapters are integration layers, not the source of truth

Eventing is split in two:

- Unix sockets are the inter-process control plane between CLI, plugin, and daemon
- `rt-events` is the in-process observer bus used inside the daemon and future long-lived components

That split matters. `rt-events` decouples subsystems without pretending to be a
network protocol, while the socket protocol remains explicit and inspectable.

## Zellij backend version pin

The `resurreccion-zellij` backend is tested against Zellij 0.40.x CLI output format.
`zellij list-sessions` output and `zellij action dump-layout` format may change in future releases.

## Directory binding and BindingKey composition

`BindingKey` is a BLAKE3-derived `[u8; 32]` that stably identifies a workspace across
machine restarts and renames. The key is computed from a canonical filesystem path plus optional
git metadata, allowing the same workspace to be recognized even after being moved or cloned.

Two scopes control how the key is composed:

- **PathScoped** — keyed on the canonical filesystem path only. Symlink-stable (both the
  symlink and its target canonicalize to the same path) but rename-sensitive: moving a workspace
  breaks the binding.

- **RepoScoped** — keyed on the git remote URL (origin), or the worktree basename if no remote
  is configured. Rename-stable within a checkout; clones on different machines with the same
  origin URL will share the same key, enabling workspace state to follow a repository across
  machines.

**Default:** When a path is inside a git repository, use `RepoScoped` (keyed on remote).
Otherwise, fall back to `PathScoped` (keyed on the canonical path alone).

The scope itself is mixed into the hash input to ensure `PathScoped` and `RepoScoped` keys
for the same path are distinct.
