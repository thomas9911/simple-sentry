use axum::response::IntoResponse;
use axum::routing::post;
use axum::Router;
use axum_extra::extract::JsonLines;
use futures_util::StreamExt;
use serde::Deserialize;

type Object = serde_json::Map<String, serde_json::Value>;

#[derive(Debug, Deserialize)]
pub struct LogEvent {
    pub timestamp: f64,
    pub logentry: LogEntry,
    pub contexts: Object,
    pub environment: String,
    pub event_id: String,
    pub platform: String,
    pub sdk: Object,
    pub server_name: String,
    pub user: Object,
    #[serde(default = "empty_object")]
    pub extra: Object,
    #[serde(default = "empty_object")]
    pub breadcrumbs: Object,
    #[serde(default = "empty_object")]
    pub tags: Object,
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(flatten)]
    pub unknown: Object,
}

fn default_log_level() -> String {
    "error".to_string()
}

fn empty_object() -> Object {
    serde_json::Map::new()
}

#[derive(Debug, Deserialize)]
pub struct LogEntry {
    pub message: String,
    #[serde(flatten)]
    pub unknown: Object,
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/api/*key", post(handle_post));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn handle_post(mut stream: JsonLines<LogEvent>) -> impl IntoResponse {
    while let Some(value) = stream.next().await {
        if let Some(event) = value.ok() {
            dbg!(event);
        }
    }
    ""
}
