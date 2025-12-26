use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
    ErrorData as McpError, ServerHandler, ServiceExt,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use waveform_mcp::{
    find_conditional_events, find_signal_by_path, find_signal_events, get_signal_metadata,
    list_signals, read_signal_values,
};

// Waveform store - using RwLock for interior mutability
type WaveformStore = Arc<RwLock<HashMap<String, wellen::simple::Waveform>>>;

#[derive(Debug, Clone)]
pub struct WaveformHandler {
    waveforms: WaveformStore,
    tool_router: ToolRouter<WaveformHandler>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct OpenWaveformArgs {
    pub file_path: String,
    #[serde(default)]
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ListSignalsArgs {
    pub waveform_id: String,
    #[serde(default)]
    pub name_pattern: Option<String>,
    #[serde(default)]
    pub hierarchy_prefix: Option<String>,
    #[serde(default = "default_recursive")]
    pub recursive: Option<bool>,
    #[serde(default = "default_list_signals_limit")]
    pub limit: Option<isize>,
}

fn default_recursive() -> Option<bool> {
    Some(false)
}

fn default_list_signals_limit() -> Option<isize> {
    Some(100)
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ReadSignalArgs {
    pub waveform_id: String,
    pub signal_path: String,
    #[serde(default = "default_time_index")]
    pub time_index: Option<usize>,
    #[serde(default)]
    pub time_indices: Option<Vec<usize>>,
}

fn default_time_index() -> Option<usize> {
    None
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GetSignalInfoArgs {
    pub waveform_id: String,
    pub signal_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FindSignalEventsArgs {
    pub waveform_id: String,
    pub signal_path: String,
    #[serde(default = "default_start_time")]
    pub start_time_index: Option<usize>,
    #[serde(default = "default_end_time")]
    pub end_time_index: Option<usize>,
    #[serde(default = "default_find_signal_events_limit")]
    pub limit: Option<isize>,
}

fn default_start_time() -> Option<usize> {
    None
}

fn default_end_time() -> Option<usize> {
    None
}

fn default_find_signal_events_limit() -> Option<isize> {
    Some(100)
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FindConditionalEventsArgs {
    pub waveform_id: String,
    pub condition: String,
    #[serde(default = "default_start_time")]
    pub start_time_index: Option<usize>,
    #[serde(default = "default_end_time")]
    pub end_time_index: Option<usize>,
    #[serde(default = "default_find_conditional_events_limit")]
    pub limit: Option<isize>,
}

fn default_find_conditional_events_limit() -> Option<isize> {
    Some(100)
}

impl Default for WaveformHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl WaveformHandler {
    pub fn new() -> Self {
        Self {
            waveforms: Arc::new(RwLock::new(HashMap::new())),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Open a VCD or FST waveform file")]
    async fn open_waveform(
        &self,
        args: Parameters<OpenWaveformArgs>,
    ) -> Result<CallToolResult, McpError> {
        let args = &args.0;
        let path = PathBuf::from(&args.file_path);

        if !path.exists() {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "File not found: {}",
                args.file_path
            ))]));
        }

        let waveform = match wellen::simple::read(&path) {
            Ok(w) => w,
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to read waveform: {}",
                    e
                ))]));
            }
        };

        let alias = args.alias.clone().unwrap_or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string()
        });

        let mut waveforms = self.waveforms.write().await;
        waveforms.insert(alias.clone(), waveform);

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Waveform opened successfully with alias: {}",
            alias
        ))]))
    }

    #[tool(
        description = "List all signals in an open waveform. Use waveform_id from open_waveform. Optional: filter by name_pattern (case-insensitive substring), hierarchy_prefix (e.g., 'top.module'), recursive (default: true), and limit."
    )]
    async fn list_signals(
        &self,
        args: Parameters<ListSignalsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let args = &args.0;
        let waveforms = self.waveforms.read().await;

        let waveform = waveforms.get(&args.waveform_id).ok_or_else(|| {
            McpError::invalid_params(format!("Waveform not found: {}", args.waveform_id), None)
        })?;

        let hierarchy = waveform.hierarchy();
        let recursive = args.recursive.unwrap_or(true);

        let signals = list_signals(
            hierarchy,
            args.name_pattern.as_deref(),
            args.hierarchy_prefix.as_deref(),
            recursive,
            args.limit,
        );

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Found {} signals:\n{}",
            signals.len(),
            signals.join("\n")
        ))]))
    }

    #[tool(
        description = "Read signal values from a waveform. Use waveform_id from open_waveform and signal_path from list_signals. Provide either time_index (single) or time_indices (array)."
    )]
    async fn read_signal(
        &self,
        args: Parameters<ReadSignalArgs>,
    ) -> Result<CallToolResult, McpError> {
        let args = &args.0;
        let mut waveforms = self.waveforms.write().await;

        let waveform = waveforms.get_mut(&args.waveform_id).ok_or_else(|| {
            McpError::invalid_params(format!("Waveform not found: {}", args.waveform_id), None)
        })?;

        let hierarchy = waveform.hierarchy();
        let signal_ref = find_signal_by_path(hierarchy, &args.signal_path).ok_or_else(|| {
            McpError::invalid_params(format!("Signal not found: {}", args.signal_path), None)
        })?;

        // Load the signal data
        waveform.load_signals(&[signal_ref]);

        // Determine which time indices to read
        let indices_to_read: Vec<usize> = if let Some(ref indices) = args.time_indices {
            indices.clone()
        } else if let Some(index) = args.time_index {
            vec![index]
        } else {
            return Ok(CallToolResult::error(vec![Content::text(
                "Either time_index or time_indices must be provided".to_string(),
            )]));
        };

        let results = read_signal_values(waveform, signal_ref, &indices_to_read)
            .map_err(|e| McpError::internal_error(e, None))?;

        Ok(CallToolResult::success(vec![Content::text(
            results.join("\n"),
        )]))
    }

    #[tool(
        description = "Get metadata about a signal. Use waveform_id from open_waveform and signal_path from list_signals."
    )]
    async fn get_signal_info(
        &self,
        args: Parameters<GetSignalInfoArgs>,
    ) -> Result<CallToolResult, McpError> {
        let args = &args.0;
        let waveforms = self.waveforms.read().await;

        let waveform = waveforms.get(&args.waveform_id).ok_or_else(|| {
            McpError::invalid_params(format!("Waveform not found: {}", args.waveform_id), None)
        })?;

        let hierarchy = waveform.hierarchy();

        let info = get_signal_metadata(hierarchy, &args.signal_path)
            .map_err(|e| McpError::invalid_params(e, None))?;

        Ok(CallToolResult::success(vec![Content::text(info)]))
    }

    #[tool(
        description = "Find events (changes) of a signal within a time range. Use waveform_id from open_waveform and signal_path from list_signals. Optional: start_time_index, end_time_index, limit."
    )]
    async fn find_signal_events(
        &self,
        args: Parameters<FindSignalEventsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let args = &args.0;
        let mut waveforms = self.waveforms.write().await;

        let waveform = waveforms.get_mut(&args.waveform_id).ok_or_else(|| {
            McpError::invalid_params(format!("Waveform not found: {}", args.waveform_id), None)
        })?;

        let hierarchy = waveform.hierarchy();
        let signal_ref = find_signal_by_path(hierarchy, &args.signal_path).ok_or_else(|| {
            McpError::invalid_params(format!("Signal not found: {}", args.signal_path), None)
        })?;

        // Load the signal data
        waveform.load_signals(&[signal_ref]);

        let time_table = waveform.time_table();
        let start_idx = args.start_time_index.unwrap_or(0);
        let end_idx = args
            .end_time_index
            .unwrap_or(time_table.len().saturating_sub(1));
        let limit = args.limit.unwrap_or(-1);

        let events = find_signal_events(waveform, signal_ref, start_idx, end_idx, limit)
            .map_err(|e| McpError::internal_error(e, None))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Found {} events for signal '{}' (time range: {} to {}):\n{}",
            events.len(),
            args.signal_path,
            start_idx,
            end_idx,
            events.join("\n")
        ))]))
    }

    #[tool(
        description = "Find events where a condition is satisfied. The condition uses signal paths with && (AND), || (OR), ! (NOT), == (equality), != (inequality), and parentheses. Supports Verilog-style literals: 4'b0101, 3'd2, 5'h1A. Examples: 'TOP.signal1 && TOP.signal2', 'TOP.counter == 4'b0101', 'TOP.state != 3'd2'. Optional: start_time_index, end_time_index, limit."
    )]
    async fn find_conditional_events(
        &self,
        args: Parameters<FindConditionalEventsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let args = &args.0;
        let mut waveforms = self.waveforms.write().await;

        let waveform = waveforms.get_mut(&args.waveform_id).ok_or_else(|| {
            McpError::invalid_params(format!("Waveform not found: {}", args.waveform_id), None)
        })?;

        let time_table = waveform.time_table();
        let start_idx = args.start_time_index.unwrap_or(0);
        let end_idx = args
            .end_time_index
            .unwrap_or(time_table.len().saturating_sub(1));
        let limit = args.limit.unwrap_or(-1);

        let events = find_conditional_events(waveform, &args.condition, start_idx, end_idx, limit)
            .map_err(|e| McpError::invalid_params(e, None))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Found {} events for condition '{}' (time range: {} to {}):\n{}",
            events.len(),
            args.condition,
            start_idx,
            end_idx,
            events.join("\n")
        ))]))
    }
}

#[tool_handler]
impl ServerHandler for WaveformHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "MCP server for reading VCD/FST waveform files using the wellen library. \
                Available tools: open_waveform, list_signals, read_signal, get_signal_info, find_signal_events, find_conditional_events."
                    .to_string(),
            ),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let handler = WaveformHandler::new();

    let service = handler.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("Serving error: {:?}", e);
    })?;

    service.waiting().await?;

    Ok(())
}
