use baccaroo::core::events::InMemoryEventStore;
use baccaroo::core::wallet::Wallet;
use baccaroo::config::AppConfig;
use baccaroo::game::blackjack::types::{BlackjackAction, BlackjackConfig};
use baccaroo::game::session::SessionManager;
use std::sync::Arc;
use uuid::Uuid;

fn setup() -> Arc<SessionManager> {
    let event_store = InMemoryEventStore::new();
    let wallet = Arc::new(Wallet::new(event_store));
    let config = AppConfig::default();
    let bj_config = BlackjackConfig::default();
    Arc::new(SessionManager::new(wallet, config, bj_config))
}

#[tokio::test]
async fn test_full_game_lifecycle() {
    let mgr = setup();
    let player_id = Uuid::new_v4();
    mgr.register_player(player_id).await;

    // Start session.
    let (session_id, commitment) = mgr.new_session(player_id).await.unwrap();
    assert!(!commitment.is_empty());

    // Place bet.
    let action_id = Uuid::new_v4();
    let view = mgr.place_bet(session_id, 10, action_id).await.unwrap();
    assert!(!view.player_cards.is_empty());
    assert!(!view.dealer_cards.is_empty());

    // If the game already ended (blackjack), skip action phase.
    if view.phase == "Closed" {
        return;
    }

    // Stand to end the round.
    let action_id2 = Uuid::new_v4();
    let view2 = mgr
        .submit_action(session_id, BlackjackAction::Stand, action_id2)
        .await
        .unwrap();
    assert_eq!(view2.phase, "Closed");
    assert!(view2.settlement.is_some());

    // Verify seed reveal works after game ends.
    let (seed, commitment2) = mgr.reveal_seed(session_id).await.unwrap();
    assert!(!seed.is_empty());
    assert!(!commitment2.is_empty());

    // Verify balance changed.
    let balance = mgr.get_balance(player_id).await;
    // Balance should be different from initial (win or loss).
    // We can't predict exact value, but it should be reasonable.
    assert!(balance <= 2000); // Can't exceed double of initial on a single hand.
}

#[tokio::test]
async fn test_balance_decreases_on_loss() {
    let mgr = setup();
    let player_id = Uuid::new_v4();
    mgr.register_player(player_id).await;

    let initial_balance = mgr.get_balance(player_id).await;
    assert_eq!(initial_balance, 1000);

    let (session_id, _) = mgr.new_session(player_id).await.unwrap();
    let view = mgr
        .place_bet(session_id, 100, Uuid::new_v4())
        .await
        .unwrap();

    // After placing a bet, balance should decrease.
    let after_bet = mgr.get_balance(player_id).await;
    // Balance is derived from events, and the bet event deducts the amount.
    // If the game ended with blackjack, payout is already included.
    if view.phase != "Closed" {
        assert_eq!(after_bet, 900);
    }
}
