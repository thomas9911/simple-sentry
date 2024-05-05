use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use axum_extra::extract::JsonLines;
use futures_util::StreamExt;
use serde::Deserialize;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use sqlx::types::Json;
use std::str::FromStr;
use tracing::{error, info};

type Object = serde_json::Map<String, serde_json::Value>;

mod templates;
mod time;
mod ui;

#[derive(Debug, Deserialize, sqlx::Decode, sqlx::Encode)]
pub struct LogEvent {
    pub timestamp: crate::time::Timestamp,
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
    #[serde(default = "empty_json")]
    pub breadcrumbs: serde_json::Value,
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

fn empty_json() -> serde_json::Value {
    serde_json::Value::Object(Object::new())
}

#[derive(Debug, Deserialize)]
pub struct LogEntry {
    pub message: String,
    #[serde(flatten)]
    pub unknown: Object,
}

#[derive(Debug, Clone)]
struct AppState {
    pub pool: SqlitePool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let db_path = std::env::var("SIMPLE_SENTRY_DB").unwrap_or("sqlite://data/data.db".to_string());
    info!("Using database at: {db_path}");

    let connect_opts = SqliteConnectOptions::from_str(&db_path)?
        .journal_mode(SqliteJournalMode::Wal)
        .create_if_missing(true);

    let pool = SqlitePoolOptions::default()
        .connect_with(connect_opts)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    info!("Database created");

    let app_state = AppState { pool };

    let app = Router::new()
        .route("/", get(ui::get_home))
        .route("/ui/data", get(ui::get_data))
        .route("/ui/data/contents", get(ui::get_data_contents))
        .route("/api/*key", post(handle_post))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

// async fn handle_post(
//     State(app_state): State<AppState>,
//     mut stream: JsonLines<serde_json::Value>,
// ) -> impl IntoResponse {
//     while let Some(value) = stream.next().await {
//         println!("{}", &value.unwrap());
//     }

//     "{}"
// }

async fn handle_post(
    State(app_state): State<AppState>,
    mut stream: JsonLines<LogEvent>,
) -> impl IntoResponse {
    while let Some(value) = stream.next().await {
        dbg!(&value);
        if let Some(event) = value.ok() {
            if let Err(error) = insert_event_into_db(&app_state.pool, event).await {
                error!("Failed to insert log entry into database => {error}");
            };
        }
    }
    "{}"
}

async fn insert_event_into_db(pool: &SqlitePool, event: LogEvent) -> anyhow::Result<()> {
    let mut conn = pool.acquire().await?;

    let sdk = Json::from(event.sdk);
    let user = Json::from(event.user);
    let tags = Json::from(event.tags);
    let contexts = Json::from(event.contexts);
    let extra = Json::from(event.extra);
    let breadcrumbs = Json::from(event.breadcrumbs);
    let timestamp = event.timestamp.to_unix();

    sqlx::query_file!(
        "./sql/insert_sentry_log.sql",
        timestamp,
        event.logentry.message,
        event.level,
        event.environment,
        event.event_id,
        event.platform,
        event.server_name,
        sdk,
        user,
        tags,
        contexts,
        extra,
        breadcrumbs
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}
