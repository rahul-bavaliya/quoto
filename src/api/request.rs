// src/api/request.rs: what every call with a body shares
//
// `body` (a body that isn't the expected JSON is one plain answer), `changes` and `merge` (a PATCH:
// only the sent fields change, checked by the same rules as a POST) and `StatusInput` (the body
// of every `.../status` call). Nothing here knows about a branch or a customer.

use super::error::ApiError;
use axum::Json;
use axum::extract::rejection::JsonRejection;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({"status": "inactive"}))]
pub(super) struct StatusInput {
    /// "active" or "inactive".
    pub(super) status: String,
}

/// A body that isn't the expected JSON is one plain answer, not serde's wording.
pub(super) fn body<T>(input: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    input
        .map(|Json(value)| value)
        .map_err(|_| ApiError::BadBody)
}

/// The parts of a view that are not part of an input: a PATCH can't change them.
/// (The owner of a machine and where its details came from are changed by their own calls.)
const NOT_INPUT: [&str; 5] = ["id", "country", "status", "customer_id", "source"];

/// A page is this long unless the caller asks for fewer or more (at most `MAX_PAGE`).
const DEFAULT_PAGE: u64 = 50;
const MAX_PAGE: u64 = 200;

/// How many records one page of a list holds: what was asked for, between 1 and `MAX_PAGE`.
pub(super) fn page_size(asked: Option<u64>) -> u64 {
    asked.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE)
}

/// The body of a PATCH: a JSON object with at least one field. (An empty one changes nothing, so
/// it is refused as a mistake, and so is a field that is not an input.) Checked before the database is touched.
pub(super) fn changes(
    input: Result<Json<Map<String, Value>>, JsonRejection>,
) -> Result<Map<String, Value>, ApiError> {
    let changes = body(input)?;
    if changes.is_empty() || NOT_INPUT.iter().any(|key| changes.contains_key(*key)) {
        return Err(ApiError::BadBody);
    }
    Ok(changes)
}

/// Lays the sent fields over the saved ones and reads the result as a whole input, so a PATCH
/// goes through exactly the same rules as a POST or a PUT. A field that is left out keeps its
/// saved value; `null` clears an optional field and is refused for a required one; a field that
/// isn't an input (an `id`, a `status`...) is refused.
pub(super) fn merge<T: DeserializeOwned>(
    saved: &impl Serialize,
    changes: Map<String, Value>,
) -> Result<T, ApiError> {
    let mut merged = match serde_json::to_value(saved) {
        Ok(Value::Object(fields)) => fields,
        _ => Map::new(), // can't happen; and an empty one fails below instead of saving anything
    };
    for key in NOT_INPUT {
        merged.remove(key);
    }
    merged.extend(changes);
    serde_json::from_value(Value::Object(merged)).map_err(|_| ApiError::BadBody)
}
