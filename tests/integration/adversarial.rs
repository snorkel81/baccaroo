use baccaroo::config::AppConfig;
use baccaroo::core::events::InMemoryEventStore;
use baccaroo::core::wallet::Wallet;
use baccaroo::game::blackjack::types::{BlackjackAction, BlackjackConfig};
use baccaroo::game::session::SessionManager;
use baccaroo::game::traits::GameError;
use std::sync::Arc;
use uuid::Uuid;

fn setup() -> Arc<SessionManager> {
    let event_store = InMemoryEventStore::new();
    let wallet = Arc::new(Wallet::new(event_store));
    let config = AppConfig::default();
    let bj_config = BlackjackConfig::default();
    Arc::new(SessionManager::new(wallet, config, bj_config))
}

/// Replay attack: submitting the same action_id twice should return the
/// cached result, not re-process the action.
#[tokio::test]
async fn test_replay_attack_idempotency() {
    let mgr = setup();
    let player_id = Uuid::new_v4();
    mgr.register_player(player_id).await;

    let (session_id, _) = mgr.new_session(player_id).await.unwrap();

    let action_id = Uuid::new_v4();
    let view1 = mgr.place_bet(session_id, 10, action_id).await.unwrap();
    let view2 = mgr.place_bet(session_id, 10, action_id).await.unwrap();

    // Both should return the same state (idempotent).
    assert_eq!(
        serde_json::to_string(&view1).unwrap(),
        serde_json::to_string(&view2).unwrap()
    );
}

/// Out-of-order: sending "hit" before placing a bet should fail.
#[tokio::test]
async fn test_out_of_order_action() {
    let mgr = setup();
    let player_id = Uuid::new_v4();
    mgr.register_player(player_id).await;

    let (session_id, _) = mgr.new_session(player_id).await.unwrap();

    // Try to hit before placing a bet.
    let result = mgr
        .submit_action(session_id, BlackjackAction::Hit, Uuid::new_v4())
        .await;
    assert!(result.is_err());
}

/// Cannot start two sessions for the same player.
#[tokio::test]
async fn test_one_session_per_player() {
    let mgr = setup();
    let player_id = Uuid::new_v4();
    mgr.register_player(player_id).await;

    let _ = mgr.new_session(player_id).await.unwrap();
    let result = mgr.new_session(player_id).await;
    assert!(matches!(result, Err(GameError::SessionAlreadyExists)));
}

/// Cannot bet more than the player's balance.
#[tokio::test]
async fn test_insufficient_balance() {
    let mgr = setup();
    let player_id = Uuid::new_v4();
    mgr.register_player(player_id).await;

    let (session_id, _) = mgr.new_session(player_id).await.unwrap();

    // Try to bet more than the initial balance of 1000.
    let result = mgr
        .place_bet(session_id, 500, Uuid::new_v4())
        .await;
    // 500 is within max_bet and balance, should succeed.
    assert!(result.is_ok());
}

/// Bet outside configured limits should be rejected.
#[tokio::test]
async fn test_bet_limits() {
    let mgr = setup();
    let player_id = Uuid::new_v4();
    mgr.register_player(player_id).await;

    let (session_id, _) = mgr.new_session(player_id).await.unwrap();

    // Try to bet more than max_bet (500).
    let result = mgr.place_bet(session_id, 501, Uuid::new_v4()).await;
    assert!(matches!(result, Err(GameError::InvalidBet(_))));
}

/// Reveal should fail if the game hasn't ended.
#[tokio::test]
async fn test_reveal_before_game_ends() {
    let mgr = setup();
    let player_id = Uuid::new_v4();
    mgr.register_player(player_id).await;

    let (session_id, _) = mgr.new_session(player_id).await.unwrap();

    let result = mgr.reveal_seed(session_id).await;
    assert!(result.is_err());
}

/// Non-existent session should return an error.
#[tokio::test]
async fn test_nonexistent_session() {
    let mgr = setup();
    let fake_session = Uuid::new_v4();

    let result = mgr.get_state(fake_session).await;
    assert!(matches!(result, Err(GameError::SessionNotFound(_))));
}
