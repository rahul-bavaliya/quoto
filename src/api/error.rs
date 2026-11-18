// src/api/error.rs: what the person gets back when a call fails
//
// Every failure has the same shape: {"code": "...", "message": "..."}. The code is the stable
// `QuoteError::code()`. Nothing about the database, the key or the request body is ever sent back.

use crate::errors::{ErrorKind, QuoteError};
use axum::Json;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use tracing::warn;
use utoipa::ToSchema;

pub(super) enum ApiError {
    /// Something Quoto's own rules refused (wrong input, a branch that doesn't exist...).
    Quote(QuoteError),
    /// The body is not the JSON this call expects.
    BadBody,
    /// No key, or the wrong one.
    Unauthorized,
}

impl From<QuoteError> for ApiError {
    fn from(error: QuoteError) -> Self {
        ApiError::Quote(error)
    }
}

/// What every failed call answers.
#[derive(Serialize, ToSchema)]
#[schema(example = json!({"code": "UNKNOWN_BRANCH", "message": "There is no branch with that id or number."}))]
pub(super) struct ErrorBody {
    /// A stable word for the problem (the same codes the library uses).
    code: &'static str,
    /// A short sentence for a person.
    message: String,
}

/// The HTTP status for a Quoto error: its kind decides, except two cases that have a better word.
fn status_of(error: &QuoteError) -> StatusCode {
    match error {
        QuoteError::UnknownBranch => StatusCode::NOT_FOUND,
        QuoteError::DuplicateBranchNumber { .. } => StatusCode::CONFLICT,
        _ => match error.kind() {
            ErrorKind::InvalidInput => StatusCode::BAD_REQUEST,
            ErrorKind::BusinessRule => StatusCode::CONFLICT,
            ErrorKind::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            ErrorKind::Configuration => StatusCode::INTERNAL_SERVER_ERROR,
        },
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Quote(error) => {
                error.log(); // the code only, never the message
                let status = status_of(&error);
                let body = ErrorBody {
                    code: error.code(),
                    message: error.to_string(),
                };
                (status, Json(body)).into_response()
            }
            ApiError::BadBody => {
                let body = ErrorBody {
                    code: "INVALID_REQUEST_BODY",
                    message: String::from("The request body is not valid JSON for this call."),
                };
                (StatusCode::BAD_REQUEST, Json(body)).into_response()
            }
            ApiError::Unauthorized => {
                warn!("api key missing or refused");
                let body = ErrorBody {
                    code: "UNAUTHORIZED",
                    message: String::from("A valid API key is needed."),
                };
                (
                    StatusCode::UNAUTHORIZED,
                    [(header::WWW_AUTHENTICATE, "Bearer")],
                    Json(body),
                )
                    .into_response()
            }
        }
    }
}
