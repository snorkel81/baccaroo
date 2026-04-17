use crate::game::traits::GamePhase;
use tracing::warn;

/// Validates FSM transitions. Any transition not explicitly allowed here is
/// rejected. Invalid attempts are logged at WARN level (Design Rule 2 + 8).
pub fn is_valid_transition(from: GamePhase, to: GamePhase) -> bool {
    let valid = matches!(
        (from, to),
        (GamePhase::Init, GamePhase::BetPlaced)
            | (GamePhase::BetPlaced, GamePhase::PlayerTurn)
            | (GamePhase::PlayerTurn, GamePhase::PlayerTurn)   // hit
            | (GamePhase::PlayerTurn, GamePhase::DealerTurn)   // stand / double
            | (GamePhase::PlayerTurn, GamePhase::Settlement)   // bust / blackjack
            | (GamePhase::DealerTurn, GamePhase::Settlement)   // dealer finishes
            | (GamePhase::Settlement, GamePhase::Closed)       // settle
    );

    if !valid {
        warn!(
            from = %from,
            to = %to,
            "rejected invalid state transition"
        );
    }

    valid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        assert!(is_valid_transition(GamePhase::Init, GamePhase::BetPlaced));
        assert!(is_valid_transition(GamePhase::BetPlaced, GamePhase::PlayerTurn));
        assert!(is_valid_transition(GamePhase::PlayerTurn, GamePhase::PlayerTurn));
        assert!(is_valid_transition(GamePhase::PlayerTurn, GamePhase::DealerTurn));
        assert!(is_valid_transition(GamePhase::PlayerTurn, GamePhase::Settlement));
        assert!(is_valid_transition(GamePhase::DealerTurn, GamePhase::Settlement));
        assert!(is_valid_transition(GamePhase::Settlement, GamePhase::Closed));
    }

    #[test]
    fn test_invalid_transitions() {
        assert!(!is_valid_transition(GamePhase::Init, GamePhase::PlayerTurn));
        assert!(!is_valid_transition(GamePhase::Closed, GamePhase::Init));
        assert!(!is_valid_transition(GamePhase::Settlement, GamePhase::PlayerTurn));
        assert!(!is_valid_transition(GamePhase::DealerTurn, GamePhase::PlayerTurn));
    }
}
