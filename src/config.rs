/// Application configuration.
///
/// All values have sensible defaults for development. In production these
/// would be loaded from environment variables or a config file.

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub initial_balance: u64,
    pub min_bet: u64,
    pub max_bet: u64,
    pub session_timeout_secs: u64,
    pub rate_limit_per_second: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 3000,
            initial_balance: 1000,
            min_bet: 1,
            max_bet: 500,
            session_timeout_secs: 300,
            rate_limit_per_second: 10,
        }
    }
}
