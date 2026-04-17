use tracing_subscriber::{fmt, EnvFilter};

/// Initialize structured logging with `tracing`.
///
/// Log level defaults to INFO and can be overridden via the `RUST_LOG`
/// environment variable. Output is JSON-formatted for machine consumption.
pub fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .json()
        .init();
}
