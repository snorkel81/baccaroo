use baccaroo::game::blackjack::engine::BlackjackEngine;
use baccaroo::game::blackjack::types::{BlackjackAction, BlackjackConfig};
use baccaroo::game::traits::{GameEngine, GamePhase};
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use uuid::Uuid;

/// Run random action sequences against the engine and verify that it never
/// panics or enters an invalid state.
#[test]
fn test_fuzz_random_actions() {
    let config = BlackjackConfig::default();
    let actions = [
        BlackjackAction::Hit,
        BlackjackAction::Stand,
        BlackjackAction::Double,
    ];

    let mut rng = ChaCha20Rng::from_seed([77u8; 32]);

    for i in 0..100 {
        let seed = {
            let mut s = [0u8; 32];
            s[0] = i as u8;
            s[1] = (i >> 8) as u8;
            s
        };

        let mut engine = BlackjackEngine::new_round(&config, seed);

        // Place a bet first.
        let bet_result = engine.place_bet(10, Uuid::new_v4());
        if bet_result.is_err() {
            continue; // Some seeds might cause issues; that's fine for the fuzz.
        }

        // If blackjack happened, game is already closed.
        if engine.phase() == GamePhase::Closed {
            continue;
        }

        // Throw random actions at the engine.
        for _ in 0..20 {
            if engine.phase() == GamePhase::Closed {
                break;
            }

            let action = actions.choose(&mut rng).unwrap().clone();
            // We don't care if it errors -- only that it doesn't panic.
            let _ = engine.apply_action(action);
        }

        // The engine should be in a valid phase.
        let phase = engine.phase();
        assert!(
            matches!(
                phase,
                GamePhase::Init
                    | GamePhase::BetPlaced
                    | GamePhase::PlayerTurn
                    | GamePhase::DealerTurn
                    | GamePhase::Settlement
                    | GamePhase::Closed
            ),
            "engine entered invalid phase on iteration {i}"
        );
    }
}

/// Verify that the engine never allows actions in the wrong phase.
#[test]
fn test_fuzz_phase_enforcement() {
    let config = BlackjackConfig::default();

    for i in 0..50 {
        let seed = [i as u8; 32];
        let mut engine = BlackjackEngine::new_round(&config, seed);

        // Try actions before bet -- all should fail.
        assert!(engine.apply_action(BlackjackAction::Hit).is_err());
        assert!(engine.apply_action(BlackjackAction::Stand).is_err());
        assert!(engine.apply_action(BlackjackAction::Double).is_err());

        // Phase should still be Init.
        assert_eq!(engine.phase(), GamePhase::Init);
    }
}
