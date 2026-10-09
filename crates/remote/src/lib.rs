//! HTTP remote control.

use axum::Json;
use axum::Router;
use axum::extract::{Path, Query, Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use serde::Serialize;
use soundboard_core::{Command, SoundId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::oneshot;

const PAGE: &str = include_str!("page.html");

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RemoteTab {
    pub id: String,
    pub name: String,
    pub sounds: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RemoteSound {
    pub id: String,
    pub name: String,
    pub color: String,
    pub hotkey: Option<String>,
    pub playing: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct RemoteState {
    pub tabs: Vec<RemoteTab>,
    pub sounds: Vec<RemoteSound>,
}

/// What the server needs from the running app.
pub trait RemoteApi: Send + Sync + 'static {
    fn state(&self) -> RemoteState;
    fn send(&self, cmd: Command);
}

#[derive(Clone)]
struct AppState {
    api: Arc<dyn RemoteApi>,
    token: Arc<str>,
}

/// Compares in time independent of where the inputs differ.
pub fn token_matches(given: &str, expected: &str) -> bool {
    let (a, b) = (given.as_bytes(), expected.as_bytes());
    if a.len() != b.len() || b.is_empty() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

async fn require_token(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
    request: Request,
    next: Next,
) -> Response {
    let bearer = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let given = bearer.or(query.get("token").map(String::as_str));
    match given {
        Some(token) if token_matches(token, &state.token) => next.run(request).await,
        _ => StatusCode::UNAUTHORIZED.into_response(),
    }
}

async fn page() -> Html<&'static str> {
    Html(PAGE)
}

async fn state(State(state): State<AppState>) -> Json<RemoteState> {
    Json(state.api.state())
}

fn known_sound(state: &AppState, raw: &str) -> Result<SoundId, StatusCode> {
    let id: SoundId = raw.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
    let id_text = id.to_string();
    if state.api.state().sounds.iter().any(|s| s.id == id_text) {
        Ok(id)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn play(State(state): State<AppState>, Path(id): Path<String>) -> StatusCode {
    match known_sound(&state, &id) {
        Ok(id) => {
            state.api.send(Command::Play(id));
            StatusCode::NO_CONTENT
        }
        Err(status) => status,
    }
}

async fn stop(State(state): State<AppState>, Path(id): Path<String>) -> StatusCode {
    match known_sound(&state, &id) {
        Ok(id) => {
            state.api.send(Command::Stop(id));
            StatusCode::NO_CONTENT
        }
        Err(status) => status,
    }
}

async fn stop_all(State(state): State<AppState>) -> StatusCode {
    state.api.send(Command::StopAll);
    StatusCode::NO_CONTENT
}

pub fn router(api: Arc<dyn RemoteApi>, token: String) -> Router {
    let state = AppState {
        api,
        token: token.into(),
    };
    let api_routes = Router::new()
        .route("/state", get(self::state))
        .route("/sounds/{id}/play", post(play))
        .route("/sounds/{id}/stop", post(stop))
        .route("/stop", post(stop_all))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_token));
    Router::new()
        .route("/", get(page))
        .nest("/api", api_routes)
        .with_state(state)
}

/// A running server on its own thread; dropping or stopping shuts it down.
pub struct RemoteServer {
    port: u16,
    shutdown: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl RemoteServer {
    pub fn start(port: u16, token: String, api: Arc<dyn RemoteApi>) -> Result<Self, String> {
        let listener = std::net::TcpListener::bind(("0.0.0.0", port))
            .map_err(|e| format!("could not listen on port {port}: {e}"))?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let (shutdown, shutdown_rx) = oneshot::channel::<()>();
        let app = router(api, token);
        let thread = std::thread::Builder::new()
            .name("remote".into())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(e) => return tracing::error!("remote runtime failed: {e}"),
                };
                runtime.block_on(async move {
                    let listener = match tokio::net::TcpListener::from_std(listener) {
                        Ok(listener) => listener,
                        Err(e) => return tracing::error!("remote listener failed: {e}"),
                    };
                    let served = axum::serve(listener, app)
                        .with_graceful_shutdown(async {
                            let _ = shutdown_rx.await;
                        })
                        .await;
                    if let Err(e) = served {
                        tracing::error!("remote server stopped: {e}");
                    }
                });
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            port,
            shutdown: Some(shutdown),
            thread: Some(thread),
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn stop(self) {
        drop(self);
    }
}

impl Drop for RemoteServer {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests;
