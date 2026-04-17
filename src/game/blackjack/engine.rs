use chrono::Utc;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use uuid::Uuid;

use crate::core::events::{CardRecipient, GameEvent};
use crate::game::blackjack::types::*;
use crate::game::state_machine::is_valid_transition;
use crate::game::traits::{GameEngine, GameError, GamePhase, SettlementResult};

/// Server-authoritative Blackjack engine.
///
/// All randomness is derived from the injected seed. Every card deal and
/// action is emitted as an event. The `player_view()` method hides the
/// dealer's hole card until it's the dealer's turn.
pub struct BlackjackEngine {
    phase: GamePhase,
    session_id: Uuid,
    player_cards: Vec<Card>,
    dealer_cards: Vec<Card>,
    shoe: Vec<Card>,
    bet: u64,
    actions_taken: u32,
    settlement_result: Option<SettlementResult>,
    config: BlackjackConfig,
}

impl BlackjackEngine {
    fn deal_card(&mut self) -> Card {
        self.shoe
            .pop()
            .expect("shoe should not be empty during a round")
    }

    fn transition(&mut self, to: GamePhase) -> Result<(), GameError> {
        if !is_valid_transition(self.phase, to) {
            return Err(GameError::InvalidAction {
                action: format!("transition to {to}"),
                phase: self.phase,
            });
        }
        self.phase = to;
        Ok(())
    }

    /// Place a bet (transitions Init -> BetPlaced).
    pub fn place_bet(
        &mut self,
        amount: u64,
        action_id: Uuid,
    ) -> Result<Vec<GameEvent>, GameError> {
        if self.phase != GamePhase::Init {
            return Err(GameError::InvalidAction {
                action: "place_bet".into(),
                phase: self.phase,
            });
        }

        self.bet = amount;
        self.transition(GamePhase::BetPlaced)?;

        let mut events = vec![GameEvent::BetPlaced {
            session_id: self.session_id,
            amount,
            action_id,
            timestamp: Utc::now(),
        }];

        // Deal initial cards: player, dealer, player, dealer.
        let p1 = self.deal_card();
        self.player_cards.push(p1);
        events.push(GameEvent::CardDealt {
            session_id: self.session_id,
            recipient: CardRecipient::Player,
            card: p1.to_string(),
            face_up: true,
            sequence: 1,
            timestamp: Utc::now(),
        });

        let d1 = self.deal_card();
        self.dealer_cards.push(d1);
        events.push(GameEvent::CardDealt {
            session_id: self.session_id,
            recipient: CardRecipient::Dealer,
            card: d1.to_string(),
            face_up: true,
            sequence: 2,
            timestamp: Utc::now(),
        });

        let p2 = self.deal_card();
        self.player_cards.push(p2);
        events.push(GameEvent::CardDealt {
            session_id: self.session_id,
            recipient: CardRecipient::Player,
            card: p2.to_string(),
            face_up: true,
            sequence: 3,
            timestamp: Utc::now(),
        });

        let d2 = self.deal_card();
        self.dealer_cards.push(d2);
        events.push(GameEvent::CardDealt {
            session_id: self.session_id,
            recipient: CardRecipient::Dealer,
            card: d2.to_string(),
            face_up: false, // hole card
            sequence: 4,
            timestamp: Utc::now(),
        });

        self.transition(GamePhase::PlayerTurn)?;

        // Check for natural blackjack.
        if is_blackjack(&self.player_cards) {
            self.transition(GamePhase::Settlement)?;
            let payout = if is_blackjack(&self.dealer_cards) {
                // Push -- return the bet.
                self.bet as i64
            } else {
                // Blackjack pays 3:2.
                self.bet as i64 + (self.bet as i64 * 3) / 2
            };
            let reason = if is_blackjack(&self.dealer_cards) {
                "push (both blackjack)"
            } else {
                "player blackjack (3:2)"
            };
            self.settlement_result = Some(SettlementResult {
                payout,
                reason: reason.into(),
            });
            events.push(GameEvent::Payout {
                session_id: self.session_id,
                amount: payout,
                reason: reason.into(),
                timestamp: Utc::now(),
            });
            self.transition(GamePhase::Closed)?;
            events.push(GameEvent::SessionEnded {
                session_id: self.session_id,
                timestamp: Utc::now(),
            });
        }

        Ok(events)
    }

    fn play_dealer(&mut self) -> Vec<GameEvent> {
        let mut events = Vec::new();

        // Dealer hits on soft 17 if configured.
        loop {
            let total = hand_value(&self.dealer_cards);
            let soft_17 = total == 17
                && self
                    .dealer_cards
                    .iter()
                    .any(|c| c.rank == Rank::Ace)
                && self.config.dealer_hits_soft_17;

            if total >= 17 && !soft_17 {
                break;
            }

            let card = self.deal_card();
            self.dealer_cards.push(card);
            events.push(GameEvent::CardDealt {
                session_id: self.session_id,
                recipient: CardRecipient::Dealer,
                card: card.to_string(),
                face_up: true,
                sequence: 0, // sequence isn't critical for dealer play
                timestamp: Utc::now(),
            });
            events.push(GameEvent::DealerAction {
                session_id: self.session_id,
                action: "hit".into(),
                timestamp: Utc::now(),
            });
        }

        events
    }

    fn settle(&mut self) -> Vec<GameEvent> {
        let player_total = hand_value(&self.player_cards);
        let dealer_total = hand_value(&self.dealer_cards);

        let (payout, reason) = if is_bust(&self.dealer_cards) {
            (self.bet as i64 * 2, "dealer bust".to_string())
        } else if player_total > dealer_total {
            (self.bet as i64 * 2, "player wins".to_string())
        } else if player_total == dealer_total {
            (self.bet as i64, "push".to_string())
        } else {
            (0, "dealer wins".to_string())
        };

        self.settlement_result = Some(SettlementResult {
            payout,
            reason: reason.clone(),
        });

        vec![GameEvent::Payout {
            session_id: self.session_id,
            amount: payout,
            reason,
            timestamp: Utc::now(),
        }]
    }

    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    pub fn bet_amount(&self) -> u64 {
        self.bet
    }
}

impl GameEngine for BlackjackEngine {
    type Action = BlackjackAction;
    type State = BlackjackView;
    type Config = BlackjackConfig;

    fn new_round(config: &BlackjackConfig, rng_seed: [u8; 32]) -> Self {
        let mut rng = ChaCha20Rng::from_seed(rng_seed);

        // Build shoe from `deck_count` decks and shuffle with the CSPRNG.
        let mut shoe: Vec<Card> = (0..config.deck_count)
            .flat_map(|_| standard_deck())
            .collect();
        shoe.shuffle(&mut rng);

        Self {
            phase: GamePhase::Init,
            session_id: Uuid::new_v4(),
            player_cards: Vec::new(),
            dealer_cards: Vec::new(),
            shoe,
            bet: 0,
            actions_taken: 0,
            settlement_result: None,
            config: config.clone(),
        }
    }

    fn phase(&self) -> GamePhase {
        self.phase
    }

    fn valid_actions(&self) -> Vec<BlackjackAction> {
        match self.phase {
            GamePhase::PlayerTurn => {
                let mut actions = vec![BlackjackAction::Hit, BlackjackAction::Stand];
                // Double is only valid as the first player action.
                if self.actions_taken == 0 && self.player_cards.len() == 2 {
                    actions.push(BlackjackAction::Double);
                }
                actions
            }
            _ => Vec::new(),
        }
    }

    fn apply_action(&mut self, action: BlackjackAction) -> Result<Vec<GameEvent>, GameError> {
        if self.phase != GamePhase::PlayerTurn {
            return Err(GameError::InvalidAction {
                action: action.to_string(),
                phase: self.phase,
            });
        }

        if !self.valid_actions().contains(&action) {
            return Err(GameError::InvalidAction {
                action: action.to_string(),
                phase: self.phase,
            });
        }

        self.actions_taken += 1;
        let mut events = Vec::new();

        match action {
            BlackjackAction::Hit => {
                events.push(GameEvent::PlayerAction {
                    session_id: self.session_id,
                    action: "hit".into(),
                    action_id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                });

                let card = self.deal_card();
                self.player_cards.push(card);
                events.push(GameEvent::CardDealt {
                    session_id: self.session_id,
                    recipient: CardRecipient::Player,
                    card: card.to_string(),
                    face_up: true,
                    sequence: 0,
                    timestamp: Utc::now(),
                });

                if is_bust(&self.player_cards) {
                    self.transition(GamePhase::Settlement)?;
                    self.settlement_result = Some(SettlementResult {
                        payout: 0,
                        reason: "player bust".into(),
                    });
                    events.push(GameEvent::Payout {
                        session_id: self.session_id,
                        amount: 0,
                        reason: "player bust".into(),
                        timestamp: Utc::now(),
                    });
                    self.transition(GamePhase::Closed)?;
                    events.push(GameEvent::SessionEnded {
                        session_id: self.session_id,
                        timestamp: Utc::now(),
                    });
                }
            }
            BlackjackAction::Stand => {
                events.push(GameEvent::PlayerAction {
                    session_id: self.session_id,
                    action: "stand".into(),
                    action_id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                });

                self.transition(GamePhase::DealerTurn)?;
                events.extend(self.play_dealer());
                self.transition(GamePhase::Settlement)?;
                events.extend(self.settle());
                self.transition(GamePhase::Closed)?;
                events.push(GameEvent::SessionEnded {
                    session_id: self.session_id,
                    timestamp: Utc::now(),
                });
            }
            BlackjackAction::Double => {
                self.bet *= 2;
                events.push(GameEvent::PlayerAction {
                    session_id: self.session_id,
                    action: "double".into(),
                    action_id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                });
                events.push(GameEvent::BetPlaced {
                    session_id: self.session_id,
                    amount: self.bet / 2, // the additional bet
                    action_id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                });

                let card = self.deal_card();
                self.player_cards.push(card);
                events.push(GameEvent::CardDealt {
                    session_id: self.session_id,
                    recipient: CardRecipient::Player,
                    card: card.to_string(),
                    face_up: true,
                    sequence: 0,
                    timestamp: Utc::now(),
                });

                if is_bust(&self.player_cards) {
                    self.transition(GamePhase::Settlement)?;
                    self.settlement_result = Some(SettlementResult {
                        payout: 0,
                        reason: "player bust (after double)".into(),
                    });
                    events.push(GameEvent::Payout {
                        session_id: self.session_id,
                        amount: 0,
                        reason: "player bust (after double)".into(),
                        timestamp: Utc::now(),
                    });
                } else {
                    self.transition(GamePhase::DealerTurn)?;
                    events.extend(self.play_dealer());
                    self.transition(GamePhase::Settlement)?;
                    events.extend(self.settle());
                }
                self.transition(GamePhase::Closed)?;
                events.push(GameEvent::SessionEnded {
                    session_id: self.session_id,
                    timestamp: Utc::now(),
                });
            }
        }

        Ok(events)
    }

    fn player_view(&self) -> BlackjackView {
        let show_dealer = matches!(
            self.phase,
            GamePhase::DealerTurn | GamePhase::Settlement | GamePhase::Closed
        );

        let dealer_cards: Vec<String> = self
            .dealer_cards
            .iter()
            .enumerate()
            .map(|(i, c)| {
                if show_dealer || i == 0 {
                    c.to_string()
                } else {
                    "??".to_string()
                }
            })
            .collect();

        let dealer_total = if show_dealer {
            Some(hand_value(&self.dealer_cards))
        } else {
            None
        };

        let settlement = self.settlement_result.as_ref().map(|s| SettlementInfo {
            payout: s.payout,
            reason: s.reason.clone(),
            dealer_total: hand_value(&self.dealer_cards),
            player_total: hand_value(&self.player_cards),
        });

        BlackjackView {
            player_cards: self.player_cards.iter().map(|c| c.to_string()).collect(),
            player_total: hand_value(&self.player_cards),
            dealer_cards,
            dealer_total,
            phase: self.phase.to_string(),
            available_actions: self
                .valid_actions()
                .iter()
                .map(|a| a.to_string())
                .collect(),
            bet: self.bet,
            settlement,
        }
    }

    fn settlement(&self) -> Option<SettlementResult> {
        self.settlement_result.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_round_starts_in_init() {
        let config = BlackjackConfig::default();
        let engine = BlackjackEngine::new_round(&config, [1u8; 32]);
        assert_eq!(engine.phase(), GamePhase::Init);
    }

    #[test]
    fn test_place_bet_transitions_correctly() {
        let config = BlackjackConfig::default();
        let mut engine = BlackjackEngine::new_round(&config, [1u8; 32]);
        let events = engine.place_bet(100, Uuid::new_v4()).unwrap();

        // Should have at least bet + 4 card deals.
        assert!(events.len() >= 5);

        // Phase should be PlayerTurn or Settlement/Closed (if blackjack).
        assert!(matches!(
            engine.phase(),
            GamePhase::PlayerTurn | GamePhase::Settlement | GamePhase::Closed
        ));
    }

    #[test]
    fn test_determinism() {
        let config = BlackjackConfig::default();
        let seed = [42u8; 32];

        let mut e1 = BlackjackEngine::new_round(&config, seed);
        let mut e2 = BlackjackEngine::new_round(&config, seed);

        e1.place_bet(10, Uuid::new_v4()).unwrap();
        e2.place_bet(10, Uuid::new_v4()).unwrap();

        let v1 = e1.player_view();
        let v2 = e2.player_view();

        assert_eq!(v1.player_cards, v2.player_cards);
        assert_eq!(v1.dealer_cards, v2.dealer_cards);
    }

    #[test]
    fn test_action_before_bet_fails() {
        let config = BlackjackConfig::default();
        let mut engine = BlackjackEngine::new_round(&config, [1u8; 32]);
        let result = engine.apply_action(BlackjackAction::Hit);
        assert!(result.is_err());
    }
}
