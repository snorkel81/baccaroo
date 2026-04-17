use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Stub metrics collector. In production this would export to
/// Prometheus / Datadog / etc. For the initial commit it provides
/// an in-memory counters API.
#[derive(Debug, Clone)]
pub struct Metrics {
    counters: Arc<Mutex<HashMap<String, Arc<AtomicU64>>>>,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn increment(&self, name: &str) {
        let mut map = self.counters.lock().await;
        let counter = map
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(AtomicU64::new(0)));
        counter.fetch_add(1, Ordering::Relaxed);
    }

    pub async fn get(&self, name: &str) -> u64 {
        let map = self.counters.lock().await;
        map.get(name)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}
