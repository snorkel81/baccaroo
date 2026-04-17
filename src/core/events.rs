use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Every meaningful thing that happens in a game session is captured as an event.
/// Events are append-only and form the authoritative record of what happened.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    SessionStarted {
        session_id: Uuid,
        player_id: Uuid,
        timestamp: DateTime<Utc>,
    },
    BetPlaced {
        session_id: Uuid,
        amount: u64,
        action_id: Uuid,
        timestamp: DateTime<Utc>,
    },
    CardDealt {
        session_id: Uuid,
        recipient: CardRecipient,
        card: String,
        face_up: bool,
        sequence: u32,
        timestamp: DateTime<Utc>,
    },
    PlayerAction {
        session_id: Uuid,
        action: String,
        action_id: Uuid,
        timestamp: DateTime<Utc>,
    },
    DealerAction {
        session_id: Uuid,
        action: String,
        timestamp: DateTime<Utc>,
    },
    Payout {
        session_id: Uuid,
        amount: i64,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    SessionEnded {
        session_id: Uuid,
        timestamp: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CardRecipient {
    Player,
    Dealer,
}

impl GameEvent {
    pub fn session_id(&self) -> Uuid {
        match self {
            GameEvent::SessionStarted { session_id, .. }
            | GameEvent::BetPlaced { session_id, .. }
            | GameEvent::CardDealt { session_id, .. }
            | GameEvent::PlayerAction { session_id, .. }
            | GameEvent::DealerAction { session_id, .. }
            | GameEvent::Payout { session_id, .. }
            | GameEvent::SessionEnded { session_id, .. } => *session_id,
        }
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            GameEvent::SessionStarted { timestamp, .. }
            | GameEvent::BetPlaced { timestamp, .. }
            | GameEvent::CardDealt { timestamp, .. }
            | GameEvent::PlayerAction { timestamp, .. }
            | GameEvent::DealerAction { timestamp, .. }
            | GameEvent::Payout { timestamp, .. }
            | GameEvent::SessionEnded { timestamp, .. } => *timestamp,
        }
    }
}

/// Trait for event storage backends. The in-memory implementation is used for
/// the initial commit; a persistent backend can be swapped in later.
pub trait EventStore: Send + Sync {
    fn append(&self, event: GameEvent);
    fn events_for_session(&self, session_id: Uuid) -> Vec<GameEvent>;
    fn all_events_for_player(&self, player_id: Uuid) -> Vec<GameEvent>;
}

/// In-memory event store backed by a `Mutex<HashMap>`.
#[derive(Debug, Clone)]
pub struct InMemoryEventStore {
    events: Arc<Mutex<Vec<GameEvent>>>,
    session_player_map: Arc<Mutex<HashMap<Uuid, Uuid>>>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            session_player_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

impl EventStore for InMemoryEventStore {
    fn append(&self, event: GameEvent) {
        if let GameEvent::SessionStarted {
            session_id,
            player_id,
            ..
        } = &event
        {
            let mut map = self.session_player_map.lock().expect("lock poisoned");
            map.insert(*session_id, *player_id);
        }
        let mut events = self.events.lock().expect("lock poisoned");
        events.push(event);
    }

    fn events_for_session(&self, session_id: Uuid) -> Vec<GameEvent> {
        let events = self.events.lock().expect("lock poisoned");
        events
            .iter()
            .filter(|e| e.session_id() == session_id)
            .cloned()
            .collect()
    }

    fn all_events_for_player(&self, player_id: Uuid) -> Vec<GameEvent> {
        let map = self.session_player_map.lock().expect("lock poisoned");
        let session_ids: Vec<Uuid> = map
            .iter()
            .filter(|(_, pid)| **pid == player_id)
            .map(|(sid, _)| *sid)
            .collect();
        drop(map);

        let events = self.events.lock().expect("lock poisoned");
        events
            .iter()
            .filter(|e| session_ids.contains(&e.session_id()))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_and_retrieve_events() {
        let store = InMemoryEventStore::new();
        let session_id = Uuid::new_v4();
        let player_id = Uuid::new_v4();

        store.append(GameEvent::SessionStarted {
            session_id,
            player_id,
            timestamp: Utc::now(),
        });

        store.append(GameEvent::BetPlaced {
            session_id,
            amount: 100,
            action_id: Uuid::new_v4(),
            timestamp: Utc::now(),
        });

        let events = store.events_for_session(session_id);
        assert_eq!(events.len(), 2);

        let player_events = store.all_events_for_player(player_id);
        assert_eq!(player_events.len(), 2);
    }
}
