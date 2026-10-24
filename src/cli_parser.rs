//! Command-line parsing for waveform-cli

use crate::matcher::MatchMode;
use crate::suggest::did_you_mean;

/// Names of every CLI command, in help order.
pub const COMMAND_NAMES: [&str; 8] = [
    "open_waveform",
    "close_waveform",
    "list_signals",
    "read_hierarchy",
    "read_signal",
    "get_signal_info",
    "find_signal_events",
    "find_conditional_events",
];

/// Message for an unknown command, suggesting the closest command name.
pub fn unknown_command_message(name: &str) -> String {
    let candidates: Vec<String> = COMMAND_NAMES.iter().map(|name| name.to_string()).collect();
    let suggestions = did_you_mean(name, &candidates);

    if suggestions.len() == 1 {
        return format!(
            "Unknown command '{}'. Did you mean '{}'?",
            name, suggestions[0]
        );
    }

    if suggestions.len() > 1 {
        let quoted: Vec<String> = suggestions
            .iter()
            .map(|suggestion| format!("'{}'", suggestion))
            .collect();
        return format!(
            "Unknown command '{}'. Did you mean one of: {}?",
            name,
            quoted.join(", ")
        );
    }

    format!(
        "Unknown command '{}'. Available commands: {}. Run 'waveform-cli --help' for usage.",
        name,
        COMMAND_NAMES.join(", ")
    )
}

/// A signal path pattern together with the way it should be matched.
#[derive(Debug, Clone, PartialEq)]
pub struct SignalPattern {
    pub pattern: String,
    pub mode: MatchMode,
}

/// A parsed CLI command
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    OpenWaveform {
        file_path: String,
        alias: Option<String>,
    },
    CloseWaveform {
        waveform_id: String,
    },
    ListSignals {
        waveform_id: String,
        patterns: Vec<SignalPattern>,
        hierarchy_prefix: Option<String>,
        recursive: bool,
        limit: Option<isize>,
    },
    ReadHierarchy {
        waveform_id: String,
        scope_path: Option<String>,
        recursive: bool,
        limit: Option<isize>,
    },
    ReadSignal {
        waveform_id: String,
        signal_path: String,
        time_index: Option<usize>,
        time_indices: Option<Vec<usize>>,
    },
    GetSignalInfo {
        waveform_id: String,
        signal_path: String,
    },
    FindSignalEvents {
        waveform_id: String,
        signal_path: String,
        start_time_index: Option<usize>,
        end_time_index: Option<usize>,
        limit: Option<isize>,
    },
    FindConditionalEvents {
        waveform_id: String,
        condition: String,
        start_time_index: Option<usize>,
        end_time_index: Option<usize>,
        limit: Option<isize>,
    },
}

/// Parse command line arguments into a vector of commands
///
/// Commands can be chained using "--" as a separator.
pub fn parse_args(args: Vec<String>) -> Result<Vec<Command>, String> {
    if args.is_empty() {
        return Err("No arguments provided".to_string());
    }

    split_command_groups(&args)
        .into_iter()
        .map(parse_command)
        .collect()
}

/// Split the raw arguments into per-command groups using "--" as the separator.
pub fn split_command_groups(args: &[String]) -> Vec<Vec<String>> {
    let mut groups: Vec<Vec<String>> = vec![vec![]];
    for arg in args {
        if arg == "--" {
            groups.push(vec![]);
        } else {
            groups.last_mut().unwrap().push(arg.clone());
        }
    }
    groups.retain(|group| !group.is_empty());
    groups
}

/// The general usage/help text.
pub fn usage_text() -> String {
    [
        concat!(
            "waveform-cli ",
            env!("CARGO_PKG_VERSION"),
            " - command-line interface for reading and analyzing VCD/FST waveforms"
        ),
        "",
        "Usage:",
        "  waveform-cli <command> [args...] [-- <command> [args...] ...]",
        "  waveform-cli help [<command>]",
        "",
        "Commands can be chained with '--'; opened waveforms stay available to the",
        "commands that follow in the chain.",
        "",
        "Commands:",
        "  open_waveform            Open a VCD/FST waveform file",
        "  close_waveform           Close a waveform and free its memory",
        "  list_signals             List signals, optionally filtered by path patterns",
        "  read_hierarchy           Show the module hierarchy as an indented tree",
        "  read_signal              Read signal values at given time indices",
        "  get_signal_info          Show signal type, width and index range",
        "  find_signal_events       Find every change of a signal in a time range",
        "  find_conditional_events  Find events where an expression holds",
        "",
        "Options:",
        "  -h, --help     Show this help; 'help <command>' shows details for a command",
        "  -V, --version  Show the version",
        "",
        "Signal path patterns (list_signals):",
        "  --contains <p>  case-insensitive substring match (default; --pattern is an alias)",
        "  --glob <p>      case-insensitive glob over the whole path, e.g. '*bank*'",
        "  --regex <p>     regular expression, case-insensitive unless '(?-i)' is used",
        "  The flag selects the match mode and can be repeated to OR several patterns.",
        "",
        "Examples:",
        "  waveform-cli open_waveform test.vcd",
        "  waveform-cli open_waveform test.vcd -- list_signals test.vcd --glob '*bank*'",
        "  waveform-cli open_waveform test.vcd -- list_signals test.vcd --contains clk --hierarchy tb.dut",
        "  waveform-cli open_waveform test.vcd -- read_signal test.vcd tb.clk --time-index 0",
        "  waveform-cli open_waveform test.vcd -- find_signal_events test.vcd tb.reset --limit 10",
        "  waveform-cli help list_signals",
    ]
    .join("\n")
        + "\n"
}

/// Detailed help for a single command, or `None` if the command is unknown.
pub fn command_help(name: &str) -> Option<String> {
    let text = match name {
        "open_waveform" => {
            r#"Usage: waveform-cli open_waveform <file_path> [--alias <name>]

Open a VCD or FST waveform and keep it in memory for the rest of the chain.

Arguments:
  <file_path>         Path to a .vcd or .fst file

Options:
  -a, --alias <name>  Waveform id to use instead of the file name; this is the id that
                      later commands expect as <waveform_id>
"#
        }
        "close_waveform" => {
            r#"Usage: waveform-cli close_waveform <waveform_id>

Close a waveform and free its memory.

Arguments:
  <waveform_id>  Id or alias of an opened waveform
"#
        }
        "list_signals" => {
            r#"Usage: waveform-cli list_signals <waveform_id> [patterns] [options]

List the full path of every signal (e.g. tb.dut.abc_bank) that matches the patterns.
Without patterns every signal is listed.

Patterns (repeatable, OR-ed, one mode per command, all case-insensitive):
  -p, --contains <p>  Substring match; --pattern is an alias
  -g, --glob <p>      Glob over the whole path: '*bank*', 'tb.*.clk', 'bus[0-9]'
  -x, --regex <p>     Regular expression; use '(?-i)' to make it case-sensitive

Options:
  --hierarchy <prefix>      Only list signals below this scope, e.g. tb.dut
  -r, --recursive <bool>    Include signals from sub-scopes (default: true)
  -l, --limit <n>           Maximum number of signals, -1 for unlimited (default: 100)

Examples:
  waveform-cli list_signals wave --contains clk --hierarchy tb.dut
  waveform-cli list_signals wave --glob '*bank*'
  waveform-cli list_signals wave --regex '^tb\.dut\..*_valid$'
"#
        }
        "read_hierarchy" => {
            r#"Usage: waveform-cli read_hierarchy <waveform_id> [--scope <scope>] [--recursive <bool>] [--limit <n>]

Show the module hierarchy as an indented tree, starting at <scope> when given.
Only module scopes are shown.

Options:
  -s, --scope <scope>     Start at this scope path, e.g. tb.dut (--hierarchy is an alias)
  -r, --recursive <bool>  Include descendants (default: false)
  -l, --limit <n>         Maximum number of modules, -1 for unlimited (default: 200)
"#
        }
        "read_signal" => {
            r#"Usage: waveform-cli read_signal <waveform_id> <signal_path> (--time-index <idx> | --time-indices <list>)

Read signal values at one or more time indices (not timestamps).

Arguments:
  <signal_path>  Full signal path, e.g. tb.dut.abc_bank

Options:
  -t, --time-index <idx>     Single time index to read
  -T, --time-indices <list>  Comma-separated time indices, e.g. 0,5,10
"#
        }
        "get_signal_info" => {
            r#"Usage: waveform-cli get_signal_info <waveform_id> <signal_path>

Show the signal type, bit width and index range.

Arguments:
  <signal_path>  Full signal path, e.g. tb.dut.abc_bank
"#
        }
        "find_signal_events" => {
            r#"Usage: waveform-cli find_signal_events <waveform_id> <signal_path> [--start <idx>] [--end <idx>] [--limit <n>]

List every value change of a signal within a time index range.

Options:
  -s, --start <idx>  First time index of the range (default: 0)
  -e, --end <idx>    Last time index of the range (default: last index)
  -l, --limit <n>    Maximum number of events, -1 for unlimited (default: 100)
"#
        }
        "find_conditional_events" => {
            r#"Usage: waveform-cli find_conditional_events <waveform_id> <condition> [--start <idx>] [--end <idx>] [--limit <n>]

List the time indices where a condition holds.

Condition syntax:
  signal paths (TOP.signal), bitwise ~ & | ^, logical && || !, comparisons == !=,
  parentheses, $past(signal), bit extraction signal[bit] / signal[msb:lsb],
  and Verilog literals such as 4'b0101, 3'd2, 5'h1A.
  Precedence (high to low): ~ !, == !=, &, ^, |, &&, ||

Options:
  -s, --start <idx>  First time index of the range (default: 0)
  -e, --end <idx>    Last time index of the range (default: last index)
  -l, --limit <n>    Maximum number of events, -1 for unlimited (default: 100)

Examples:
  waveform-cli find_conditional_events wave 'tb.dut.valid && tb.dut.ready'
  waveform-cli find_conditional_events wave '!$past(tb.dut.valid) && tb.dut.valid'
"#
        }
        _ => return None,
    };

    Some(text.to_string())
}

/// Detect a help request without running any command.
///
/// * `["help"]`, `["--help"]`, `["-h"]` -> `Some(None)` (general help)
/// * `["help", "list_signals"]` -> `Some(Some("list_signals"))`
/// * `["list_signals", "wave", "--help"]` -> `Some(Some("list_signals"))`
pub fn help_request(args: &[String]) -> Option<Option<String>> {
    let first = args.first()?;

    if first == "help" {
        return Some(match args.get(1) {
            Some(name) if !name.starts_with('-') => Some(name.clone()),
            _ => None,
        });
    }

    if first == "--help" || first == "-h" {
        return Some(None);
    }

    for group in split_command_groups(args) {
        if group
            .iter()
            .skip(1)
            .any(|arg| arg == "--help" || arg == "-h")
        {
            return Some(Some(group[0].clone()));
        }
    }

    None
}

/// Parse a numeric option value, naming the flag when it fails.
fn parse_number<T>(flag: &str, value: &str) -> Result<T, String>
where
    T: std::str::FromStr,
{
    value
        .trim()
        .parse()
        .map_err(|_| format!("{} expects a number, got '{}'", flag, value))
}

/// Parse a boolean option value (`true`/`false`, also accepting yes/no and 1/0).
fn parse_bool(flag: &str, value: &str) -> Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "1" => Ok(true),
        "false" | "no" | "0" => Ok(false),
        _ => Err(format!("{} expects true or false, got '{}'", flag, value)),
    }
}

/// Parse a comma-separated list of numbers, e.g. `0,5,10`.
fn parse_number_list<T>(flag: &str, value: &str) -> Result<Vec<T>, String>
where
    T: std::str::FromStr,
{
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| parse_number(flag, part))
        .collect()
}

/// Parse a single command from a group of arguments
fn parse_command(group: Vec<String>) -> Result<Command, String> {
    if group.is_empty() {
        return Err("Empty command group".to_string());
    }

    let cmd_name = &group[0];
    let cmd_args: Vec<String> = group[1..].to_vec();

    match cmd_name.as_str() {
        "open_waveform" => parse_open_waveform(&cmd_args),
        "close_waveform" => parse_close_waveform(&cmd_args),
        "list_signals" => parse_list_signals(&cmd_args),
        "read_hierarchy" => parse_read_hierarchy(&cmd_args),
        "read_signal" => parse_read_signal(&cmd_args),
        "get_signal_info" => parse_get_signal_info(&cmd_args),
        "find_signal_events" => parse_find_signal_events(&cmd_args),
        "find_conditional_events" => parse_find_conditional_events(&cmd_args),
        _ => Err(unknown_command_message(cmd_name)),
    }
}

fn parse_open_waveform(args: &[String]) -> Result<Command, String> {
    if args.is_empty() {
        return Err("open_waveform requires a file path".to_string());
    }

    let file_path = args[0].clone();
    let mut alias = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--alias" | "-a" => {
                i += 1;
                if i < args.len() {
                    alias = Some(args[i].clone());
                } else {
                    return Err("--alias requires a value".to_string());
                }
            }
            _ => return Err(format!("Unknown option '{}' for open_waveform", args[i])),
        }
        i += 1;
    }

    Ok(Command::OpenWaveform { file_path, alias })
}

fn parse_close_waveform(args: &[String]) -> Result<Command, String> {
    if args.is_empty() {
        return Err("close_waveform requires a waveform_id".to_string());
    }

    Ok(Command::CloseWaveform {
        waveform_id: args[0].clone(),
    })
}

fn parse_list_signals(args: &[String]) -> Result<Command, String> {
    if args.is_empty() {
        return Err("list_signals requires a waveform_id".to_string());
    }

    let waveform_id = args[0].clone();
    let mut patterns: Vec<SignalPattern> = Vec::new();
    // The match mode is implied by the flag used, and only one mode is allowed per command.
    let mut selected_mode: Option<MatchMode> = None;
    let mut hierarchy_prefix = None;
    let mut recursive = true;
    let mut limit = Some(100isize);

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--contains" | "--pattern" | "-p" => {
                let flag = args[i].clone();
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| format!("{} requires a value", flag))?;
                add_pattern(
                    &mut patterns,
                    &mut selected_mode,
                    value,
                    MatchMode::Substring,
                )?;
            }
            "--glob" | "-g" => {
                let flag = args[i].clone();
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| format!("{} requires a value", flag))?;
                add_pattern(&mut patterns, &mut selected_mode, value, MatchMode::Glob)?;
            }
            "--regex" | "-x" => {
                let flag = args[i].clone();
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| format!("{} requires a value", flag))?;
                add_pattern(&mut patterns, &mut selected_mode, value, MatchMode::Regex)?;
            }
            "--hierarchy" => {
                i += 1;
                if i < args.len() {
                    hierarchy_prefix = Some(args[i].clone());
                } else {
                    return Err("--hierarchy requires a value".to_string());
                }
            }
            "--recursive" | "-r" => {
                i += 1;
                if i < args.len() {
                    recursive = parse_bool("--recursive", &args[i])?;
                } else {
                    return Err("--recursive requires a value (true/false)".to_string());
                }
            }
            "--limit" | "-l" => {
                i += 1;
                if i < args.len() {
                    limit = Some(parse_number("--limit", &args[i])?);
                } else {
                    return Err("--limit requires a value".to_string());
                }
            }
            _ => return Err(format!("Unknown option '{}' for list_signals", args[i])),
        }
        i += 1;
    }

    Ok(Command::ListSignals {
        waveform_id,
        patterns,
        hierarchy_prefix,
        recursive,
        limit,
    })
}

/// Record a pattern, rejecting mixtures of match modes.
///
/// The match mode is selected by the flag used (`--contains`, `--glob`, or `--regex`) and only
/// one mode may be used per command; multiple patterns of the same mode are OR-ed.
fn add_pattern(
    patterns: &mut Vec<SignalPattern>,
    selected_mode: &mut Option<MatchMode>,
    pattern: &str,
    mode: MatchMode,
) -> Result<(), String> {
    if let Some(current) = *selected_mode
        && current != mode
    {
        return Err(format!(
            "{} cannot be combined with {}; use only one match mode per list_signals",
            mode_flag(mode),
            mode_flag(current)
        ));
    }

    *selected_mode = Some(mode);
    patterns.push(SignalPattern {
        pattern: pattern.to_string(),
        mode,
    });
    Ok(())
}

/// The CLI flag that selects `mode`.
fn mode_flag(mode: MatchMode) -> &'static str {
    match mode {
        MatchMode::Substring => "--contains",
        MatchMode::Glob => "--glob",
        MatchMode::Regex => "--regex",
    }
}

fn parse_read_hierarchy(args: &[String]) -> Result<Command, String> {
    if args.is_empty() {
        return Err("read_hierarchy requires a waveform_id".to_string());
    }

    let waveform_id = args[0].clone();
    let mut scope_path = None;
    let mut recursive = false;
    let mut limit = Some(200isize);

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--scope" | "--hierarchy" | "-s" => {
                i += 1;
                if i < args.len() {
                    scope_path = Some(args[i].clone());
                } else {
                    return Err("--scope requires a value".to_string());
                }
            }
            "--recursive" | "-r" => {
                i += 1;
                if i < args.len() {
                    recursive = parse_bool("--recursive", &args[i])?;
                } else {
                    return Err("--recursive requires a value (true/false)".to_string());
                }
            }
            "--limit" | "-l" => {
                i += 1;
                if i < args.len() {
                    limit = Some(parse_number("--limit", &args[i])?);
                } else {
                    return Err("--limit requires a value".to_string());
                }
            }
            _ => return Err(format!("Unknown option '{}' for read_hierarchy", args[i])),
        }
        i += 1;
    }

    Ok(Command::ReadHierarchy {
        waveform_id,
        scope_path,
        recursive,
        limit,
    })
}

fn parse_read_signal(args: &[String]) -> Result<Command, String> {
    if args.len() < 2 {
        return Err("read_signal requires waveform_id and signal_path".to_string());
    }

    let waveform_id = args[0].clone();
    let signal_path = args[1].clone();
    let mut time_index = None;
    let mut time_indices = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--time-index" | "-t" => {
                i += 1;
                if i < args.len() {
                    time_index = Some(parse_number("--time-index", &args[i])?);
                } else {
                    return Err("--time-index requires a value".to_string());
                }
            }
            "--time-indices" | "-T" => {
                i += 1;
                if i < args.len() {
                    time_indices = Some(parse_number_list("--time-indices", &args[i])?);
                } else {
                    return Err("--time-indices requires a value".to_string());
                }
            }
            _ => return Err(format!("Unknown option '{}' for read_signal", args[i])),
        }
        i += 1;
    }

    if time_index.is_some() && time_indices.is_some() {
        return Err("provide either --time-index or --time-indices, not both".to_string());
    }

    Ok(Command::ReadSignal {
        waveform_id,
        signal_path,
        time_index,
        time_indices,
    })
}

fn parse_get_signal_info(args: &[String]) -> Result<Command, String> {
    if args.len() < 2 {
        return Err("get_signal_info requires waveform_id and signal_path".to_string());
    }

    Ok(Command::GetSignalInfo {
        waveform_id: args[0].clone(),
        signal_path: args[1].clone(),
    })
}

fn parse_find_signal_events(args: &[String]) -> Result<Command, String> {
    if args.len() < 2 {
        return Err("find_signal_events requires waveform_id and signal_path".to_string());
    }

    let waveform_id = args[0].clone();
    let signal_path = args[1].clone();
    let mut start_time_index = None;
    let mut end_time_index = None;
    let mut limit = Some(100isize);

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--start" | "-s" => {
                i += 1;
                if i < args.len() {
                    start_time_index = Some(parse_number("--start", &args[i])?);
                } else {
                    return Err("--start requires a value".to_string());
                }
            }
            "--end" | "-e" => {
                i += 1;
                if i < args.len() {
                    end_time_index = Some(parse_number("--end", &args[i])?);
                } else {
                    return Err("--end requires a value".to_string());
                }
            }
            "--limit" | "-l" => {
                i += 1;
                if i < args.len() {
                    limit = Some(parse_number("--limit", &args[i])?);
                } else {
                    return Err("--limit requires a value".to_string());
                }
            }
            _ => {
                return Err(format!(
                    "Unknown option '{}' for find_signal_events",
                    args[i]
                ));
            }
        }
        i += 1;
    }

    Ok(Command::FindSignalEvents {
        waveform_id,
        signal_path,
        start_time_index,
        end_time_index,
        limit,
    })
}

fn parse_find_conditional_events(args: &[String]) -> Result<Command, String> {
    if args.len() < 2 {
        return Err("find_conditional_events requires waveform_id and condition".to_string());
    }

    let waveform_id = args[0].clone();
    let condition = args[1].clone();
    let mut start_time_index = None;
    let mut end_time_index = None;
    let mut limit = Some(100isize);

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--start" | "-s" => {
                i += 1;
                if i < args.len() {
                    start_time_index = Some(parse_number("--start", &args[i])?);
                } else {
                    return Err("--start requires a value".to_string());
                }
            }
            "--end" | "-e" => {
                i += 1;
                if i < args.len() {
                    end_time_index = Some(parse_number("--end", &args[i])?);
                } else {
                    return Err("--end requires a value".to_string());
                }
            }
            "--limit" | "-l" => {
                i += 1;
                if i < args.len() {
                    limit = Some(parse_number("--limit", &args[i])?);
                } else {
                    return Err("--limit requires a value".to_string());
                }
            }
            _ => {
                return Err(format!(
                    "Unknown option '{}' for find_conditional_events",
                    args[i]
                ));
            }
        }
        i += 1;
    }

    Ok(Command::FindConditionalEvents {
        waveform_id,
        condition,
        start_time_index,
        end_time_index,
        limit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_args() {
        let result = parse_args(vec![]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No arguments provided");
    }

    #[test]
    fn test_parse_open_waveform() {
        let args = vec!["open_waveform".to_string(), "/path/to/test.vcd".to_string()];
        let result = parse_args(args);
        assert!(result.is_ok());
        let commands = result.unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(
            commands[0],
            Command::OpenWaveform {
                file_path: "/path/to/test.vcd".to_string(),
                alias: None,
            }
        );
    }

    #[test]
    fn test_parse_open_waveform_with_alias() {
        let args = vec![
            "open_waveform".to_string(),
            "/path/to/test.vcd".to_string(),
            "--alias".to_string(),
            "mywave".to_string(),
        ];
        let result = parse_args(args);
        assert!(result.is_ok());
        let commands = result.unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(
            commands[0],
            Command::OpenWaveform {
                file_path: "/path/to/test.vcd".to_string(),
                alias: Some("mywave".to_string()),
            }
        );
    }

    #[test]
    fn test_parse_open_waveform_with_short_alias() {
        let args = vec![
            "open_waveform".to_string(),
            "/path/to/test.vcd".to_string(),
            "-a".to_string(),
            "wave1".to_string(),
        ];
        let result = parse_args(args);
        assert!(result.is_ok());
        let commands = result.unwrap();
        assert_eq!(
            commands[0].clone(),
            Command::OpenWaveform {
                file_path: "/path/to/test.vcd".to_string(),
                alias: Some("wave1".to_string()),
            }
        );
    }

    #[test]
    fn test_parse_close_waveform() {
        let args = vec!["close_waveform".to_string(), "test.vcd".to_string()];
        let result = parse_args(args);
        assert!(result.is_ok());
        let commands = result.unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(
            commands[0],
            Command::CloseWaveform {
                waveform_id: "test.vcd".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_list_signals_minimal() {
        let args = vec!["list_signals".to_string(), "test.vcd".to_string()];
        let result = parse_args(args);
        assert!(result.is_ok());
        let commands = result.unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(
            commands[0],
            Command::ListSignals {
                waveform_id: "test.vcd".to_string(),
                patterns: vec![],
                hierarchy_prefix: None,
                recursive: true,
                limit: Some(100),
            }
        );
    }

    #[test]
    fn test_parse_list_signals_with_pattern() {
        let args = vec![
            "list_signals".to_string(),
            "test.vcd".to_string(),
            "--pattern".to_string(),
            "clk".to_string(),
        ];
        let result = parse_args(args);
        assert!(result.is_ok());
        let commands = result.unwrap();
        assert_eq!(
            commands[0].clone(),
            Command::ListSignals {
                waveform_id: "test.vcd".to_string(),
                patterns: vec![SignalPattern {
                    pattern: "clk".to_string(),
                    mode: MatchMode::Substring,
                }],
                hierarchy_prefix: None,
                recursive: true,
                limit: Some(100),
            }
        );
    }

    #[test]
    fn test_parse_list_signals_with_all_options() {
        let args = vec![
            "list_signals".to_string(),
            "test.vcd".to_string(),
            "-p".to_string(),
            "data".to_string(),
            "--hierarchy".to_string(),
            "top".to_string(),
            "-r".to_string(),
            "false".to_string(),
            "-l".to_string(),
            "50".to_string(),
        ];
        let result = parse_args(args);
        assert!(result.is_ok());
        let commands = result.unwrap();
        assert_eq!(
            commands[0].clone(),
            Command::ListSignals {
                waveform_id: "test.vcd".to_string(),
                patterns: vec![SignalPattern {
                    pattern: "data".to_string(),
                    mode: MatchMode::Substring,
                }],
                hierarchy_prefix: Some("top".to_string()),
                recursive: false,
                limit: Some(50),
            }
        );
    }

    #[test]
    fn test_parse_list_signals_with_repeated_glob_patterns() {
        let args = vec![
            "list_signals".to_string(),
            "test.vcd".to_string(),
            "--glob".to_string(),
            "*bank*".to_string(),
            "-g".to_string(),
            "*ram*".to_string(),
        ];
        let result = parse_args(args);
        assert!(result.is_ok());
        let commands = result.unwrap();
        assert_eq!(
            commands[0].clone(),
            Command::ListSignals {
                waveform_id: "test.vcd".to_string(),
                patterns: vec![
                    SignalPattern {
                        pattern: "*bank*".to_string(),
                        mode: MatchMode::Glob,
                    },
                    SignalPattern {
                        pattern: "*ram*".to_string(),
                        mode: MatchMode::Glob,
                    },
                ],
                hierarchy_prefix: None,
                recursive: true,
                limit: Some(100),
            }
        );
    }

    #[test]
    fn test_parse_list_signals_with_regex_patterns() {
        let args = vec![
            "list_signals".to_string(),
            "test.vcd".to_string(),
            "--regex".to_string(),
            "^tb\\.dut\\.ram\\d+$".to_string(),
            "-p".to_string(),
            "clk".to_string(),
        ];
        let result = parse_args(args);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "--contains cannot be combined with --regex; use only one match mode per list_signals"
        );
    }

    #[test]
    fn test_parse_list_signals_rejects_mixed_match_modes() {
        let args = vec![
            "list_signals".to_string(),
            "test.vcd".to_string(),
            "--glob".to_string(),
            "*bank*".to_string(),
            "--regex".to_string(),
            "^tb\\.dut\\.ram\\d+$".to_string(),
        ];
        let result = parse_args(args);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "--regex cannot be combined with --glob; use only one match mode per list_signals"
        );
    }

    #[test]
    fn test_help_request_general() {
        for args in [
            vec!["--help"],
            vec!["-h"],
            vec!["help"],
            vec!["help", "--help"],
        ] {
            let args: Vec<String> = args.into_iter().map(String::from).collect();
            assert_eq!(help_request(&args), Some(None), "args: {:?}", args);
        }
    }

    #[test]
    fn test_help_request_for_command() {
        let args: Vec<String> = vec!["help", "list_signals"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(help_request(&args), Some(Some("list_signals".to_string())));

        let args: Vec<String> = vec!["list_signals", "wave", "--help"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(help_request(&args), Some(Some("list_signals".to_string())));

        // `-h` is help, not the hierarchy filter.
        let args: Vec<String> = vec!["list_signals", "wave", "-h"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(help_request(&args), Some(Some("list_signals".to_string())));

        // The second command in a chain can ask for help too.
        let args: Vec<String> = vec!["open_waveform", "wave.vcd", "--", "read_signal", "--help"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(help_request(&args), Some(Some("read_signal".to_string())));
    }

    #[test]
    fn test_help_request_none_for_normal_commands() {
        let args: Vec<String> = vec!["open_waveform", "test.vcd"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(help_request(&args), None);
        assert_eq!(help_request(&[]), None);
    }

    #[test]
    fn test_usage_text_mentions_commands_and_pattern_flags() {
        let usage = usage_text();
        for expected in [
            "Usage:",
            "open_waveform",
            "find_conditional_events",
            "help <command>",
            "--contains",
            "--glob",
            "--regex",
            "--version",
        ] {
            assert!(usage.contains(expected), "usage text is missing {expected}");
        }
    }

    #[test]
    fn test_command_help_covers_every_command() {
        for name in [
            "open_waveform",
            "close_waveform",
            "list_signals",
            "read_hierarchy",
            "read_signal",
            "get_signal_info",
            "find_signal_events",
            "find_conditional_events",
        ] {
            let help = command_help(name).unwrap_or_else(|| panic!("no help for {name}"));
            assert!(
                help.starts_with("Usage:"),
                "help for {name} lacks usage line"
            );
            assert!(
                help.contains("waveform-cli"),
                "help for {name} lacks command name"
            );
        }
        assert!(command_help("nope").is_none());
    }

    #[test]
    fn test_list_signals_help_documents_patterns() {
        let help = command_help("list_signals").expect("help exists");
        for expected in ["--contains", "--glob", "--regex", "--hierarchy", "--limit"] {
            assert!(help.contains(expected), "help is missing {expected}");
        }
    }

    #[test]
    fn test_split_command_groups() {
        let args: Vec<String> = vec!["a", "--", "b", "c", "--", "--", "d"]
            .into_iter()
            .map(String::from)
            .collect();
        let groups = split_command_groups(&args);
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0], vec!["a".to_string()]);
        assert_eq!(groups[1], vec!["b".to_string(), "c".to_string()]);
        assert_eq!(groups[2], vec!["d".to_string()]);
    }

    #[test]
    fn test_parse_read_signal_with_time_index() {
        let args = vec![
            "read_signal".to_string(),
            "test.vcd".to_string(),
            "top.clk".to_string(),
            "--time-index".to_string(),
            "5".to_string(),
        ];
        let result = parse_args(args);
        assert!(result.is_ok());
        let commands = result.unwrap();
        assert_eq!(
            commands[0].clone(),
            Command::ReadSignal {
                waveform_id: "test.vcd".to_string(),
                signal_path: "top.clk".to_string(),
                time_index: Some(5),
                time_indices: None,
            }
        );
    }

    #[test]
    fn test_parse_read_hierarchy_minimal() {
        let args = vec!["read_hierarchy".to_string(), "test.vcd".to_string()];
        let result = parse_args(args);
        assert!(result.is_ok());
        let commands = result.unwrap();
        assert_eq!(
            commands[0].clone(),
            Command::ReadHierarchy {
                waveform_id: "test.vcd".to_string(),
                scope_path: None,
                recursive: false,
                limit: Some(200),
            }
        );
    }

    #[test]
    fn test_parse_read_hierarchy_with_options() {
        let args = vec![
            "read_hierarchy".to_string(),
            "test.vcd".to_string(),
            "--scope".to_string(),
            "top.submodule".to_string(),
            "--recursive".to_string(),
            "true".to_string(),
            "--limit".to_string(),
            "50".to_string(),
        ];
        let result = parse_args(args);
        assert!(result.is_ok());
        let commands = result.unwrap();
        assert_eq!(
            commands[0].clone(),
            Command::ReadHierarchy {
                waveform_id: "test.vcd".to_string(),
                scope_path: Some("top.submodule".to_string()),
                recursive: true,
                limit: Some(50),
            }
        );
    }

    #[test]
    fn test_parse_read_signal_with_time_indices() {
        let args = vec![
            "read_signal".to_string(),
            "test.vcd".to_string(),
            "top.data".to_string(),
            "--time-indices".to_string(),
            "0,1,2,3".to_string(),
        ];
        let result = parse_args(args);
        assert!(result.is_ok());
        let commands = result.unwrap();
        assert_eq!(
            commands[0].clone(),
            Command::ReadSignal {
                waveform_id: "test.vcd".to_string(),
                signal_path: "top.data".to_string(),
                time_index: None,
                time_indices: Some(vec![0, 1, 2, 3]),
            }
        );
    }

    #[test]
    fn test_parse_get_signal_info() {
        let args = vec![
            "get_signal_info".to_string(),
            "test.vcd".to_string(),
            "top.clk".to_string(),
        ];
        let result = parse_args(args);
        assert!(result.is_ok());
        let commands = result.unwrap();
        assert_eq!(
            commands[0].clone(),
            Command::GetSignalInfo {
                waveform_id: "test.vcd".to_string(),
                signal_path: "top.clk".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_find_signal_events() {
        let args = vec![
            "find_signal_events".to_string(),
            "test.vcd".to_string(),
            "top.clk".to_string(),
            "--start".to_string(),
            "0".to_string(),
            "--end".to_string(),
            "100".to_string(),
            "--limit".to_string(),
            "10".to_string(),
        ];
        let result = parse_args(args);
        assert!(result.is_ok());
        let commands = result.unwrap();
        assert_eq!(
            commands[0].clone(),
            Command::FindSignalEvents {
                waveform_id: "test.vcd".to_string(),
                signal_path: "top.clk".to_string(),
                start_time_index: Some(0),
                end_time_index: Some(100),
                limit: Some(10),
            }
        );
    }

    #[test]
    fn test_parse_find_conditional_events() {
        let args = vec![
            "find_conditional_events".to_string(),
            "test.vcd".to_string(),
            "top.clk == 1'b1".to_string(),
            "-s".to_string(),
            "10".to_string(),
            "-e".to_string(),
            "50".to_string(),
        ];
        let result = parse_args(args);
        assert!(result.is_ok());
        let commands = result.unwrap();
        assert_eq!(
            commands[0].clone(),
            Command::FindConditionalEvents {
                waveform_id: "test.vcd".to_string(),
                condition: "top.clk == 1'b1".to_string(),
                start_time_index: Some(10),
                end_time_index: Some(50),
                limit: Some(100),
            }
        );
    }

    #[test]
    fn test_parse_rejects_invalid_numeric_and_boolean_values() {
        for (args, expected) in [
            (
                vec!["list_signals", "wave", "--limit", "abc"],
                "--limit expects a number, got 'abc'",
            ),
            (
                vec!["list_signals", "wave", "--recursive", "maybe"],
                "--recursive expects true or false, got 'maybe'",
            ),
            (
                vec!["read_hierarchy", "wave", "--limit", "-"],
                "--limit expects a number, got '-'",
            ),
            (
                vec!["read_signal", "wave", "tb.clk", "--time-index", "x"],
                "--time-index expects a number, got 'x'",
            ),
            (
                vec!["read_signal", "wave", "tb.clk", "--time-indices", "0,x,2"],
                "--time-indices expects a number, got 'x'",
            ),
            (
                vec!["find_signal_events", "wave", "tb.clk", "--start", "1.5"],
                "--start expects a number, got '1.5'",
            ),
            (
                vec!["find_conditional_events", "wave", "tb.clk", "--end", "late"],
                "--end expects a number, got 'late'",
            ),
        ] {
            let args: Vec<String> = args.into_iter().map(String::from).collect();
            assert_eq!(parse_args(args).unwrap_err(), expected);
        }
    }

    #[test]
    fn test_parse_accepts_boolean_aliases() {
        let args: Vec<String> = vec!["list_signals", "wave", "-r", "no"]
            .into_iter()
            .map(String::from)
            .collect();
        match &parse_args(args).unwrap()[0] {
            Command::ListSignals { recursive, .. } => assert!(!recursive),
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn test_parse_read_signal_rejects_two_index_forms() {
        let args: Vec<String> = vec![
            "read_signal",
            "wave",
            "tb.clk",
            "--time-index",
            "0",
            "--time-indices",
            "1,2",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        assert_eq!(
            parse_args(args).unwrap_err(),
            "provide either --time-index or --time-indices, not both"
        );
    }

    #[test]
    fn test_parse_time_indices_ignores_blank_entries() {
        let args: Vec<String> = vec!["read_signal", "wave", "tb.clk", "-T", "0, ,2"]
            .into_iter()
            .map(String::from)
            .collect();
        match &parse_args(args).unwrap()[0] {
            Command::ReadSignal { time_indices, .. } => {
                assert_eq!(time_indices, &Some(vec![0, 2]));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn test_parse_chained_commands() {
        let args = vec![
            "open_waveform".to_string(),
            "test.vcd".to_string(),
            "--".to_string(),
            "list_signals".to_string(),
            "test.vcd".to_string(),
            "--".to_string(),
            "close_waveform".to_string(),
            "test.vcd".to_string(),
        ];
        let result = parse_args(args);
        assert!(result.is_ok());
        let commands = result.unwrap();
        assert_eq!(commands.len(), 3);

        // First command: open_waveform
        assert_eq!(
            commands[0].clone(),
            Command::OpenWaveform {
                file_path: "test.vcd".to_string(),
                alias: None,
            }
        );

        // Second command: list_signals
        assert_eq!(
            commands[1].clone(),
            Command::ListSignals {
                waveform_id: "test.vcd".to_string(),
                patterns: vec![],
                hierarchy_prefix: None,
                recursive: true,
                limit: Some(100),
            }
        );

        // Third command: close_waveform
        assert_eq!(
            commands[2].clone(),
            Command::CloseWaveform {
                waveform_id: "test.vcd".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_unknown_command() {
        let args = vec!["unknown_command".to_string(), "arg1".to_string()];
        let result = parse_args(args);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(
            error.contains("Unknown command 'unknown_command'"),
            "unexpected error: {error}"
        );
        assert!(
            error.contains("Available commands"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn test_unknown_command_suggests_similar_names() {
        let args: Vec<String> = vec!["list_signal", "wave"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(
            parse_args(args).unwrap_err(),
            "Unknown command 'list_signal'. Did you mean 'list_signals'?"
        );

        assert_eq!(
            unknown_command_message("open_wavefor"),
            "Unknown command 'open_wavefor'. Did you mean 'open_waveform'?"
        );
    }

    #[test]
    fn test_parse_missing_required_args() {
        let args = vec!["open_waveform".to_string()];
        let result = parse_args(args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "open_waveform requires a file path");
    }

    #[test]
    fn test_parse_missing_list_signals_waveform_id() {
        let args = vec!["list_signals".to_string()];
        let result = parse_args(args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "list_signals requires a waveform_id");
    }

    #[test]
    fn test_parse_missing_read_signal_args() {
        let args = vec!["read_signal".to_string(), "test.vcd".to_string()];
        let result = parse_args(args);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "read_signal requires waveform_id and signal_path"
        );
    }
}
