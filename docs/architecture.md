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
