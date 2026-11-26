// src/api.rs: the web API (REST + JSON, built on axum)
//
// The pieces live in `src/api/`:
//   settings   the address and the key
//   auth       the key check
//   error      what a failed call returns
//   log        the one log line per call
//   request    what calls with a body share (body, PATCH merge, status body)
//   branches   the branch calls
//   provinces  the province and tax rate calls
//   docs       the Swagger page at /docs
// Every call needs the API key (`Authorization: Bearer <key>`); only the documentation
// addresses don't.

mod auth;
mod branches;
mod docs;
mod error;
mod log;
mod provinces;
mod request;
mod settings;

pub use settings::ApiSettings;

use crate::errors::{AppError, AppResult};
use crate::secret::Secret;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::routing::{get, patch};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tracing::{debug, info};

/// A request body is a few lines of text. Anything bigger than this is refused before it is read.
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
        .route(
            "/branches/{id}",
            get(branches::read)
                .put(branches::replace)
                .patch(branches::change),
        )
        .route("/branches/{id}/status", patch(branches::change_status))
        .route("/provinces", get(provinces::list))
        .route("/provinces/{code}", get(provinces::read))
        .route("/provinces/{code}/rates", patch(provinces::change_rates))
        .merge(docs::routes())
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        // Added last, so it runs first: nothing is read or answered without the key.
        .layer(from_fn_with_state(state.clone(), auth::require_key))
        // Outermost, so every call is logged, including the ones the key check turns away.
        .layer(from_fn(log::log_request))
        .with_state(state)
}

/// Starts the API and keeps it running until Ctrl+C.
pub async fn serve(settings: ApiSettings, db: DatabaseConnection) -> AppResult<()> {
    let address = settings.address();
    let app = router(db, settings.key);
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .map_err(|cause| {
            debug!(%cause, "could not listen");
            AppError::ServerStartFailed
        })?;
    info!(%address, "the API is listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .map_err(|cause| {
            debug!(%cause, "the server stopped with an error");
            AppError::ServerStartFailed
        })
}
