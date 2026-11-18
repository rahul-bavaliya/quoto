// src/api.rs: the web API (REST + JSON, built on axum)
//
// The pieces live in `src/api/`: `settings` (address and key), `auth` (the key check),
// `error` (what a failed call returns) and `branches` (the branch calls).
// Every call needs the API key (`Authorization: Bearer <key>`).

mod auth;
mod branches;
mod error;
mod settings;

pub use settings::ApiSettings;

use crate::errors::{QuoteError, QuoteResult};
use crate::secret::Secret;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::middleware::from_fn_with_state;
use axum::routing::{get, patch};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tracing::{debug, info};

/// A branch is a few lines of text. Anything bigger than this is refused before it is read.
const MAX_BODY_BYTES: usize = 16 * 1024;

/// What every call can use: the database and the key.
#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) db: DatabaseConnection,
    pub(crate) key: Arc<Secret>,
}

/// All the routes. Tests call this directly (no network needed).
pub fn router(db: DatabaseConnection, key: Secret) -> Router {
    let state = AppState {
        db,
        key: Arc::new(key),
    };
    Router::new()
        .route("/branches", get(branches::list).post(branches::create))
        .route("/branches/{id}", get(branches::read).put(branches::replace))
        .route("/branches/{id}/status", patch(branches::change_status))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        // Added last, so it runs first: nothing is read or answered without the key.
        .layer(from_fn_with_state(state.clone(), auth::require_key))
        .with_state(state)
}

/// Starts the API and keeps it running until Ctrl+C.
pub async fn serve(settings: ApiSettings, db: DatabaseConnection) -> QuoteResult<()> {
    let address = settings.address();
    let app = router(db, settings.key);
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .map_err(|cause| {
            debug!(%cause, "could not listen");
            QuoteError::ServerStartFailed
        })?;
    info!(%address, "the API is listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .map_err(|cause| {
            debug!(%cause, "the server stopped with an error");
            QuoteError::ServerStartFailed
        })
}
