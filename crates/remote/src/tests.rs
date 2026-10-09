use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use soundboard_core::{Command, SoundId, TabId};
use std::sync::Mutex;
use tower::ServiceExt;

const TOKEN: &str = "secret-token";

struct FakeApi {
    sound: SoundId,
    tab: TabId,
    sent: Mutex<Vec<String>>,
}

impl FakeApi {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            sound: SoundId::new(),
            tab: TabId::new(),
            sent: Mutex::new(Vec::new()),
        })
    }

    fn sent(&self) -> Vec<String> {
        self.sent.lock().unwrap().clone()
    }
}

impl RemoteApi for FakeApi {
    fn state(&self) -> RemoteState {
        RemoteState {
            tabs: vec![RemoteTab {
                id: self.tab.to_string(),
                name: "Sounds".into(),
                sounds: vec![self.sound.to_string()],
            }],
            sounds: vec![RemoteSound {
                id: self.sound.to_string(),
                name: "Airhorn".into(),
                color: "#465a8c".into(),
                hotkey: None,
                playing: false,
            }],
        }
    }

    fn send(&self, cmd: Command) {
        self.sent.lock().unwrap().push(format!("{cmd:?}"));
    }
}

fn app(api: &Arc<FakeApi>) -> axum::Router {
    router(api.clone(), TOKEN.into())
}

fn get(uri: &str) -> Request<Body> {
    Request::get(uri).body(Body::empty()).unwrap()
}

fn post(uri: &str) -> Request<Body> {
    Request::post(uri)
        .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
        .body(Body::empty())
        .unwrap()
}

async fn body_text(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn api_without_token_is_unauthorized() {
    let api = FakeApi::new();
    let response = app(&api).oneshot(get("/api/state")).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn wrong_token_is_unauthorized() {
    let api = FakeApi::new();
    let response = app(&api)
        .oneshot(get("/api/state?token=nope"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn query_token_is_accepted() {
    let api = FakeApi::new();
    let response = app(&api)
        .oneshot(get(&format!("/api/state?token={TOKEN}")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn state_lists_tabs_and_sounds() {
    let api = FakeApi::new();
    let request = Request::get("/api/state")
        .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
        .body(Body::empty())
        .unwrap();
    let response = app(&api).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body_text(response).await).unwrap();
    assert_eq!(json["tabs"][0]["name"], "Sounds");
    assert_eq!(json["sounds"][0]["name"], "Airhorn");
    assert_eq!(json["sounds"][0]["id"], api.sound.to_string());
    assert_eq!(json["sounds"][0]["playing"], false);
}

#[tokio::test]
async fn play_sends_a_play_command() {
    let api = FakeApi::new();
    let uri = format!("/api/sounds/{}/play", api.sound);
    let response = app(&api).oneshot(post(&uri)).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(api.sent(), [format!("Play({})", api.sound)]);
}

#[tokio::test]
async fn stop_sends_a_stop_command() {
    let api = FakeApi::new();
    let uri = format!("/api/sounds/{}/stop", api.sound);
    let response = app(&api).oneshot(post(&uri)).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(api.sent(), [format!("Stop({})", api.sound)]);
}

#[tokio::test]
async fn unknown_sound_is_not_found() {
    let api = FakeApi::new();
    let uri = format!("/api/sounds/{}/play", SoundId::new());
    let response = app(&api).oneshot(post(&uri)).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(api.sent().is_empty());
}

#[tokio::test]
async fn malformed_sound_id_is_bad_request() {
    let api = FakeApi::new();
    let response = app(&api)
        .oneshot(post("/api/sounds/not-a-uuid/play"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn stop_all_sends_stop_all() {
    let api = FakeApi::new();
    let response = app(&api).oneshot(post("/api/stop")).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(api.sent(), ["StopAll"]);
}

#[tokio::test]
async fn page_is_served_without_token() {
    let api = FakeApi::new();
    let response = app(&api).oneshot(get("/")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let text = body_text(response).await;
    assert!(text.contains("<html"));
}

#[test]
fn token_comparison_is_exact() {
    assert!(token_matches("abc", "abc"));
    assert!(!token_matches("abc", "abd"));
    assert!(!token_matches("abc", "abcd"));
    assert!(!token_matches("", "abc"));
}

#[test]
fn server_binds_and_stops() {
    let api: Arc<dyn RemoteApi> = FakeApi::new();
    let server = RemoteServer::start(0, TOKEN.into(), api).unwrap();
    assert_ne!(server.port(), 0);
    server.stop();
}
