//! Command-line parsing for waveform-cli

use crate::matcher::MatchMode;

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
        _ => Err(format!("Unknown command '{}'", cmd_name)),
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
            "--hierarchy" | "-h" => {
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
                    recursive = args[i].parse().unwrap_or(true);
                } else {
                    return Err("--recursive requires a value (true/false)".to_string());
                }
            }
            "--limit" | "-l" => {
                i += 1;
                if i < args.len() {
                    limit = args[i].parse().ok();
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
                    recursive = args[i].parse().unwrap_or(false);
                } else {
                    return Err("--recursive requires a value (true/false)".to_string());
                }
            }
            "--limit" | "-l" => {
                i += 1;
                if i < args.len() {
                    limit = args[i].parse().ok();
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
                    time_index = args[i].parse().ok();
                } else {
                    return Err("--time-index requires a value".to_string());
                }
            }
            "--time-indices" | "-T" => {
                i += 1;
                if i < args.len() {
                    time_indices =
                        Some(args[i].split(',').filter_map(|s| s.parse().ok()).collect());
                } else {
                    return Err("--time-indices requires a value".to_string());
                }
            }
            _ => return Err(format!("Unknown option '{}' for read_signal", args[i])),
        }
        i += 1;
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
                    start_time_index = args[i].parse().ok();
                } else {
                    return Err("--start requires a value".to_string());
                }
            }
            "--end" | "-e" => {
                i += 1;
                if i < args.len() {
                    end_time_index = args[i].parse().ok();
                } else {
                    return Err("--end requires a value".to_string());
                }
            }
            "--limit" | "-l" => {
                i += 1;
                if i < args.len() {
                    limit = args[i].parse().ok();
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
                    start_time_index = args[i].parse().ok();
                } else {
                    return Err("--start requires a value".to_string());
                }
            }
            "--end" | "-e" => {
                i += 1;
                if i < args.len() {
                    end_time_index = args[i].parse().ok();
                } else {
                    return Err("--end requires a value".to_string());
                }
            }
            "--limit" | "-l" => {
                i += 1;
                if i < args.len() {
                    limit = args[i].parse().ok();
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
            "-h".to_string(),
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
        assert_eq!(result.unwrap_err(), "Unknown command 'unknown_command'");
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
