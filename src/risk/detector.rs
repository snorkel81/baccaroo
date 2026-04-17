use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

/// Signals that the risk engine can flag.
#[derive(Debug, Clone, Serialize)]
pub enum RiskSignal {
    /// Action intervals are suspiciously consistent (bot-like).
    PerfectTiming { avg_interval_ms: u64, variance_ms: u64 },
    /// Player has been active for an unusually long period.
    ExtendedPlay { duration_mins: u64 },
    /// Win rate deviates significantly from expected odds.
    StatisticalAnomaly { win_rate: f64, expected: f64 },
}

/// Tracks per-player action timestamps for basic timing analysis.
#[derive(Debug, Default)]
pub struct RiskDetector {
    action_times: HashMap<Uuid, Vec<u64>>,
}

impl RiskDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record the timestamp (epoch millis) of an action for a player.
    pub fn record_action(&mut self, player_id: Uuid, timestamp_ms: u64) {
        self.action_times
            .entry(player_id)
            .or_default()
            .push(timestamp_ms);
    }

    /// Analyze a player's action timing for bot-like patterns.
    pub fn analyze_timing(&self, player_id: Uuid) -> Option<RiskSignal> {
        let times = self.action_times.get(&player_id)?;
        if times.len() < 5 {
            return None;
        }

        let intervals: Vec<u64> = times.windows(2).map(|w| w[1] - w[0]).collect();
        let avg = intervals.iter().sum::<u64>() / intervals.len() as u64;
        let variance = intervals
            .iter()
            .map(|&i| {
                let diff = i as i64 - avg as i64;
                (diff * diff) as u64
            })
            .sum::<u64>()
            / intervals.len() as u64;

        // Flag if variance is very low (suspiciously consistent timing).
        if variance < 100 && avg < 500 {
            return Some(RiskSignal::PerfectTiming {
                avg_interval_ms: avg,
                variance_ms: variance,
            });
        }

        None
    }
}
