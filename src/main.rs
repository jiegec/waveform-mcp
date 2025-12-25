use anyhow::{Context, Result};
use async_trait::async_trait;
use mcp_sdk_rs::error::{Error as McpError, ErrorCode};
use mcp_sdk_rs::server::{Server, ServerHandler};
use mcp_sdk_rs::transport::stdio::StdioTransport;
use mcp_sdk_rs::types::{
    Implementation, ServerCapabilities, ToolResult, MessageContent, ClientCapabilities,
};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

// Global state to hold open waveforms
type WaveformStore = Arc<RwLock<HashMap<String, Arc<wellen::simple::Waveform>>>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenWaveformArgs {
    file_path: String,
    #[serde(default)]
    alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ListSignalsArgs {
    waveform_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GetSignalValueArgs {
    waveform_id: String,
    signal_path: String,
    time_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GetSignalInfoArgs {
    waveform_id: String,
    signal_path: String,
}

struct WaveformHandler {
    waveforms: WaveformStore,
}

#[async_trait]
impl ServerHandler for WaveformHandler {
    async fn initialize(
        &self,
        _implementation: Implementation,
        _capabilities: ClientCapabilities,
    ) -> Result<ServerCapabilities, McpError> {
        Ok(ServerCapabilities {
            tools: Some(serde_json::json!({
                "tools": [
                    {
                        "name": "open_waveform",
                        "description": "Open a VCD or FST waveform file",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "file_path": {
                                    "type": "string",
                                    "description": "Path to waveform file (.vcd or .fst)"
                                },
                                "alias": {
                                    "type": "string",
                                    "description": "Optional alias for waveform (defaults to filename)"
                                }
                            },
                            "required": ["file_path"]
                        }
                    },
                    {
                        "name": "list_signals",
                        "description": "List all signals in an open waveform",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "waveform_id": {
                                    "type": "string",
                                    "description": "ID or alias of waveform"
                                }
                            },
                            "required": ["waveform_id"]
                        }
                    },
                    {
                        "name": "read_signal",
                        "description": "Read signal values from a waveform",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "waveform_id": {
                                    "type": "string",
                                    "description": "ID or alias of waveform"
                                },
                                "signal_path": {
                                    "type": "string",
                                    "description": "Path to signal (e.g., 'top.module.signal')"
                                },
                                "time_index": {
                                    "type": "integer",
                                    "description": "Time index to read the value at"
                                }
                            },
                            "required": ["waveform_id", "signal_path", "time_index"]
                        }
                    },
                    {
                        "name": "get_signal_info",
                        "description": "Get metadata about a signal",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "waveform_id": {
                                    "type": "string",
                                    "description": "ID or alias of waveform"
                                },
                                "signal_path": {
                                    "type": "string",
                                    "description": "Path to signal (e.g., 'top.module.signal')"
                                }
                            },
                            "required": ["waveform_id", "signal_path"]
                        }
                    }
                ]
            })),
            ..Default::default()
        })
    }

    async fn shutdown(&self) -> Result<(), McpError> {
        Ok(())
    }

    async fn handle_method(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, McpError> {
        match method {
            "tools/list" => {
                Ok(serde_json::json!({
                    "tools": [
                        {
                            "name": "open_waveform",
                            "description": "Open a VCD or FST waveform file",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "file_path": {
                                        "type": "string",
                                        "description": "Path to waveform file (.vcd or .fst)"
                                    },
                                    "alias": {
                                        "type": "string",
                                        "description": "Optional alias for waveform (defaults to filename)"
                                    }
                                },
                                "required": ["file_path"]
                            }
                        },
                        {
                            "name": "list_signals",
                            "description": "List all signals in an open waveform",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "waveform_id": {
                                        "type": "string",
                                        "description": "ID or alias of waveform"
                                    }
                                },
                                "required": ["waveform_id"]
                            }
                        },
                        {
                            "name": "read_signal",
                            "description": "Read signal values from a waveform",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "waveform_id": {
                                        "type": "string",
                                        "description": "ID or alias of waveform"
                                    },
                                    "signal_path": {
                                        "type": "string",
                                        "description": "Path to signal (e.g., 'top.module.signal')"
                                    },
                                    "time_index": {
                                        "type": "integer",
                                        "description": "Time index to read the value at"
                                    }
                                },
                                "required": ["waveform_id", "signal_path", "time_index"]
                            }
                        },
                        {
                            "name": "get_signal_info",
                            "description": "Get metadata about a signal",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "waveform_id": {
                                        "type": "string",
                                        "description": "ID or alias of waveform"
                                    },
                                    "signal_path": {
                                        "type": "string",
                                        "description": "Path to signal (e.g., 'top.module.signal')"
                                    }
                                },
                                "required": ["waveform_id", "signal_path"]
                            }
                        }
                    ]
                }))
            }
            "tools/call" => {
                let params = params.ok_or_else(|| {
                    McpError::protocol(ErrorCode::InvalidParams, "Missing parameters")
                })?;
                let tool_name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        McpError::protocol(ErrorCode::InvalidParams, "Missing tool name")
                    })?
                    .to_string();
                let arguments = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));
                
                let result = self.handle_tool_call(&tool_name, arguments).await
                    .map_err(|e| McpError::protocol(ErrorCode::InternalError, format!("Tool error: {}", e)))?;
                
                Ok(serde_json::to_value(result)?)
            }
            _ => Err(McpError::protocol(ErrorCode::MethodNotFound, format!("Unknown method: {}", method))),
        }
    }
}

impl WaveformHandler {
    async fn handle_tool_call(
        &self,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<ToolResult> {
        match tool_name {
            "open_waveform" => self.handle_open_waveform(args).await,
            "list_signals" => self.handle_list_signals(args).await,
            "read_signal" => self.handle_read_signal(args).await,
            "get_signal_info" => self.handle_get_signal_info(args).await,
            _ => Err(anyhow::anyhow!("Unknown tool: {}", tool_name)),
        }
    }

    async fn handle_open_waveform(
        &self,
        args: serde_json::Value,
    ) -> Result<ToolResult> {
        let args: OpenWaveformArgs = serde_json::from_value(args)
            .context("Failed to parse open_waveform arguments")?;

        let path = PathBuf::from(&args.file_path);
        if !path.exists() {
            return Ok(ToolResult {
                content: vec![MessageContent::Text {
                    text: format!("Error: File not found: {}", args.file_path),
                }],
                structured_content: None,
            });
        }

        let waveform = wellen::simple::read(&path)
            .context("Failed to read waveform file")?;

        let waveform_id = args.alias.unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("waveform")
                .to_string()
        });

        let mut waveforms = self.waveforms.write().await;
        waveforms.insert(waveform_id.clone(), Arc::new(waveform));

        Ok(ToolResult {
            content: vec![MessageContent::Text {
                text: format!("Opened waveform '{}' from '{}'", waveform_id, args.file_path),
            }],
            structured_content: None,
        })
    }

    async fn handle_list_signals(
        &self,
        args: serde_json::Value,
    ) -> Result<ToolResult> {
        let args: ListSignalsArgs = serde_json::from_value(args)
            .context("Failed to parse list_signals arguments")?;

        let waveforms = self.waveforms.read().await;
        let waveform = waveforms.get(&args.waveform_id)
            .ok_or_else(|| anyhow::anyhow!("Waveform '{}' not found", args.waveform_id))?;

        let hierarchy = waveform.hierarchy();
        let mut signals = Vec::new();
        collect_signals(hierarchy, String::new(), &mut signals);

        let output = if signals.is_empty() {
            "No signals found".to_string()
        } else {
            signals.join("\n")
        };

        Ok(ToolResult {
            content: vec![MessageContent::Text { text: output }],
            structured_content: None,
        })
    }

    async fn handle_read_signal(
        &self,
        args: serde_json::Value,
    ) -> Result<ToolResult> {
        let args: GetSignalValueArgs = serde_json::from_value(args)
            .context("Failed to parse read_signal arguments")?;

        let waveforms = self.waveforms.read().await;
        let waveform = waveforms.get(&args.waveform_id)
            .ok_or_else(|| anyhow::anyhow!("Waveform '{}' not found", args.waveform_id))?;

        let hierarchy = waveform.hierarchy();
        let _signal_ref = find_signal_by_path(hierarchy, &args.signal_path)
            .ok_or_else(|| anyhow::anyhow!("Signal '{}' not found", args.signal_path))?;

        let time_table = waveform.time_table();
        if args.time_index >= time_table.len() {
            return Ok(ToolResult {
                content: vec![MessageContent::Text {
                    text: format!(
                        "Error: Time index {} out of range (max: {})",
                        args.time_index,
                        time_table.len() - 1
                    ),
                }],
                structured_content: None,
            });
        }

        let time = time_table[args.time_index];
        Ok(ToolResult {
            content: vec![MessageContent::Text {
                text: format!("Signal: {}\nTime: {}\nNote: Signal value reading requires signal loading (not yet implemented)", 
                    args.signal_path, time),
            }],
            structured_content: None,
        })
    }

    async fn handle_get_signal_info(
        &self,
        args: serde_json::Value,
    ) -> Result<ToolResult> {
        let args: GetSignalInfoArgs = serde_json::from_value(args)
            .context("Failed to parse get_signal_info arguments")?;

        let waveforms = self.waveforms.read().await;
        let waveform = waveforms.get(&args.waveform_id)
            .ok_or_else(|| anyhow::anyhow!("Waveform '{}' not found", args.waveform_id))?;

        let hierarchy = waveform.hierarchy();
        let signal_ref = find_signal_by_path(hierarchy, &args.signal_path)
            .ok_or_else(|| anyhow::anyhow!("Signal '{}' not found", args.signal_path))?;

        let vars = hierarchy.get_unique_signals_vars();
        let var_ref = vars.get(signal_ref.index())
            .and_then(|v| v.as_ref())
            .ok_or_else(|| anyhow::anyhow!("Signal reference invalid"))?;

        let signal_encoding = hierarchy.get_signal_tpe(signal_ref)
            .ok_or_else(|| anyhow::anyhow!("Failed to get signal type"))?;

        let info = format!(
            "Signal: {}\nType: {:?}\nSignal index: {:?}\nEncoding: {:?}",
            args.signal_path,
            var_ref.var_type(),
            signal_ref,
            signal_encoding
        );

        Ok(ToolResult {
            content: vec![MessageContent::Text { text: info }],
            structured_content: None,
        })
    }
}

fn collect_signals(hierarchy: &wellen::Hierarchy, prefix: String, signals: &mut Vec<String>) {
    for item in hierarchy.items() {
        match item {
            wellen::ScopeOrVarRef::Var(var_ref) => {
                let var = &hierarchy[var_ref];
                let path = if prefix.is_empty() {
                    var.name(hierarchy).to_string()
                } else {
                    format!("{}.{}", prefix, var.name(hierarchy))
                };
                signals.push(format!("{} ({:?})", path, var.var_type()));
            }
            wellen::ScopeOrVarRef::Scope(scope_ref) => {
                let scope = &hierarchy[scope_ref];
                let new_prefix = if prefix.is_empty() {
                    scope.name(hierarchy).to_string()
                } else {
                    format!("{}.{}", prefix, scope.name(hierarchy))
                };
                collect_signals_from_scope(hierarchy, scope, &new_prefix, signals);
            }
        }
    }
}

fn collect_signals_from_scope(
    hierarchy: &wellen::Hierarchy,
    scope: &wellen::Scope,
    prefix: &str,
    signals: &mut Vec<String>,
) {
    for item in scope.items(hierarchy) {
        match item {
            wellen::ScopeOrVarRef::Var(var_ref) => {
                let var = &hierarchy[var_ref];
                let path = format!("{}.{}", prefix, var.name(hierarchy));
                signals.push(format!("{} ({:?})", path, var.var_type()));
            }
            wellen::ScopeOrVarRef::Scope(scope_ref) => {
                let child_scope = &hierarchy[scope_ref];
                let new_prefix = format!("{}.{}", prefix, child_scope.name(hierarchy));
                collect_signals_from_scope(hierarchy, child_scope, &new_prefix, signals);
            }
        }
    }
}

fn find_signal_by_path(hierarchy: &wellen::Hierarchy, path: &str) -> Option<wellen::SignalRef> {
    let parts: Vec<&str> = path.split('.').collect();
    find_signal_recursive(hierarchy, &parts, 0)
}

fn find_signal_recursive(
    hierarchy: &wellen::Hierarchy,
    parts: &[&str],
    current_idx: usize,
) -> Option<wellen::SignalRef> {
    if current_idx >= parts.len() {
        return None;
    }

    let current_name = parts[current_idx];
    let is_last = current_idx == parts.len() - 1;

    for item in hierarchy.items() {
        match item {
            wellen::ScopeOrVarRef::Var(var_ref) if is_last => {
                let var = &hierarchy[var_ref];
                if var.name(hierarchy) == current_name {
                    return Some(var.signal_ref());
                }
            }
            wellen::ScopeOrVarRef::Scope(scope_ref) if !is_last => {
                let scope = &hierarchy[scope_ref];
                if scope.name(hierarchy) == current_name {
                    return find_signal_in_scope(hierarchy, scope, parts, current_idx + 1);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_signal_in_scope(
    hierarchy: &wellen::Hierarchy,
    scope: &wellen::Scope,
    parts: &[&str],
    current_idx: usize,
) -> Option<wellen::SignalRef> {
    if current_idx >= parts.len() {
        return None;
    }

    let current_name = parts[current_idx];
    let is_last = current_idx == parts.len() - 1;

    for item in scope.items(hierarchy) {
        match item {
            wellen::ScopeOrVarRef::Var(var_ref) if is_last => {
                let var = &hierarchy[var_ref];
                if var.name(hierarchy) == current_name {
                    return Some(var.signal_ref());
                }
            }
            wellen::ScopeOrVarRef::Scope(scope_ref) if !is_last => {
                let child_scope = &hierarchy[scope_ref];
                if child_scope.name(hierarchy) == current_name {
                    return find_signal_in_scope(hierarchy, child_scope, parts, current_idx + 1);
                }
            }
            _ => {}
        }
    }
    None
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let waveform_store: WaveformStore = Arc::new(RwLock::new(HashMap::new()));
    let handler = Arc::new(WaveformHandler {
        waveforms: waveform_store,
    });

    // Use stdio transport for MCP communication
    let (read_sender, read_receiver) = tokio::sync::mpsc::channel::<String>(100);
    let (write_sender, mut write_receiver) = tokio::sync::mpsc::channel::<String>(100);

    // Spawn task to read from stdin
    let stdin = tokio::io::stdin();
    let read_sender_clone = read_sender.clone();
    tokio::spawn(async move {
        use tokio::io::{AsyncBufReadExt, BufReader};
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if !line.is_empty() {
                // Validate JSON before sending to transport to prevent panic
                match serde_json::from_str::<serde_json::Value>(&line) {
                    Ok(_) => {
                        let _ = read_sender_clone.send(line).await;
                    }
                    Err(e) => {
                        tracing::error!("Invalid JSON received: {} - error: {}", line, e);
                        // Could send an error response, but transport expects valid messages
                    }
                }
            }
        }
    });

    // Spawn task to write to stdout
    tokio::spawn(async move {
        while let Some(line) = write_receiver.recv().await {
            println!("{}", line);
        }
    });

    let transport = Arc::new(StdioTransport::new(read_receiver, write_sender));
    let server = Server::new(transport, handler);
    server.start().await?;

    Ok(())
}
