use baccaroo::game::blackjack::engine::BlackjackEngine;
use baccaroo::game::blackjack::types::BlackjackConfig;
use baccaroo::game::traits::GameEngine;
use uuid::Uuid;

/// Same seed must always produce the same card sequence and game outcome.
#[test]
fn test_same_seed_same_outcome() {
    let config = BlackjackConfig::default();
    let seed = [99u8; 32];

    let mut engine1 = BlackjackEngine::new_round(&config, seed);
    let mut engine2 = BlackjackEngine::new_round(&config, seed);

    engine1.place_bet(10, Uuid::new_v4()).unwrap();
    engine2.place_bet(10, Uuid::new_v4()).unwrap();

    let view1 = engine1.player_view();
    let view2 = engine2.player_view();

    assert_eq!(view1.player_cards, view2.player_cards);
    assert_eq!(view1.dealer_cards, view2.dealer_cards);
    assert_eq!(view1.player_total, view2.player_total);
}

/// Different seeds must produce different outcomes (with overwhelming probability).
#[test]
fn test_different_seeds_different_outcomes() {
    let config = BlackjackConfig::default();

    let mut engine1 = BlackjackEngine::new_round(&config, [1u8; 32]);
    let mut engine2 = BlackjackEngine::new_round(&config, [2u8; 32]);

    engine1.place_bet(10, Uuid::new_v4()).unwrap();
    engine2.place_bet(10, Uuid::new_v4()).unwrap();

    let view1 = engine1.player_view();
    let view2 = engine2.player_view();

    // Extremely unlikely that two different seeds produce identical hands.
    let same = view1.player_cards == view2.player_cards
        && view1.dealer_cards == view2.dealer_cards;
    assert!(!same, "different seeds produced identical hands");
}

/// Verify commit-reveal integrity.
#[test]
fn test_commit_reveal_integrity() {
    use baccaroo::core::rng::CommitRevealRng;

    let rng = CommitRevealRng::new();
    let commitment = rng.commitment().to_string();
    let seed = rng.reveal_seed();

    assert!(
        CommitRevealRng::verify(&seed, &commitment),
        "commit-reveal verification failed"
    );
}
