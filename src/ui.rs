use askama_axum::{IntoResponse, Response};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{de, Deserialize, Deserializer, Serialize};
use sqlx::{Pool, Sqlite};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracing::error;

use crate::{templates, AppState, ProjectItem};

const START_POINTER: i64 = 9223372036854775807;
const ITERATION_SIZE: u32 = 5;

#[derive(Debug, Deserialize, Serialize)]
pub struct DataContentsParameters {
    pointer: Option<i64>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    project: Option<i64>,
}

fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => std::str::FromStr::from_str(s)
            .map_err(de::Error::custom)
            .map(Some),
    }
}

#[derive(Debug, sqlx::Decode, sqlx::Encode)]
pub struct LogListItem {
    pub id: i64,
    pub project_id: i64,
    pub timestamp: i64,
    pub logentry: String,
    pub event_id: String,
    pub level: String,
}

#[derive(Debug, sqlx::Decode, sqlx::Encode)]
pub struct LogGettItem {
    pub logentry: String,
    pub timestamp: i64,
    pub level: String,
    pub environment: Option<String>,
    pub tags: Option<String>,
    pub breadcrumbs: Option<String>,
    pub exception: Option<String>,
}

pub struct LogListItemView {
    pub id: i64,
    pub project: String,
    pub timestamp: String,
    pub logentry: String,
    pub event_id: String,
    pub level: String,
}

#[derive(Debug, sqlx::Decode, sqlx::Encode)]
pub struct LogGettItemView {
    pub logentry: String,
    pub timestamp: String,
    pub level: String,
    pub environment: String,
    pub tags: String,
    pub breadcrumbs: String,
    pub exception: String,
}

pub struct ProjectItemView {
    pub id: i64,
    pub name: String,
}

impl From<LogListItem> for LogListItemView {
    fn from(value: LogListItem) -> Self {
        LogListItemView {
            id: value.id,
            project: value.project_id.to_string(),
            timestamp: OffsetDateTime::from_unix_timestamp(value.timestamp)
                .unwrap()
                .format(&Rfc3339)
                .unwrap(),
            logentry: value.logentry,
            event_id: value.event_id,
            level: value.level,
        }
    }
}

impl LogListItem {
    pub fn to_view_item(self, projects: &[ProjectItem]) -> LogListItemView {
        let project: String = projects
            .iter()
            .find(|p| p.id == Some(self.project_id))
            .map(|p| p.name.clone())
            .flatten()
            .unwrap_or(self.project_id.to_string());

        LogListItemView {
            id: self.id,
            project,
            timestamp: OffsetDateTime::from_unix_timestamp(self.timestamp)
                .unwrap()
                .format(&Rfc3339)
                .unwrap(),
            logentry: self.logentry,
            event_id: self.event_id,
            level: self.level,
        }
    }
}

impl From<LogGettItem> for LogGettItemView {
    fn from(value: LogGettItem) -> Self {
        LogGettItemView {
            timestamp: OffsetDateTime::from_unix_timestamp(value.timestamp)
                .unwrap()
                .format(&Rfc3339)
                .unwrap(),
            logentry: value.logentry,
            level: value.level,
            environment: value.environment.unwrap_or_default(),
            tags: format_json(&value.tags.unwrap_or_default()).unwrap_or_default(),
            breadcrumbs: format_json(&value.breadcrumbs.unwrap_or_default()).unwrap_or_default(),
            exception: format_json(&value.exception.unwrap_or_default()).unwrap_or_default(),
        }
    }
}

impl From<ProjectItem> for ProjectItemView {
    fn from(value: ProjectItem) -> Self {
        ProjectItemView {
            id: value.id.unwrap_or(0),
            name: value.name.unwrap_or_default(),
        }
    }
}

impl<'a> From<&'a ProjectItem> for ProjectItemView {
    fn from(value: &ProjectItem) -> ProjectItemView {
        ProjectItemView {
            id: value.id.unwrap_or(0),
            name: value.name.clone().unwrap_or_default(),
        }
    }
}

pub async fn get_home() -> impl IntoResponse {
    templates::HomeTemplate {}
}

pub async fn get_data(State(app_state): State<AppState>) -> impl IntoResponse {
    let reading = app_state.projects.read().await;

    let projects: Vec<_> = reading.iter().map(ProjectItemView::from).collect();
    templates::DataTemplate { projects }
}

async fn get_data_query(
    pool: &Pool<Sqlite>,
    parameters: DataContentsParameters,
) -> sqlx::Result<Vec<LogListItem>> {
    let pointer = parameters.pointer.unwrap_or(START_POINTER);
    if let Some(project_id) = parameters.project {
        return sqlx::query_file_as!(
            LogListItem,
            "./sql/list_sentry_log_project_filter.sql",
            pointer,
            project_id,
            ITERATION_SIZE
        )
        .fetch_all(pool)
        .await;
    };

    sqlx::query_file_as!(
        LogListItem,
        "./sql/list_sentry_log.sql",
        pointer,
        ITERATION_SIZE
    )
    .fetch_all(pool)
    .await
}

pub async fn get_data_contents(
    Query(parameters): Query<DataContentsParameters>,
    State(app_state): State<AppState>,
) -> Response {
    match get_data_query(&app_state.pool, parameters).await {
        Ok(entries) => {
            // dbg!(&entries);
            let projects = app_state.projects.read().await;

            templates::DataContentsTemplate {
                entries: entries
                    .into_iter()
                    .map(|x| x.to_view_item(&projects))
                    .collect(),
            }
            .into_response()
        }
        Err(error) => {
            error!("fetching data failed => {error}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
        }
    }
}

pub async fn get_data_content(
    Path(id): Path<String>,
    State(app_state): State<AppState>,
) -> Response {
    match sqlx::query_file_as!(LogGettItem, "./sql/get_sentry_log.sql", id)
        .fetch_one(&app_state.pool)
        .await
    {
        Ok(entry) => templates::DataContentTemplate {
            entry: LogGettItemView::from(entry),
        }
        .into_response(),
        Err(error) => {
            error!("fetching data failed => {error}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
        }
    }
}

pub async fn get_projects(State(app_state): State<AppState>) -> Response {
    let projects = app_state.projects.read().await;
    templates::ProjectsTemplate {
        projects: &projects,
    }
    .into_response()
}

pub async fn edit_project(Path(id): Path<i64>, State(app_state): State<AppState>) -> Response {
    let projects = app_state.projects.read().await;

    match projects.iter().find(|x| x.id == Some(id)) {
        None => {
            error!("project not found");
            (StatusCode::NOT_FOUND, "Project not found").into_response()
        }
        Some(project) => {
            let project = ProjectItemView::from(project);
            templates::ProjectEditTemplate { project }.into_response()
        }
    }
}

fn format_json(json_string: &str) -> Result<String, serde_json::Error> {
    let data: serde_json::Value = serde_json::from_str(json_string)?;
    serde_json::to_string_pretty(&data)
}
