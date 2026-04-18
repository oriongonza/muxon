# Snapshot Format

Snapshots are the durable artifacts created by Resurreccion.

The first pass should separate:

- manifest metadata in SQLite
- filesystem artifacts on disk
- backend-specific extras stored as opaque payloads

The exact wire and storage format is intentionally deferred until the control
plane and store schema are in place.
