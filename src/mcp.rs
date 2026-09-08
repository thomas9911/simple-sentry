use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, ContentBlock, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use sqlx::SqlitePool;

use crate::ui::{self, DataContentsParameters, LogListItem};
use crate::AppState;

const ITERATION_SIZE: u32 = 20;
const START_POINTER: i64 = i64::MAX;

pub fn build_service(app_state: AppState) -> StreamableHttpService<SentryMcp, LocalSessionManager> {
    StreamableHttpService::new(
        move || Ok(SentryMcp::new(app_state.clone())),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default(),
    )
}

#[derive(Clone)]
pub struct SentryMcp {
    app_state: AppState,
    tool_router: ToolRouter<SentryMcp>,
}

impl SentryMcp {
    pub fn new(app_state: AppState) -> Self {
        Self {
            app_state,
            tool_router: Self::tool_router(),
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListEventsArgs {
    /// Restrict results to this project id. Omit to list events across all projects.
    pub project_id: Option<i64>,
    /// Keyset pagination cursor: id of the last event seen. Omit to start from the newest event.
    pub pointer: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetEventArgs {
    /// The sentry_log row id (not the sentry event_id string).
    pub id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchEventsArgs {
    /// Substring to search for in the event's log message (case-insensitive).
    pub query: String,
    /// Restrict results to this project id. Omit to search across all projects.
    pub project_id: Option<i64>,
    /// Keyset pagination cursor: id of the last event seen. Omit to start from the newest match.
    pub pointer: Option<i64>,
}

#[tool_router]
impl SentryMcp {
    #[tool(
        description = "List known projects (id and name) configured in this simple-sentry instance."
    )]
    async fn list_projects(&self) -> Result<CallToolResult, McpError> {
        let projects = self.app_state.projects.read().await;
        let payload: Vec<_> = projects
            .iter()
            .map(|p| json!({ "id": p.id, "name": p.name }))
            .collect();
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&payload).unwrap_or_default(),
        )]))
    }

    #[tool(
        description = "List recent sentry events, newest first, up to 20 at a time. Optionally filter by project id and page backwards with `pointer` (pass the smallest `id` from the previous page)."
    )]
    async fn list_events(
        &self,
        Parameters(args): Parameters<ListEventsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let params = DataContentsParameters::new(args.pointer, args.project_id);
        let entries = ui::get_data_query(&self.app_state.pool, params)
            .await
            .map_err(|e| McpError::internal_error(format!("query failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(
                &entries
                    .into_iter()
                    .map(log_list_item_json)
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_default(),
        )]))
    }

    #[tool(
        description = "Fetch full details (message, level, environment, tags, breadcrumbs, exception) for a single event by its row id."
    )]
    async fn get_event(
        &self,
        Parameters(args): Parameters<GetEventArgs>,
    ) -> Result<CallToolResult, McpError> {
        let entry = ui::get_event_query(&self.app_state.pool, &args.id.to_string())
            .await
            .map_err(|e| McpError::internal_error(format!("event not found: {e}"), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&json!({
                "logentry": entry.logentry,
                "timestamp": entry.timestamp,
                "level": entry.level,
                "environment": entry.environment,
                "tags": entry.tags,
                "breadcrumbs": entry.breadcrumbs,
                "exception": entry.exception,
            }))
            .unwrap_or_default(),
        )]))
    }

    #[tool(
        description = "Full-text search event log messages (case-insensitive substring match), newest first, up to 20 at a time. Optionally filter by project id and page with `pointer`."
    )]
    async fn search_events(
        &self,
        Parameters(args): Parameters<SearchEventsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let entries = search_events_query(&self.app_state.pool, &args)
            .await
            .map_err(|e| McpError::internal_error(format!("query failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(
                &entries
                    .into_iter()
                    .map(log_list_item_json)
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_default(),
        )]))
    }
}

#[tool_handler(router = self.tool_router.clone())]
impl ServerHandler for SentryMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_protocol_version(ProtocolVersion::V_2024_11_05)
            .with_server_info(Implementation::new(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "Read-only access to a local simple-sentry event log. Tools: list_projects, \
                 list_events (paginated, optional project filter), get_event (by row id), \
                 search_events (substring match on log message).",
            )
    }
}

fn log_list_item_json(item: LogListItem) -> serde_json::Value {
    json!({
        "id": item.id,
        "project_id": item.project_id,
        "timestamp": item.timestamp,
        "logentry": item.logentry,
        "event_id": item.event_id,
        "level": item.level,
    })
}

async fn search_events_query(
    pool: &SqlitePool,
    args: &SearchEventsArgs,
) -> sqlx::Result<Vec<LogListItem>> {
    let pointer = args.pointer.unwrap_or(START_POINTER);
    let pattern = format!("%{}%", args.query);
    if let Some(project_id) = args.project_id {
        return sqlx::query_file_as!(
            LogListItem,
            "./sql/search_sentry_log_project_filter.sql",
            pointer,
            project_id,
            pattern,
            ITERATION_SIZE
        )
        .fetch_all(pool)
        .await;
    }
    sqlx::query_file_as!(
        LogListItem,
        "./sql/search_sentry_log.sql",
        pointer,
        pattern,
        ITERATION_SIZE
    )
    .fetch_all(pool)
    .await
}
