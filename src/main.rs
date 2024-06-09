use askama_axum::Response;
use axum::extract::{Path, State};
use axum::http::{status, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Form, Router};
use axum_extra::extract::JsonLines;
use futures_util::StreamExt;
use serde::Deserialize;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use sqlx::types::Json;
use std::str::FromStr;
use std::sync::Arc;
use tokio::signal::ctrl_c;
use tokio::sync::RwLock;
use tracing::{error, info};

type Object = serde_json::Map<String, serde_json::Value>;

mod templates;
mod time;
mod ui;

#[derive(Debug, Deserialize, sqlx::Decode, sqlx::Encode)]
pub struct Message {
    pub message: serde_json::Value,
    pub formatted: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct LogEntry {
    pub message: String,
    #[serde(flatten)]
    pub unknown: Object,
}

#[derive(Debug, Deserialize, sqlx::Decode, sqlx::Encode)]
pub struct LogEvent {
    pub timestamp: crate::time::Timestamp,
    pub logentry: Option<LogEntry>,
    pub message: Option<Message>,
    pub contexts: Object,
    pub environment: Option<String>,
    pub event_id: String,
    pub platform: String,
    pub sdk: Object,
    pub server_name: Option<String>,
    pub exception: Option<serde_json::Value>,
    pub user: Object,
    #[serde(default = "empty_object")]
    pub extra: Object,
    #[serde(default = "empty_json")]
    pub breadcrumbs: serde_json::Value,
    #[serde(default = "empty_object")]
    pub tags: Object,
    #[serde(default = "default_log_level")]
    pub level: String,
    pub fingerprint: Option<Vec<String>>,
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

#[derive(Debug, Clone)]
struct AppState {
    pub pool: SqlitePool,
    pub projects: Arc<RwLock<Vec<ProjectItem>>>,
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

    let projects = match list_all_projects(&pool).await {
        Ok(projects) => projects,
        Err(err) => {
            error!("Failed to list projects: {}", err);
            return Err(anyhow::anyhow!(err));
        }
    };

    let app_state = AppState {
        pool,
        projects: Arc::new(RwLock::new(projects)),
    };

    let app = Router::new()
        .route("/", get(ui::get_home))
        .route("/ui/data", get(ui::get_data))
        .route("/ui/data/contents", get(ui::get_data_contents))
        .route("/ui/data/contents/:content_id", get(ui::get_data_content))
        .route("/ui/projects", get(ui::get_projects))
        .route(
            "/ui/projects/:project_id/edit",
            get(ui::edit_project).put(update_project),
        )
        .route("/api/:project_id/envelope/", post(handle_post))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(async { ctrl_c().await.unwrap() })
        .await
        .unwrap();

    Ok(())
}

async fn handle_post(
    Path(project_id): Path<i64>,
    State(app_state): State<AppState>,
    mut stream: JsonLines<LogEvent>,
) -> impl IntoResponse {
    while let Some(value) = stream.next().await {
        if let Some(event) = value.ok() {
            if let Err(error) = insert_event_into_db(&app_state.pool, event, project_id).await {
                error!("Failed to insert log entry into database => {error}");
            };
        }
    }

    "{}"
}

async fn insert_event_into_db(
    pool: &SqlitePool,
    event: LogEvent,
    project_id: i64,
) -> anyhow::Result<()> {
    let mut conn = pool.acquire().await?;

    let sdk = Json::from(event.sdk);
    let user = Json::from(event.user);
    let tags = Json::from(event.tags);
    let contexts = Json::from(event.contexts);
    let extra = Json::from(event.extra);
    let breadcrumbs = Json::from(event.breadcrumbs);
    let exception_json = Json::from(&event.exception);
    let timestamp = event.timestamp.to_unix();

    let message = if let Some(logentry) = event.logentry {
        Some(logentry.message)
    } else if let Some(message) = event.message {
        Some(message.formatted.to_string())
    } else if let Some(ref exception) = event.exception {
        if let Some(single_line) = exception.pointer("/0/value").map(|x| x.as_str()).flatten() {
            Some(single_line.to_string())
        } else {
            Some(exception.to_string())
        }
    } else {
        None
    };

    sqlx::query_file!("./sql/insert_project.sql", project_id, None::<String>)
        .execute(&mut *conn)
        .await?;

    sqlx::query_file!(
        "./sql/insert_sentry_log.sql",
        project_id,
        timestamp,
        message,
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
        breadcrumbs,
        exception_json
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}

#[derive(Debug)]
pub struct ProjectItem {
    pub id: Option<i64>,
    pub name: Option<String>,
}

async fn list_all_projects(pool: &SqlitePool) -> Result<Vec<ProjectItem>, sqlx::Error> {
    sqlx::query_file_as!(ProjectItem, "./sql/list_projects.sql")
        .fetch_all(pool)
        .await
}

#[derive(Deserialize)]
pub struct UpdateProjectForm {
    name: String,
}

async fn update_project(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
    Form(update_form): Form<UpdateProjectForm>,
) -> Response {
    match sqlx::query_file!("./sql/update_project.sql", update_form.name, id)
        .execute(&app_state.pool)
        .await
    {
        Ok(_) => (),
        Err(err) => {
            error!("Update project failed: {:?}", err);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response();
        }
    };

    match refresh_project_state(app_state).await {
        Ok(_) => (),
        Err(err) => {
            error!("Update project state failed: {:?}", err);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response();
        }
    };

    (
        StatusCode::FOUND,
        r#"<script>window.location="/ui/projects"</script>"#,
    )
        .into_response()
}

async fn refresh_project_state(app_state: AppState) -> Result<(), anyhow::Error> {
    let projects = list_all_projects(&app_state.pool).await?;
    let mut projects_state = app_state.projects.write().await;

    *projects_state = projects;

    Ok(())
}

fn format_json(json_string: &str) -> Result<String, serde_json::Error> {
    let data: serde_json::Value = serde_json::from_str(json_string)?;
    serde_json::to_string_pretty(&data)
}
