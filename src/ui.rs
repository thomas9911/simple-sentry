use askama_axum::{IntoResponse, Response};
use axum::extract::{Query, State};
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

pub struct LogListItemView {
    pub id: i64,
    pub timestamp: String,
    pub logentry: String,
    pub event_id: String,
    pub level: String,
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
