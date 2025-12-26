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
use waveform_mcp::format_time;

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
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ReadSignalArgs {
    pub waveform_id: String,
    pub signal_path: String,
    pub time_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GetSignalInfoArgs {
    pub waveform_id: String,
    pub signal_path: String,
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

    #[tool(description = "List all signals in an open waveform")]
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
        let mut signals = Vec::new();

        for var in hierarchy.iter_vars() {
            let path = var.full_name(hierarchy);
            signals.push(path);
        }

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Found {} signals:\n{}",
            signals.len(),
            signals.join("\n")
        ))]))
    }

    #[tool(description = "Read signal values from a waveform")]
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

        // Now we have mutable access to waveform
        // Load the signal data
        waveform.load_signals(&[signal_ref]);

        let time_table = waveform.time_table();
        let timescale = waveform.hierarchy().timescale();
        if args.time_index >= time_table.len() {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Time index {} out of range (max: {})",
                args.time_index,
                time_table.len() - 1
            ))]));
        }

        let time_idx = args.time_index;
        let time_value = time_table[time_idx];
        let formatted_time = format_time(time_value, timescale.as_ref());

        let signal = waveform.get_signal(signal_ref).ok_or_else(|| {
            McpError::internal_error("Signal not found after loading".to_string(), None)
        })?;

        let offset = signal
            .get_offset(time_idx.try_into().unwrap())
            .ok_or_else(|| {
                McpError::internal_error("No data available for this time index".to_string(), None)
            })?;

        let signal_value = signal.get_value_at(&offset, 0);

        let value_str = match signal_value {
            wellen::SignalValue::Event => "Event".to_string(),
            wellen::SignalValue::Binary(data, _) => format!("{:?}", data),
            wellen::SignalValue::FourValue(data, _) => format!("{:?}", data),
            wellen::SignalValue::NineValue(data, _) => format!("{:?}", data),
            wellen::SignalValue::String(s) => s.to_string(),
            wellen::SignalValue::Real(r) => format!("{}", r),
        };

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Signal '{}' at time index {} ({}): {}",
            args.signal_path, args.time_index, formatted_time, value_str
        ))]))
    }

    #[tool(description = "Get metadata about a signal")]
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

        // Find the VarRef from the path
        let parts: Vec<&str> = args.signal_path.split('.').collect();
        let var_ref = if parts.len() > 1 {
            let path = &parts[..parts.len() - 1];
            let name = parts[parts.len() - 1];
            hierarchy.lookup_var(path, name).ok_or_else(|| {
                McpError::invalid_params(format!("Signal not found: {}", args.signal_path), None)
            })?
        } else {
            hierarchy
                .lookup_var(&[], args.signal_path.as_str())
                .ok_or_else(|| {
                    McpError::invalid_params(
                        format!("Signal not found: {}", args.signal_path),
                        None,
                    )
                })?
        };

        let var = &hierarchy[var_ref];

        let info = format!(
            "Signal: {}\nType: {:?}\nSignal index: {}",
            args.signal_path,
            var.var_type(),
            var_ref.index()
        );

        Ok(CallToolResult::success(vec![Content::text(info)]))
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
                Available tools: open_waveform, list_signals, read_signal, get_signal_info."
                    .to_string(),
            ),
        }
    }
}

fn find_signal_by_path(hierarchy: &wellen::Hierarchy, path: &str) -> Option<wellen::SignalRef> {
    for var in hierarchy.iter_vars() {
        let signal_path = var.full_name(hierarchy);
        if signal_path == path {
            return Some(var.signal_ref());
        }
    }
    None
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
