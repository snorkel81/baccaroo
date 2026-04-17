use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request to start a new game session. The client sends only a player ID --
/// no state, no preferences that could influence outcomes.
#[derive(Debug, Deserialize)]
pub struct NewSessionRequest {
    pub player_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct NewSessionResponse {
    pub session_id: Uuid,
    pub commitment: String,
    pub message: String,
}

/// Request to place a bet. Contains only intent: the amount and an
/// idempotency key.
#[derive(Debug, Deserialize)]
pub struct BetRequest {
    pub amount: u64,
    pub action_id: Uuid,
}

/// Request to submit a game action. The client names the action; the server
/// decides if it's valid.
#[derive(Debug, Deserialize)]
pub struct ActionRequest {
    pub action: String,
    pub action_id: Uuid,
}

/// Balance inquiry response.
#[derive(Debug, Serialize)]
pub struct BalanceResponse {
    pub player_id: Uuid,
    pub balance: u64,
}

/// Seed reveal response (only available after the game ends).
#[derive(Debug, Serialize)]
pub struct RevealResponse {
    pub seed: String,
    pub commitment: String,
}

/// Generic error response. Internal details are never leaked to the client
/// (Design Rule 12).
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

impl ErrorResponse {
    pub fn from_game_error(err: &crate::game::traits::GameError) -> Self {
        use crate::game::traits::GameError;
        match err {
            GameError::InvalidAction { .. } => Self {
                error: "Invalid action for current game state".into(),
                code: "INVALID_ACTION".into(),
            },
            GameError::InvalidBet(msg) => Self {
                error: msg.clone(),
                code: "INVALID_BET".into(),
            },
            GameError::InsufficientBalance => Self {
                error: "Insufficient balance".into(),
                code: "INSUFFICIENT_BALANCE".into(),
            },
            GameError::SessionNotFound(_) => Self {
                error: "Session not found".into(),
                code: "SESSION_NOT_FOUND".into(),
            },
            GameError::SessionAlreadyExists => Self {
                error: "You already have an active session".into(),
                code: "SESSION_EXISTS".into(),
            },
            GameError::DuplicateAction(_) => Self {
                error: "Duplicate action".into(),
                code: "DUPLICATE_ACTION".into(),
            },
            GameError::SessionLocked => Self {
                error: "Session is busy, try again".into(),
                code: "SESSION_LOCKED".into(),
            },
            GameError::SessionTimeout => Self {
                error: "Session has timed out".into(),
                code: "SESSION_TIMEOUT".into(),
            },
            GameError::Internal(_) => Self {
                // Never expose internal details.
                error: "Internal server error".into(),
                code: "INTERNAL_ERROR".into(),
            },
        }
    }
}
