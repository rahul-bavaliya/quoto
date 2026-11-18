// src/api/error.rs: what the person gets back when a call fails
//
// Every failure has the same shape: {"code": "...", "message": "..."}. The code is the stable
// `QuoteError::code()`. Nothing about the database, the key or the request body is ever sent back.

use super::log::ErrorCode;
use crate::errors::{ErrorKind, QuoteError};
use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

pub(super) enum ApiError {
    /// Something Quoto's own rules refused (wrong input, a branch that doesn't exist...).
    Quote(QuoteError),
    /// The body is not the JSON this call expects.
    BadBody,
    /// The query string (the part after `?`) is not what this call expects.
    BadQuery,
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
        QuoteError::UnknownBranch | QuoteError::UnknownCustomer => StatusCode::NOT_FOUND,
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
        let (status, code, message) = match &self {
            ApiError::Quote(error) => (status_of(error), error.code(), error.to_string()),
            ApiError::BadBody => (
                StatusCode::BAD_REQUEST,
                "INVALID_REQUEST_BODY",
                String::from("The request body is not valid JSON for this call."),
            ),
            ApiError::BadQuery => (
                StatusCode::BAD_REQUEST,
                "INVALID_QUERY",
                String::from("The query string is not valid for this call."),
            ),
            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                String::from("A valid API key is needed."),
            ),
        };
        let mut response = (status, Json(ErrorBody { code, message })).into_response();
        if matches!(self, ApiError::Unauthorized) {
            response
                .headers_mut()
                .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        }
        // The one place that writes the error to the log is `log::log_request`: it reads this
        // code, so a failed call is one line, not two.
        response.extensions_mut().insert(ErrorCode(code));
        response
    }
}
