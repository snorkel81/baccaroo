use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::core::events::{EventStore, GameEvent, InMemoryEventStore};
use crate::core::idempotency::IdempotencyTracker;
use crate::core::rng::CommitRevealRng;
use crate::core::wallet::Wallet;
use crate::game::blackjack::engine::BlackjackEngine;
use crate::game::blackjack::types::{BlackjackAction, BlackjackConfig, BlackjackView};
use crate::game::traits::{GameEngine, GameError, GamePhase};

/// Holds the state of a single game session, protected by a per-session mutex
/// so only one action can be processed at a time (Design Rule 6).
struct SessionState {
    engine: BlackjackEngine,
    rng: CommitRevealRng,
    last_activity: Instant,
    player_id: Uuid,
}

/// Manages all active sessions and enforces concurrency / idempotency rules.
pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<Uuid, Arc<Mutex<SessionState>>>>>,
    player_sessions: Arc<Mutex<HashMap<Uuid, Uuid>>>,
    idempotency: Arc<Mutex<IdempotencyTracker>>,
    wallet: Arc<Wallet<InMemoryEventStore>>,
    config: AppConfig,
    blackjack_config: BlackjackConfig,
}

impl SessionManager {
    pub fn new(
        wallet: Arc<Wallet<InMemoryEventStore>>,
        config: AppConfig,
        blackjack_config: BlackjackConfig,
    ) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            player_sessions: Arc::new(Mutex::new(HashMap::new())),
            idempotency: Arc::new(Mutex::new(IdempotencyTracker::new())),
            wallet,
            config,
            blackjack_config,
        }
    }

    /// Register a new player with the default starting balance.
    pub async fn register_player(&self, player_id: Uuid) {
        self.wallet
            .register_player(player_id, self.config.initial_balance);
    }

    /// Start a new game session for the given player.
    pub async fn new_session(&self, player_id: Uuid) -> Result<(Uuid, String), GameError> {
        // Enforce one session per player (Design Rule 9: prevent multi-tab abuse).
        let mut player_sessions = self.player_sessions.lock().await;
        if let Some(existing_sid) = player_sessions.get(&player_id) {
            // Check if the existing session is still active.
            let sessions = self.sessions.lock().await;
            if let Some(sess_lock) = sessions.get(existing_sid) {
                let sess = sess_lock.lock().await;
                if sess.engine.phase() != GamePhase::Closed {
                    return Err(GameError::SessionAlreadyExists);
                }
            }
        }

        let rng = CommitRevealRng::new();
        let commitment = rng.commitment().to_string();
        let engine = BlackjackEngine::new_round(&self.blackjack_config, {
            // Use a separate seed for the engine (derived from the same entropy source).
            let mut seed = [0u8; 32];
            rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut seed);
            seed
        });
        let session_id = engine.session_id();

        // Log session start.
        self.wallet.event_store().append(GameEvent::SessionStarted {
            session_id,
            player_id,
            timestamp: chrono::Utc::now(),
        });

        let state = SessionState {
            engine,
            rng,
            last_activity: Instant::now(),
            player_id,
        };

        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id, Arc::new(Mutex::new(state)));
        player_sessions.insert(player_id, session_id);

        Ok((session_id, commitment))
    }

    /// Place a bet on an existing session.
    pub async fn place_bet(
        &self,
        session_id: Uuid,
        amount: u64,
        action_id: Uuid,
    ) -> Result<BlackjackView, GameError> {
        // Idempotency check.
        {
            let tracker = self.idempotency.lock().await;
            if let Some(cached) = tracker.is_duplicate(session_id, action_id) {
                return serde_json::from_str(cached)
                    .map_err(|e| GameError::Internal(e.to_string()));
            }
        }

        // Validate bet limits.
        if amount < self.config.min_bet || amount > self.config.max_bet {
            return Err(GameError::InvalidBet(format!(
                "bet must be between {} and {}",
                self.config.min_bet, self.config.max_bet
            )));
        }

        let sess_lock = self.get_session(session_id).await?;
        let mut sess = sess_lock.lock().await;
        self.check_timeout(&sess)?;

        // Check balance.
        if !self.wallet.can_afford(sess.player_id, amount) {
            return Err(GameError::InsufficientBalance);
        }

        let events = sess.engine.place_bet(amount, action_id)?;
        sess.last_activity = Instant::now();

        // Persist events.
        for event in &events {
            self.wallet.event_store().append(event.clone());
        }

        let view = sess.engine.player_view();

        // Cache for idempotency.
        let view_json =
            serde_json::to_string(&view).map_err(|e| GameError::Internal(e.to_string()))?;
        {
            let mut tracker = self.idempotency.lock().await;
            tracker.record(session_id, action_id, view_json);
        }

        Ok(view)
    }

    /// Submit a game action (hit, stand, double).
    pub async fn submit_action(
        &self,
        session_id: Uuid,
        action: BlackjackAction,
        action_id: Uuid,
    ) -> Result<BlackjackView, GameError> {
        // Idempotency check.
        {
            let tracker = self.idempotency.lock().await;
            if let Some(cached) = tracker.is_duplicate(session_id, action_id) {
                return serde_json::from_str(cached)
                    .map_err(|e| GameError::Internal(e.to_string()));
            }
        }

        let sess_lock = self.get_session(session_id).await?;
        let mut sess = sess_lock.lock().await;
        self.check_timeout(&sess)?;

        // Check if player can afford double (additional bet).
        if action == BlackjackAction::Double {
            let current_bet = sess.engine.bet_amount();
            if !self.wallet.can_afford(sess.player_id, current_bet) {
                return Err(GameError::InsufficientBalance);
            }
        }

        let events = sess.engine.apply_action(action)?;
        sess.last_activity = Instant::now();

        for event in &events {
            self.wallet.event_store().append(event.clone());
        }

        let view = sess.engine.player_view();

        // If the game ended, clean up idempotency tracker for this session.
        if sess.engine.phase() == GamePhase::Closed {
            let mut tracker = self.idempotency.lock().await;
            tracker.clear_session(session_id);
        }

        // Cache for idempotency.
        let view_json =
            serde_json::to_string(&view).map_err(|e| GameError::Internal(e.to_string()))?;
        {
            let mut tracker = self.idempotency.lock().await;
            tracker.record(session_id, action_id, view_json);
        }

        Ok(view)
    }

    /// Get the current visible game state.
    pub async fn get_state(&self, session_id: Uuid) -> Result<BlackjackView, GameError> {
        let sess_lock = self.get_session(session_id).await?;
        let sess = sess_lock.lock().await;
        Ok(sess.engine.player_view())
    }

    /// Reveal the RNG seed after the game ends.
    pub async fn reveal_seed(&self, session_id: Uuid) -> Result<(String, String), GameError> {
        let sess_lock = self.get_session(session_id).await?;
        let sess = sess_lock.lock().await;

        if sess.engine.phase() != GamePhase::Closed {
            return Err(GameError::InvalidAction {
                action: "reveal".into(),
                phase: sess.engine.phase(),
            });
        }

        Ok((sess.rng.reveal_seed(), sess.rng.commitment().to_string()))
    }

    /// Get a player's current balance.
    pub async fn get_balance(&self, player_id: Uuid) -> u64 {
        self.wallet.balance(player_id)
    }

    async fn get_session(
        &self,
        session_id: Uuid,
    ) -> Result<Arc<Mutex<SessionState>>, GameError> {
        let sessions = self.sessions.lock().await;
        sessions
            .get(&session_id)
            .cloned()
            .ok_or_else(|| GameError::SessionNotFound(session_id.to_string()))
    }

    fn check_timeout(&self, sess: &SessionState) -> Result<(), GameError> {
        if sess.last_activity.elapsed() > Duration::from_secs(self.config.session_timeout_secs) {
            return Err(GameError::SessionTimeout);
        }
        Ok(())
    }
}
