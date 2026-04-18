-- Muxon initial schema
-- Version: 1
-- This file is the single source of truth for the database schema.
-- Lane A (Store implementation) executes this migration on first open.

CREATE TABLE IF NOT EXISTS schema_migrations (
    version     INTEGER PRIMARY KEY,
    applied_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS workspaces (
    id              TEXT PRIMARY KEY,       -- ULID
    binding_key     TEXT NOT NULL UNIQUE,   -- hex BLAKE3 of path + git metadata
    display_name    TEXT NOT NULL,
    root_path       TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    last_opened_at  TEXT
);

CREATE TABLE IF NOT EXISTS runtimes (
    id              TEXT PRIMARY KEY,       -- ULID
    workspace_id    TEXT NOT NULL REFERENCES workspaces(id),
    session_name    TEXT NOT NULL,
    backend         TEXT NOT NULL,          -- e.g. "zellij"
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    detached_at     TEXT
);

CREATE TABLE IF NOT EXISTS snapshots (
    id              TEXT PRIMARY KEY,       -- ULID
    workspace_id    TEXT NOT NULL REFERENCES workspaces(id),
    runtime_id      TEXT REFERENCES runtimes(id),
    fidelity        TEXT NOT NULL,          -- "exact"|"stateful"|"structural"|"historical"
    manifest_json   TEXT NOT NULL,          -- JSON blob describing what was captured
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS blobs (
    id              TEXT PRIMARY KEY,       -- BLAKE3 hex (content-addressed)
    size_bytes      INTEGER NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS events (
    id              TEXT PRIMARY KEY,       -- ULID (time-sortable)
    workspace_id    TEXT REFERENCES workspaces(id),
    kind            TEXT NOT NULL,
    payload_json    TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO schema_migrations (version) VALUES (1);
