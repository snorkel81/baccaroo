use crate::core::events::GameEvent;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Phases that every game session passes through. This FSM is the backbone
/// of the anti-cheat system (Design Rule 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GamePhase {
    Init,
    BetPlaced,
    PlayerTurn,
    DealerTurn,
    Settlement,
    Closed,
}

impl std::fmt::Display for GamePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GamePhase::Init => write!(f, "Init"),
            GamePhase::BetPlaced => write!(f, "BetPlaced"),
            GamePhase::PlayerTurn => write!(f, "PlayerTurn"),
            GamePhase::DealerTurn => write!(f, "DealerTurn"),
            GamePhase::Settlement => write!(f, "Settlement"),
            GamePhase::Closed => write!(f, "Closed"),
        }
    }
}

/// Settlement outcome for a completed round.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementResult {
    pub payout: i64,
    pub reason: String,
}

/// Errors that the game engine can produce.
#[derive(Debug, Error)]
pub enum GameError {
    #[error("invalid action '{action}' in phase {phase}")]
    InvalidAction { action: String, phase: GamePhase },

    #[error("invalid bet amount: {0}")]
    InvalidBet(String),

    #[error("insufficient balance")]
    InsufficientBalance,

    #[error("session not found: {0}")]
    SessionNotFound(String),

    #[error("session already exists for this player")]
    SessionAlreadyExists,

    #[error("duplicate action ID: {0}")]
    DuplicateAction(String),

    #[error("session is locked, another action is being processed")]
    SessionLocked,

    #[error("session has timed out")]
    SessionTimeout,

    #[error("internal error: {0}")]
    Internal(String),
}

/// The core trait that every game implementation must satisfy.
///
/// The trait is generic over action and state types so that different games
/// (blackjack, baccarat, roulette) can define their own domain types while
/// sharing the same session management and API infrastructure.
pub trait GameEngine: Send + Sync {
    /// Game-specific actions (e.g. Hit, Stand, Double).
    type Action: Send + Sync + Clone + std::fmt::Debug;
    /// Game-specific visible state returned to the player.
    type State: Send + Sync + Clone + Serialize;
    /// Game-specific configuration (e.g. deck count).
    type Config: Send + Sync + Clone;

    /// Start a new round with the given config and RNG seed.
    fn new_round(config: &Self::Config, rng_seed: [u8; 32]) -> Self;

    /// Current phase of the game.
    fn phase(&self) -> GamePhase;

    /// Actions that are valid right now.
    fn valid_actions(&self) -> Vec<Self::Action>;

    /// Apply an action, returning the resulting events or an error.
    fn apply_action(&mut self, action: Self::Action) -> Result<Vec<GameEvent>, GameError>;

    /// The view of the game state that the client is allowed to see.
    /// Must hide information the player shouldn't know (e.g. dealer hole card).
    fn player_view(&self) -> Self::State;

    /// If the game has reached settlement, return the result.
    fn settlement(&self) -> Option<SettlementResult>;
}
