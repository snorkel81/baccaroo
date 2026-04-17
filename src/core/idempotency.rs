use std::collections::HashMap;
use uuid::Uuid;

/// Tracks processed action IDs per session to enforce idempotency.
///
/// Every client action must include a unique `action_id`. If the same ID is
/// submitted twice, the server returns the cached result instead of
/// re-processing. Action IDs are scoped to a session and discarded when the
/// session closes.
#[derive(Debug, Clone)]
pub struct IdempotencyTracker {
    /// Maps session_id -> (action_id -> cached JSON response).
    processed: HashMap<Uuid, HashMap<Uuid, String>>,
}

impl IdempotencyTracker {
    pub fn new() -> Self {
        Self {
            processed: HashMap::new(),
        }
    }

    /// Check whether an action has already been processed for this session.
    pub fn is_duplicate(&self, session_id: Uuid, action_id: Uuid) -> Option<&str> {
        self.processed
            .get(&session_id)
            .and_then(|m| m.get(&action_id))
            .map(|s| s.as_str())
    }

    /// Record a processed action and its cached response.
    pub fn record(&mut self, session_id: Uuid, action_id: Uuid, response: String) {
        self.processed
            .entry(session_id)
            .or_default()
            .insert(action_id, response);
    }

    /// Remove all tracked action IDs for a closed session.
    pub fn clear_session(&mut self, session_id: Uuid) {
        self.processed.remove(&session_id);
    }
}

impl Default for IdempotencyTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idempotency_dedup() {
        let mut tracker = IdempotencyTracker::new();
        let session = Uuid::new_v4();
        let action = Uuid::new_v4();

        assert!(tracker.is_duplicate(session, action).is_none());

        tracker.record(session, action, r#"{"ok":true}"#.into());

        assert_eq!(
            tracker.is_duplicate(session, action),
            Some(r#"{"ok":true}"#)
        );
    }

    #[test]
    fn test_clear_session() {
        let mut tracker = IdempotencyTracker::new();
        let session = Uuid::new_v4();
        let action = Uuid::new_v4();

        tracker.record(session, action, "cached".into());
        tracker.clear_session(session);

        assert!(tracker.is_duplicate(session, action).is_none());
    }
}
