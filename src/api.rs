// src/api.rs: the web API (REST + JSON, built on axum)
//
// The pieces live in `src/api/`:
//   settings   the address and the key
//   auth       the key check
//   error      what a failed call returns
//   log        the one log line per call
//   request    what calls with a body share (body, PATCH merge, status body)
//   branches   the branch calls
//   customers  the customer calls
//   equipment  the machine calls
//   quotes     the quote calls
//   docs       the Swagger page at /docs
// Every call needs the API key (`Authorization: Bearer <key>`); only the documentation
// addresses don't.

mod auth;
mod branches;
mod customers;
mod docs;
mod equipment;
mod error;
mod log;
mod quotes;
mod request;
mod settings;

pub use settings::ApiSettings;

use crate::errors::{QuoteError, QuoteResult};
use crate::secret::Secret;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::routing::{get, patch, post};
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
        .route(
            "/branches/{id}",
            get(branches::read)
                .put(branches::replace)
                .patch(branches::change),
        )
        .route("/branches/{id}/status", patch(branches::change_status))
        .route("/customers", get(customers::list).post(customers::create))
        .route(
            "/customers/{id}",
            get(customers::read)
                .put(customers::replace)
                .patch(customers::change),
        )
        .route("/customers/{id}/status", patch(customers::change_status))
        .route("/equipment", get(equipment::list).post(equipment::create))
        .route(
            "/equipment/{id}",
            get(equipment::read)
                .put(equipment::replace)
                .patch(equipment::change)
                .delete(equipment::archive),
        )
        .route("/equipment/{id}/status", patch(equipment::change_status))
        .route("/equipment/{id}/owner", patch(equipment::change_owner))
        .route("/quotes", get(quotes::list).post(quotes::create))
        .route("/quotes/{id}", get(quotes::read).patch(quotes::change))
        .route("/quotes/{id}/lines", post(quotes::add_line))
        .route("/quotes/{id}/customer", patch(quotes::add_customer))
        .route("/quotes/{id}/equipment", patch(quotes::add_equipment))
        .route("/quotes/{id}/accept", post(quotes::accept))
        .route("/quotes/{id}/reject", post(quotes::reject))
        .route("/quotes/{id}/reopen", post(quotes::reopen))
        .merge(docs::routes())
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        // Added last, so it runs first: nothing is read or answered without the key.
        .layer(from_fn_with_state(state.clone(), auth::require_key))
        // Outermost, so every call is logged, including the ones the key check turns away.
        .layer(from_fn(log::log_request))
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
