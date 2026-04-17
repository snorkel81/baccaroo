use std::collections::HashMap;
use uuid::Uuid;

/// Behavioral risk scoring. Tracks win/loss history per player and computes
/// a risk score. This is a stub for the initial commit -- the interface is
/// defined but the scoring model is intentionally simple.
#[derive(Debug, Default)]
pub struct RiskScorer {
    records: HashMap<Uuid, PlayerRecord>,
}

#[derive(Debug, Default)]
struct PlayerRecord {
    wins: u64,
    losses: u64,
    total_wagered: u64,
    total_won: u64,
}

impl RiskScorer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_outcome(
        &mut self,
        player_id: Uuid,
        wagered: u64,
        payout: u64,
    ) {
        let record = self.records.entry(player_id).or_default();
        record.total_wagered += wagered;
        record.total_won += payout;
        if payout > wagered {
            record.wins += 1;
        } else {
            record.losses += 1;
        }
    }

    /// Compute a simple risk score (0.0 = low risk, 1.0 = high risk).
    /// This is a placeholder; a real implementation would use statistical
    /// models.
    pub fn risk_score(&self, player_id: Uuid) -> f64 {
        let record = match self.records.get(&player_id) {
            Some(r) => r,
            None => return 0.0,
        };

        let total_games = record.wins + record.losses;
        if total_games < 10 {
            return 0.0; // Not enough data.
        }

        let win_rate = record.wins as f64 / total_games as f64;
        // Expected blackjack win rate is roughly 42-43%.
        let expected = 0.42;
        let deviation = (win_rate - expected).abs();

        // Simple linear mapping: 10% deviation = 0.5 risk, 20% = 1.0.
        (deviation / 0.20).min(1.0)
    }
}
