//! Loopback HTTP transport. Socket I/O and long waits never hold match authority.

use std::io::Write;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::to_bytes;
use axum::extract::{Request, State};
use axum::http::{Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::{Semaphore, watch};
use tokio::time::{Instant, timeout, timeout_at};

use crate::{Host, HostError, number};

const MAX_REQUEST_BYTES: usize = 1_048_576;
const MAX_WAITERS: usize = 32;
const MAX_REQUESTS: usize = 64;
const BODY_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Command {
    method: String,
    #[serde(default = "empty_args")]
    args: Value,
}

fn empty_args() -> Value {
    json!({})
}

#[derive(Clone)]
struct Authority {
    host: Arc<Mutex<Host>>,
    changed: watch::Sender<u64>,
    closed: watch::Sender<bool>,
    requests: Arc<Semaphore>,
    waiters: Arc<Semaphore>,
}

/// Serves until the organizer closes the match. Writes no files.
///
/// # Errors
/// Returns an error if listening, startup output, or serving fails.
pub fn serve(host: Host, port: u16) -> Result<(), String> {
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|error| error.to_string())?;
    let url = format!(
        "http://{}",
        listener.local_addr().map_err(|error| error.to_string())?
    );
    println!("{}", host.launch_info(&url));
    std::io::stdout()
        .flush()
        .map_err(|error| error.to_string())?;
    serve_loop(host, listener)
}

fn serve_loop(host: Host, listener: TcpListener) -> Result<(), String> {
    listener
        .set_nonblocking(true)
        .map_err(|error| error.to_string())?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    runtime.block_on(async move {
        let listener =
            tokio::net::TcpListener::from_std(listener).map_err(|error| error.to_string())?;
        let (changed, _) = watch::channel(0);
        let (closed, mut shutdown) = watch::channel(false);
        let state = Authority {
            host: Arc::new(Mutex::new(host)),
            changed,
            closed,
            requests: Arc::new(Semaphore::new(MAX_REQUESTS)),
            waiters: Arc::new(Semaphore::new(MAX_WAITERS)),
        };
        let app = Router::new().fallback(handle).with_state(state);
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                while !*shutdown.borrow_and_update() {
                    if shutdown.changed().await.is_err() {
                        break;
                    }
                }
            })
            .await
            .map_err(|error| error.to_string())
    })
}

async fn handle(State(state): State<Authority>, request: Request) -> Response {
    let Ok(_permit) = state.requests.try_acquire() else {
        return respond(503, json!({"error":"too many in-flight requests"}));
    };
    match dispatch(&state, request).await {
        Ok(value) => respond(200, value),
        Err(error) => respond(error.status, json!({"error":error.message})),
    }
}

async fn dispatch(state: &Authority, request: Request) -> Result<Value, HostError> {
    if request.method() != Method::POST
        || request.uri().path() != "/"
        || request.uri().query().is_some()
    {
        return Err(HostError::new(404, "POST / required"));
    }
    if request.headers().contains_key(header::ORIGIN) {
        return Err(HostError::new(403, "browser origins are not accepted"));
    }
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or_else(|| HostError::new(403, "bearer capability required"))?
        .to_owned();
    // Authenticate before reading a body. A rejected request drops its async body;
    // it never drains an attacker-provided Content-Length on the authority thread.
    let host = Arc::clone(&state.host);
    let credential = token.clone();
    tokio::task::spawn_blocking(move || {
        let host = host
            .lock()
            .map_err(|_| HostError::new(500, "match authority unavailable"))?;
        host.authorize(&credential).map(|_| ())
    })
    .await
    .map_err(|_| HostError::new(500, "match authority unavailable"))??;
    if request
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|length| length > MAX_REQUEST_BYTES as u64)
    {
        return Err(HostError::new(413, "request too large"));
    }
    let bytes = timeout(
        BODY_TIMEOUT,
        to_bytes(request.into_body(), MAX_REQUEST_BYTES),
    )
    .await
    .map_err(|_| HostError::new(408, "request body timed out"))?
    .map_err(|_| HostError::new(413, "request body unavailable or too large"))?;
    let command: Command =
        serde_json::from_slice(&bytes).map_err(|_| HostError::new(400, "invalid JSON command"))?;
    let wait_ms = number(
        &command.args,
        "waitMs",
        if command.method == "wait" { 25_000 } else { 0 },
        25_000,
    )?;
    // Subscribe before the first read/commit so a move between reading a waiting
    // view and parking cannot be missed.
    let mut changes = state.changed.subscribe();
    let mut view = execute(state.clone(), token.clone(), command).await?;
    if view["status"] != "waiting" || wait_ms == 0 {
        return Ok(view);
    }
    let Ok(_waiter) = state.waiters.try_acquire() else {
        return Ok(view);
    };
    let receipt = view.get("receipt").cloned();
    let deadline =
        Instant::now() + Duration::from_millis(u64::try_from(wait_ms).expect("bounded wait"));
    loop {
        if !matches!(timeout_at(deadline, changes.changed()).await, Ok(Ok(()))) {
            return Ok(view);
        }
        view = execute(
            state.clone(),
            token.clone(),
            Command {
                method: "observe".into(),
                args: json!({}),
            },
        )
        .await?;
        if let Some(receipt) = &receipt {
            view["receipt"] = receipt.clone();
        }
        if view["status"] != "waiting" || Instant::now() >= deadline {
            return Ok(view);
        }
    }
}

async fn execute(state: Authority, token: String, command: Command) -> Result<Value, HostError> {
    tokio::task::spawn_blocking(move || {
        let mut host = state
            .host
            .lock()
            .map_err(|_| HostError::new(500, "match authority unavailable"))?;
        let before = (host.journal.len(), host.failure.is_some(), host.is_closed());
        let result = host.request(&token, &command.method, &command.args);
        let after = (host.journal.len(), host.failure.is_some(), host.is_closed());
        if before != after
            || (result.is_ok() && matches!(command.method.as_str(), "yield" | "cancelYield"))
        {
            state
                .changed
                .send_modify(|generation| *generation = generation.wrapping_add(1));
        }
        if host.is_closed() {
            state.closed.send_replace(true);
        }
        result
    })
    .await
    .map_err(|_| HostError::new(500, "match authority unavailable"))?
}

fn respond(status: u16, value: Value) -> Response {
    (
        StatusCode::from_u16(status).expect("HTTP status"),
        [(header::CACHE_CONTROL, "no-store")],
        Json(value),
    )
        .into_response()
}

#[cfg(test)]
mod tests;
