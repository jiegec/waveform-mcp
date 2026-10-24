//! Waveform CLI - Direct command-line interface for waveform tools
//!
//! Usage: waveform-cli <command1> [args...] [-- <command2> [args...] ...]
//!
//! Commands:
//!   open_waveform <file_path> [--alias <alias>]
//!   close_waveform <waveform_id>
//!   list_signals <waveform_id> [--contains <p>] [--glob <p>] [--regex <p>] [--hierarchy <prefix>] [--recursive <true|false>] [--limit <n>]
//!   read_hierarchy <waveform_id> [--scope <scope>] [--recursive <true|false>] [--limit <n>]
//!   read_signal <waveform_id> <signal_path> [--time-index <idx> | --time-indices <idx1,idx2,...>]
//!   get_signal_info <waveform_id> <signal_path>
//!   find_signal_events <waveform_id> <signal_path> [--start <idx>] [--end <idx>] [--limit <n>]
//!   find_conditional_events <waveform_id> <condition> [--start <idx>] [--end <idx>] [--limit <n>]

use std::collections::HashMap;
use std::path::PathBuf;
use waveform_mcp::{
    Command, SignalMatcher, command_help, file_not_found_message, find_conditional_events,
    find_scope_by_path, find_signal_by_path, find_signal_events, format_signal_query_result,
    get_signal_metadata, help_request, list_signals_detailed, not_found_message, parse_args,
    read_hierarchy, read_signal_values, scope_not_found_message, signal_not_found_message,
    unknown_command_message, usage_text,
};

/// Print the help text for `command`, or the general usage text when `None`.
fn print_help(command: Option<&str>) {
    match command {
        None => print!("{}", usage_text()),
        Some("help") => print!("{}", usage_text()),
        Some(name) => match command_help(name) {
            Some(text) => print!("{}", text),
            None => {
                eprintln!("{}\n", unknown_command_message(name));
                eprint!("{}", usage_text());
                std::process::exit(1);
            }
        },
    }
}

struct WaveformStore {
    waveforms: HashMap<String, wellen::simple::Waveform>,
}

impl WaveformStore {
    fn new() -> Self {
        Self {
            waveforms: HashMap::new(),
        }
    }

    /// Open a waveform, returning its id and whether an existing one was replaced.
    fn open_waveform(
        &mut self,
        file_path: &str,
        alias: Option<String>,
        overwrite: bool,
    ) -> Result<(String, bool), String> {
        let path = PathBuf::from(file_path);

        if !path.exists() {
            return Err(file_not_found_message(&path));
        }

        let id = alias.unwrap_or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string()
        });

        if self.waveforms.contains_key(&id) && !overwrite {
            return Err(format!(
                "waveform '{}' is already open; close it first, pass a different --alias, or use --overwrite",
                id
            ));
        }

        let waveform =
            wellen::simple::read(&path).map_err(|e| format!("Failed to read waveform: {}", e))?;

        let replaced = self.waveforms.insert(id.clone(), waveform).is_some();
        Ok((id, replaced))
    }

    fn close_waveform(&mut self, waveform_id: &str) -> Result<(), String> {
        match self.waveforms.remove(waveform_id) {
            Some(_) => Ok(()),
            None => Err(self.waveform_not_found(waveform_id)),
        }
    }

    fn get(&self, waveform_id: &str) -> Result<&wellen::simple::Waveform, String> {
        self.waveforms
            .get(waveform_id)
            .ok_or_else(|| self.waveform_not_found(waveform_id))
    }

    fn get_mut(&mut self, waveform_id: &str) -> Result<&mut wellen::simple::Waveform, String> {
        if !self.waveforms.contains_key(waveform_id) {
            return Err(self.waveform_not_found(waveform_id));
        }
        Ok(self
            .waveforms
            .get_mut(waveform_id)
            .expect("waveform existence was checked above"))
    }

    /// "Not found" message listing the open waveforms, or suggesting the closest id.
    fn waveform_not_found(&self, waveform_id: &str) -> String {
        let available: Vec<String> = self.waveforms.keys().cloned().collect();
        not_found_message("waveform", waveform_id, &available)
    }
}

fn execute_command(store: &mut WaveformStore, cmd: &Command) -> Result<String, String> {
    match cmd {
        Command::OpenWaveform {
            file_path,
            alias,
            overwrite,
        } => {
            let (id, replaced) = store.open_waveform(file_path, alias.clone(), *overwrite)?;
            if replaced {
                Ok(format!("Waveform reopened successfully with id: {}", id))
            } else {
                Ok(format!("Waveform opened successfully with id: {}", id))
            }
        }
        Command::CloseWaveform { waveform_id } => {
            store.close_waveform(waveform_id)?;
            Ok(format!("Waveform '{}' closed successfully", waveform_id))
        }
        Command::ListSignals {
            waveform_id,
            patterns,
            hierarchy_prefix,
            recursive,
            limit,
        } => {
            let waveform = store.get(waveform_id)?;

            let hierarchy = waveform.hierarchy();
            let specs: Vec<(String, waveform_mcp::MatchMode)> = patterns
                .iter()
                .map(|p| (p.pattern.clone(), p.mode))
                .collect();
            let matcher = SignalMatcher::new(&specs)
                .map_err(|e| format!("Error compiling path pattern: {}", e))?;

            if let Some(prefix) = hierarchy_prefix.as_deref()
                && find_scope_by_path(hierarchy, prefix).is_none()
            {
                return Err(scope_not_found_message(hierarchy, prefix));
            }

            let listing = list_signals_detailed(
                hierarchy,
                &matcher,
                hierarchy_prefix.as_deref(),
                *recursive,
                *limit,
            );

            Ok(format_signal_query_result(hierarchy, &specs, &listing))
        }
        Command::ReadHierarchy {
            waveform_id,
            scope_path,
            recursive,
            limit,
        } => {
            let waveform = store.get(waveform_id)?;

            let hierarchy = waveform.hierarchy();
            let lines = read_hierarchy(hierarchy, scope_path.as_deref(), *recursive, *limit)
                .map_err(|e| format!("Error reading hierarchy: {}", e))?;

            let header = match scope_path.as_deref() {
                Some(path) => format!("Hierarchy rooted at '{}':", path),
                None => "Hierarchy:".to_string(),
            };
            let body = if lines.is_empty() {
                "No modules found".to_string()
            } else {
                lines.join("\n")
            };

            Ok(format!("{}\n{}", header, body))
        }
        Command::ReadSignal {
            waveform_id,
            signal_path,
            time_index,
            time_indices,
        } => {
            let waveform = store.get_mut(waveform_id)?;

            let hierarchy = waveform.hierarchy();
            let signal_ref = find_signal_by_path(hierarchy, signal_path)
                .ok_or_else(|| signal_not_found_message(hierarchy, signal_path))?;

            waveform.load_signals(&[signal_ref]);

            let indices_to_read: Vec<usize> = if let Some(indices) = time_indices {
                indices.clone()
            } else if let Some(index) = time_index {
                vec![*index]
            } else {
                return Err("Either time_index or time_indices must be provided".to_string());
            };

            let results = read_signal_values(waveform, signal_ref, &indices_to_read)
                .map_err(|e| format!("Error reading signal: {}", e))?;

            Ok(results.join("\n"))
        }
        Command::GetSignalInfo {
            waveform_id,
            signal_path,
        } => {
            let waveform = store.get(waveform_id)?;

            let hierarchy = waveform.hierarchy();
            let info = get_signal_metadata(hierarchy, signal_path)
                .map_err(|e| format!("Error getting signal info: {}", e))?;

            Ok(info)
        }
        Command::FindSignalEvents {
            waveform_id,
            signal_path,
            start_time_index,
            end_time_index,
            limit,
        } => {
            let waveform = store.get_mut(waveform_id)?;

            let hierarchy = waveform.hierarchy();
            let signal_ref = find_signal_by_path(hierarchy, signal_path)
                .ok_or_else(|| signal_not_found_message(hierarchy, signal_path))?;

            waveform.load_signals(&[signal_ref]);

            let time_table = waveform.time_table();
            let start_idx = start_time_index.unwrap_or(0);
            let end_idx = end_time_index.unwrap_or(time_table.len().saturating_sub(1));
            let lim = limit.unwrap_or(-1);

            let events = find_signal_events(waveform, signal_ref, start_idx, end_idx, lim)
                .map_err(|e| format!("Error finding signal events: {}", e))?;

            Ok(format!(
                "Found {} events for signal '{}' (time range: {} to {}):\n{}",
                events.len(),
                signal_path,
                start_idx,
                end_idx,
                events.join("\n")
            ))
        }
        Command::FindConditionalEvents {
            waveform_id,
            condition,
            start_time_index,
            end_time_index,
            limit,
        } => {
            let waveform = store.get_mut(waveform_id)?;

            let time_table = waveform.time_table();
            let start_idx = start_time_index.unwrap_or(0);
            let end_idx = end_time_index.unwrap_or(time_table.len().saturating_sub(1));
            let lim = limit.unwrap_or(-1);

            let events = find_conditional_events(waveform, condition, start_idx, end_idx, lim)
                .map_err(|e| format!("Error finding conditional events: {}", e))?;

            Ok(format!(
                "Found {} events for condition '{}' (time range: {} to {}):\n{}",
                events.len(),
                condition,
                start_idx,
                end_idx,
                events.join("\n")
            ))
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        eprint!("{}", usage_text());
        std::process::exit(1);
    }

    if args[0] == "--version" || args[0] == "-V" {
        println!("waveform-cli {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if let Some(command) = help_request(&args) {
        print_help(command.as_deref());
        return;
    }

    let commands = match parse_args(args) {
        Ok(cmds) => cmds,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let mut store = WaveformStore::new();

    for (i, cmd) in commands.iter().enumerate() {
        if i > 0 {
            println!();
        }

        match execute_command(&mut store, cmd) {
            Ok(output) => println!("{}", output),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
