use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

impl Rank {
    /// Blackjack point values. Ace returns 11; the hand evaluator handles
    /// the soft/hard distinction.
    pub fn value(self) -> u8 {
        match self {
            Rank::Two => 2,
            Rank::Three => 3,
            Rank::Four => 4,
            Rank::Five => 5,
            Rank::Six => 6,
            Rank::Seven => 7,
            Rank::Eight => 8,
            Rank::Nine => 9,
            Rank::Ten | Rank::Jack | Rank::Queen | Rank::King => 10,
            Rank::Ace => 11,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rank = match self.rank {
            Rank::Two => "2",
            Rank::Three => "3",
            Rank::Four => "4",
            Rank::Five => "5",
            Rank::Six => "6",
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine => "9",
            Rank::Ten => "10",
            Rank::Jack => "J",
            Rank::Queen => "Q",
            Rank::King => "K",
            Rank::Ace => "A",
        };
        let suit = match self.suit {
            Suit::Hearts => "\u{2665}",
            Suit::Diamonds => "\u{2666}",
            Suit::Clubs => "\u{2663}",
            Suit::Spades => "\u{2660}",
        };
        write!(f, "{rank}{suit}")
    }
}

/// Evaluate the best hand total, accounting for soft aces.
pub fn hand_value(cards: &[Card]) -> u8 {
    let mut total: u8 = 0;
    let mut aces: u8 = 0;

    for card in cards {
        total = total.saturating_add(card.rank.value());
        if card.rank == Rank::Ace {
            aces += 1;
        }
    }

    // Demote aces from 11 to 1 as needed.
    while total > 21 && aces > 0 {
        total -= 10;
        aces -= 1;
    }

    total
}

pub fn is_blackjack(cards: &[Card]) -> bool {
    cards.len() == 2 && hand_value(cards) == 21
}

pub fn is_bust(cards: &[Card]) -> bool {
    hand_value(cards) > 21
}

/// Build a standard 52-card deck.
pub fn standard_deck() -> Vec<Card> {
    let suits = [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades];
    let ranks = [
        Rank::Two,
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
        Rank::Ace,
    ];

    let mut deck = Vec::with_capacity(52);
    for &suit in &suits {
        for &rank in &ranks {
            deck.push(Card { rank, suit });
        }
    }
    deck
}

/// Player actions in blackjack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlackjackAction {
    Hit,
    Stand,
    Double,
}

impl fmt::Display for BlackjackAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlackjackAction::Hit => write!(f, "hit"),
            BlackjackAction::Stand => write!(f, "stand"),
            BlackjackAction::Double => write!(f, "double"),
        }
    }
}

/// Blackjack configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackjackConfig {
    pub deck_count: u8,
    pub dealer_hits_soft_17: bool,
}

impl Default for BlackjackConfig {
    fn default() -> Self {
        Self {
            deck_count: 1,
            dealer_hits_soft_17: true,
        }
    }
}

/// The view of the game that the player is allowed to see.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackjackView {
    pub player_cards: Vec<String>,
    pub player_total: u8,
    pub dealer_cards: Vec<String>,
    pub dealer_total: Option<u8>,
    pub phase: String,
    pub available_actions: Vec<String>,
    pub bet: u64,
    pub settlement: Option<SettlementInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementInfo {
    pub payout: i64,
    pub reason: String,
    pub dealer_total: u8,
    pub player_total: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hand_value_simple() {
        let cards = vec![
            Card { rank: Rank::Ten, suit: Suit::Spades },
            Card { rank: Rank::Seven, suit: Suit::Hearts },
        ];
        assert_eq!(hand_value(&cards), 17);
    }

    #[test]
    fn test_hand_value_soft_ace() {
        let cards = vec![
            Card { rank: Rank::Ace, suit: Suit::Spades },
            Card { rank: Rank::Six, suit: Suit::Hearts },
        ];
        assert_eq!(hand_value(&cards), 17);
    }

    #[test]
    fn test_hand_value_hard_ace() {
        let cards = vec![
            Card { rank: Rank::Ace, suit: Suit::Spades },
            Card { rank: Rank::Six, suit: Suit::Hearts },
            Card { rank: Rank::Eight, suit: Suit::Clubs },
        ];
        // 11 + 6 + 8 = 25 -> demote ace -> 1 + 6 + 8 = 15
        assert_eq!(hand_value(&cards), 15);
    }

    #[test]
    fn test_blackjack() {
        let cards = vec![
            Card { rank: Rank::Ace, suit: Suit::Spades },
            Card { rank: Rank::King, suit: Suit::Hearts },
        ];
        assert!(is_blackjack(&cards));
        assert!(!is_bust(&cards));
    }

    #[test]
    fn test_bust() {
        let cards = vec![
            Card { rank: Rank::Ten, suit: Suit::Spades },
            Card { rank: Rank::Ten, suit: Suit::Hearts },
            Card { rank: Rank::Five, suit: Suit::Clubs },
        ];
        assert!(is_bust(&cards));
    }

    #[test]
    fn test_standard_deck_size() {
        assert_eq!(standard_deck().len(), 52);
    }
}
