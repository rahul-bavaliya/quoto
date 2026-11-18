// src/api/log.rs: ONE log line for every call, holding only things that are safe to keep
//
//   INFO  api_request request_id=7 method=POST route=/branches status=201 elapsed_ms=4 code=none
//
// Logged: a counter number for this call (also sent back in the `x-request-id` header, so a
// person can say "call 7 failed"), the method, the route's *pattern*, the status, the time it
// took, and the error code if the call failed.
//
// Never logged, at any level: the key or any header, the request or response body, the query
// string, the real path (it holds a branch id, or whatever a stranger typed), who called, or
// anything about a branch. Those are the rules in `claude/logging-rules.md`; `tests/api_logging.rs`
// makes calls full of recognisable secret values and fails if any shows up in the log.

use axum::extract::{MatchedPath, Request};
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tracing::{error, info, warn};

/// The next call's number. Starts at 1 each time the server starts.
static NEXT_REQUEST: AtomicU64 = AtomicU64::new(1);

/// Rides on a failed response from the error to the log line (it is not sent to the caller;
/// the caller already has it in the body).
#[derive(Clone, Copy)]
pub(super) struct ErrorCode(pub(super) &'static str);

/// Wraps every call (it is the outermost layer, so a refused key is logged too).
pub(super) async fn log_request(request: Request, next: Next) -> Response {
    let started = Instant::now();
    // Our own number. A `x-request-id` the caller sends is ignored (it could be anything).
    let request_id = NEXT_REQUEST.fetch_add(1, Ordering::Relaxed);
    let method = request.method().clone();
    // The pattern ("/branches/{id}"), never the real path. A call that matched no route
    // is just "unmatched": a stranger's text never reaches the log.
    let route = request.extensions().get::<MatchedPath>().map_or_else(
        || String::from("unmatched"),
        |path| path.as_str().to_string(),
    );

    let mut response = next.run(request).await;

    let status = response.status().as_u16();
    let code = response
        .extensions()
        .get::<ErrorCode>()
        .map_or("none", |error| error.0);
    let elapsed_ms = started.elapsed().as_millis() as u64;
    // The caller's own mistakes are warnings; only our side failing is an error.
    match status {
        500.. => {
            error!(request_id, %method, route, status, elapsed_ms, code, "api_request")
        }
        400..=499 => {
            warn!(request_id, %method, route, status, elapsed_ms, code, "api_request")
        }
        _ => info!(request_id, %method, route, status, elapsed_ms, code, "api_request"),
    }

    response
        .headers_mut()
        .insert("x-request-id", HeaderValue::from(request_id));
    response
}
