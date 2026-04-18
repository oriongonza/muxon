//! Session replay data model for Resurreccion.
//!
//! Events are read from the store, optionally filtered by workspace, and
//! returned in chronological order for playback.

use resurreccion_store::{EventRow, Store};

/// A single frame in a replay session, derived from an [`EventRow`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReplayFrame {
    /// The event's ULID identifier.
    pub event_id: String,
    /// The associated workspace ULID, if any.
    pub workspace_id: Option<String>,
    /// Event kind discriminator string.
    pub kind: String,
    /// Decoded event payload.
    pub payload: serde_json::Value,
    /// ISO-8601 creation timestamp.
    pub timestamp: String,
}

/// A replay session over a sequence of [`ReplayFrame`]s.
pub struct ReplaySession<'a> {
    /// Reference to the backing store (retained for potential future live
    /// tail use).
    store: &'a Store,
    /// Workspace filter applied when the session was created.
    workspace_id: Option<String>,
    /// All frames available for playback.
    frames: Vec<ReplayFrame>,
    /// Current playback cursor (points at the *next* frame to be returned
    /// by [`Self::next`], or the current frame for [`Self::current_frame`]).
    cursor: usize,
}

impl<'a> ReplaySession<'a> {
    /// Create a replay session covering all events in the store.
    pub fn new(store: &'a Store) -> anyhow::Result<Self> {
        let events = store.event_tail_from(None)?;
        let frames = events_to_frames(events);
        Ok(Self {
            store,
            workspace_id: None,
            frames,
            cursor: 0,
        })
    }

    /// Create a replay session filtered to a specific workspace.
    pub fn for_workspace(
        store: &'a Store,
        workspace_id: impl Into<String>,
    ) -> anyhow::Result<Self> {
        let workspace_id = workspace_id.into();
        let all_events = store.event_tail_from(None)?;
        let filtered: Vec<_> = all_events
            .into_iter()
            .filter(|e| e.workspace_id.as_deref() == Some(&workspace_id))
            .collect();
        let frames = events_to_frames(filtered);
        Ok(Self {
            store,
            workspace_id: Some(workspace_id),
            frames,
            cursor: 0,
        })
    }

    /// Returns the total number of frames available for playback.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Returns the frame at the current cursor position, if any.
    pub fn current_frame(&self) -> Option<&ReplayFrame> {
        self.frames.get(self.cursor)
    }

    /// Advances the cursor by one and returns the new current frame, if any.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<&ReplayFrame> {
        if self.cursor < self.frames.len() {
            self.cursor += 1;
        }
        self.frames.get(self.cursor)
    }

    /// Seeks the cursor to `pos`, clamped to `frame_count`.
    pub fn seek(&mut self, pos: usize) {
        self.cursor = pos.min(self.frames.len());
    }

    /// Returns a slice of all frames in chronological order.
    pub fn all_frames(&self) -> &[ReplayFrame] {
        &self.frames
    }

    /// Returns the workspace filter active for this session, if any.
    pub fn workspace_id(&self) -> Option<&str> {
        self.workspace_id.as_deref()
    }

    /// Returns a reference to the backing store.
    pub fn store(&self) -> &Store {
        self.store
    }
}

/// Convert a slice of [`EventRow`]s into [`ReplayFrame`]s.
fn events_to_frames(events: Vec<EventRow>) -> Vec<ReplayFrame> {
    events
        .into_iter()
        .map(|e| ReplayFrame {
            event_id: e.id,
            workspace_id: e.workspace_id,
            kind: e.kind,
            payload: serde_json::from_str(&e.payload_json).unwrap_or(serde_json::Value::Null),
            timestamp: e.created_at,
        })
        .collect()
}
