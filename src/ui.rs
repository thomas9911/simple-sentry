use askama_axum::{IntoResponse, Response};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracing::error;

use crate::{templates, AppState};

const START_POINTER: i64 = 9223372036854775807;

#[derive(Debug, Deserialize)]
pub struct DataContentsParameters {
    pointer: Option<i64>,
}

#[derive(Debug, sqlx::Decode, sqlx::Encode)]
pub struct LogListItem {
    pub id: i64,
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

impl From<LogListItem> for LogListItemView {
    fn from(value: LogListItem) -> Self {
        LogListItemView {
            id: value.id,
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

pub async fn get_home() -> impl IntoResponse {
    templates::HomeTemplate {}
}

pub async fn get_data() -> impl IntoResponse {
    templates::DataTemplate {}
}

pub async fn get_data_contents(
    Query(parameters): Query<DataContentsParameters>,
    State(app_state): State<AppState>,
) -> Response {
    let pointer = parameters.pointer.unwrap_or(START_POINTER);
    match sqlx::query_file_as!(LogListItem, "./sql/list_sentry_log.sql", pointer, 20)
        .fetch_all(&app_state.pool)
        .await
    {
        Ok(entries) => templates::DataContentsTemplate {
            entries: entries.into_iter().map(LogListItemView::from).collect(),
        }
        .into_response(),
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

fn format_json(json_string: &str) -> Result<String, serde_json::Error> {
    let data: serde_json::Value = serde_json::from_str(json_string)?;
    serde_json::to_string_pretty(&data)
}
