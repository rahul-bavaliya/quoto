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
const NOT_INPUT: [&str; 3] = ["id", "country", "status"];

/// The body of a PATCH: a JSON object with at least one field. (An empty one changes nothing, so
/// it is refused as a mistake.) Checked before the database is touched.
pub(super) fn changes(
    input: Result<Json<Map<String, Value>>, JsonRejection>,
) -> Result<Map<String, Value>, ApiError> {
    let changes = body(input)?;
    if changes.is_empty() {
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
