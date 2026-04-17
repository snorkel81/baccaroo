use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::api::dto::*;
use crate::game::blackjack::types::BlackjackAction;
use crate::game::session::SessionManager;

/// Application state shared across all request handlers.
pub type AppState = Arc<SessionManager>;

/// POST /api/session/new -- start a new game session.
pub async fn new_session(
    State(state): State<AppState>,
    Json(req): Json<NewSessionRequest>,
) -> impl IntoResponse {
    // Ensure player is registered.
    state.register_player(req.player_id).await;

    match state.new_session(req.player_id).await {
        Ok((session_id, commitment)) => (
            StatusCode::OK,
            Json(serde_json::to_value(NewSessionResponse {
                session_id,
                commitment,
                message: "Session created. Place your bet.".into(),
            })
            .unwrap()),
        ),
        Err(e) => {
            let err = ErrorResponse::from_game_error(&e);
            (
                error_status(&e),
                Json(serde_json::to_value(err).unwrap()),
            )
        }
    }
}

/// POST /api/session/:id/bet -- place a bet.
pub async fn place_bet(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(req): Json<BetRequest>,
) -> impl IntoResponse {
    match state.place_bet(session_id, req.amount, req.action_id).await {
        Ok(view) => (StatusCode::OK, Json(serde_json::to_value(view).unwrap())),
        Err(e) => {
            let err = ErrorResponse::from_game_error(&e);
            (
                error_status(&e),
                Json(serde_json::to_value(err).unwrap()),
            )
        }
    }
}

/// POST /api/session/:id/action -- submit a game action.
pub async fn submit_action(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(req): Json<ActionRequest>,
) -> impl IntoResponse {
    let action = match req.action.to_lowercase().as_str() {
        "hit" => BlackjackAction::Hit,
        "stand" => BlackjackAction::Stand,
        "double" => BlackjackAction::Double,
        other => {
            let err = ErrorResponse {
                error: format!("Unknown action: {other}"),
                code: "INVALID_ACTION".into(),
            };
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::to_value(err).unwrap()),
            );
        }
    };

    match state.submit_action(session_id, action, req.action_id).await {
        Ok(view) => (StatusCode::OK, Json(serde_json::to_value(view).unwrap())),
        Err(e) => {
            let err = ErrorResponse::from_game_error(&e);
            (
                error_status(&e),
                Json(serde_json::to_value(err).unwrap()),
            )
        }
    }
}

/// GET /api/session/:id/state -- get current visible game state.
pub async fn get_state(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.get_state(session_id).await {
        Ok(view) => (StatusCode::OK, Json(serde_json::to_value(view).unwrap())),
        Err(e) => {
            let err = ErrorResponse::from_game_error(&e);
            (
                error_status(&e),
                Json(serde_json::to_value(err).unwrap()),
            )
        }
    }
}

/// GET /api/session/:id/reveal -- get seed reveal after game ends.
pub async fn reveal_seed(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.reveal_seed(session_id).await {
        Ok((seed, commitment)) => (
            StatusCode::OK,
            Json(serde_json::to_value(RevealResponse { seed, commitment }).unwrap()),
        ),
        Err(e) => {
            let err = ErrorResponse::from_game_error(&e);
            (
                error_status(&e),
                Json(serde_json::to_value(err).unwrap()),
            )
        }
    }
}

/// GET /api/balance/:player_id -- check player balance.
pub async fn get_balance(
    State(state): State<AppState>,
    Path(player_id): Path<Uuid>,
) -> impl IntoResponse {
    let balance = state.get_balance(player_id).await;
    (
        StatusCode::OK,
        Json(
            serde_json::to_value(BalanceResponse {
                player_id,
                balance,
            })
            .unwrap(),
        ),
    )
}

/// Map game errors to HTTP status codes.
fn error_status(err: &crate::game::traits::GameError) -> StatusCode {
    use crate::game::traits::GameError;
    match err {
        GameError::InvalidAction { .. } | GameError::InvalidBet(_) | GameError::DuplicateAction(_) => {
            StatusCode::BAD_REQUEST
        }
        GameError::InsufficientBalance => StatusCode::PAYMENT_REQUIRED,
        GameError::SessionNotFound(_) => StatusCode::NOT_FOUND,
        GameError::SessionAlreadyExists => StatusCode::CONFLICT,
        GameError::SessionLocked => StatusCode::TOO_MANY_REQUESTS,
        GameError::SessionTimeout => StatusCode::GONE,
        GameError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
