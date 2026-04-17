use std::collections::HashMap;
use uuid::Uuid;

use crate::core::events::{EventStore, GameEvent};

/// In-memory wallet that derives balances from the event log.
///
/// Balance is never stored as a mutable field -- it is always computed by
/// summing relevant events. This makes the system auditable and prevents
/// "balance in limbo" bugs. The `initial_balances` map is used to set
/// starting balances for new players.
#[derive(Debug)]
pub struct Wallet<E: EventStore> {
    event_store: E,
    initial_balances: std::sync::Mutex<HashMap<Uuid, u64>>,
}

impl<E: EventStore> Wallet<E> {
    pub fn new(event_store: E) -> Self {
        Self {
            event_store,
            initial_balances: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Register a player with an initial balance.
    pub fn register_player(&self, player_id: Uuid, initial_balance: u64) {
        let mut map = self.initial_balances.lock().expect("lock poisoned");
        map.entry(player_id).or_insert(initial_balance);
    }

    /// Compute the current balance for a player by replaying their events.
    pub fn balance(&self, player_id: Uuid) -> u64 {
        let initial = {
            let map = self.initial_balances.lock().expect("lock poisoned");
            *map.get(&player_id).unwrap_or(&0)
        };

        let events = self.event_store.all_events_for_player(player_id);
        let mut balance = initial as i64;

        for event in &events {
            match event {
                GameEvent::BetPlaced { amount, .. } => {
                    balance -= *amount as i64;
                }
                GameEvent::Payout { amount, .. } => {
                    balance += *amount;
                }
                _ => {}
            }
        }

        // Balance should never go negative if bets are validated properly,
        // but we clamp to zero defensively (Rule 12: fail safely).
        balance.max(0) as u64
    }

    /// Check whether a player can afford a bet of the given amount.
    pub fn can_afford(&self, player_id: Uuid, amount: u64) -> bool {
        self.balance(player_id) >= amount
    }

    /// Access the underlying event store (for appending events externally).
    pub fn event_store(&self) -> &E {
        &self.event_store
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::InMemoryEventStore;
    use chrono::Utc;

    #[test]
    fn test_balance_from_events() {
        let store = InMemoryEventStore::new();
        let player_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        store.append(GameEvent::SessionStarted {
            session_id,
            player_id,
            timestamp: Utc::now(),
        });

        store.append(GameEvent::BetPlaced {
            session_id,
            amount: 50,
            action_id: Uuid::new_v4(),
            timestamp: Utc::now(),
        });

        store.append(GameEvent::Payout {
            session_id,
            amount: 100,
            reason: "blackjack".into(),
            timestamp: Utc::now(),
        });

        let wallet = Wallet::new(store);
        wallet.register_player(player_id, 1000);

        // 1000 - 50 + 100 = 1050
        assert_eq!(wallet.balance(player_id), 1050);
    }

    #[test]
    fn test_can_afford() {
        let store = InMemoryEventStore::new();
        let wallet = Wallet::new(store);
        let player_id = Uuid::new_v4();
        wallet.register_player(player_id, 100);

        assert!(wallet.can_afford(player_id, 100));
        assert!(!wallet.can_afford(player_id, 101));
    }
}
