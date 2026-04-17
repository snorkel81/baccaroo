use axum::{middleware, routing::{get, post}, Router};
use std::sync::Arc;
use tower_http::services::ServeDir;

use baccaroo::api::middleware::{auth_check, request_logging};
use baccaroo::api::routes;
use baccaroo::config::AppConfig;
use baccaroo::core::events::InMemoryEventStore;
use baccaroo::core::wallet::Wallet;
use baccaroo::game::blackjack::types::BlackjackConfig;
use baccaroo::game::session::SessionManager;
use baccaroo::observability::logging::init_logging;

#[tokio::main]
async fn main() {
    init_logging();

    let config = AppConfig::default();
    let addr = format!("{}:{}", config.host, config.port);

    let event_store = InMemoryEventStore::new();
    let wallet = Arc::new(Wallet::new(event_store));
    let blackjack_config = BlackjackConfig::default();
    let session_manager = Arc::new(SessionManager::new(wallet, config, blackjack_config));

    let app = Router::new()
        // API routes.
        .route("/api/session/new", post(routes::new_session))
        .route("/api/session/{id}/bet", post(routes::place_bet))
        .route("/api/session/{id}/action", post(routes::submit_action))
        .route("/api/session/{id}/state", get(routes::get_state))
        .route("/api/session/{id}/reveal", get(routes::reveal_seed))
        .route("/api/balance/{player_id}", get(routes::get_balance))
        // Health check.
        .route("/health", get(|| async { "ok" }))
        // Static files for the web UI.
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/", ServeDir::new("static"))
        // Middleware (applied bottom-up).
        .layer(middleware::from_fn(auth_check))
        .layer(middleware::from_fn(request_logging))
        .with_state(session_manager);

    tracing::info!(addr = %addr, "starting baccaroo server");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}
