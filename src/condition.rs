//! Condition parsing and evaluation for conditional event search.

use super::{
    formatting::format_signal_value, formatting::format_time, hierarchy::find_signal_by_path,
};
use lalrpop_util::lalrpop_mod;
use wellen;

// Import generated parser
lalrpop_mod!(condition);

/// Literal value for signal comparison.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Literal {
    Binary(Vec<bool>),
    Decimal(u64),
    Hexadecimal(u64),
}

/// Condition for finding events based on signal values.
#[derive(Debug, Clone)]
pub(super) enum Condition {
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Not(Box<Condition>),
    Signal(String),
    Eq(Box<Condition>, Box<Condition>),
    Neq(Box<Condition>, Box<Condition>),
    Literal(Literal),
    Past(Box<Condition>),
}

/// Parse a simple condition string into a Condition AST.
///
/// Supports:
/// - Signal paths (e.g., "TOP.signal")
/// - `&&` for AND
/// - `||` for OR
/// - `!` for NOT
/// - `==` for equality comparison
/// - `!=` for inequality comparison
/// - `$past(signal)` to read signal value from previous time index
/// - Parentheses for grouping
/// - Verilog-style literals: 4'b0101, 3'd2, 5'h1A
///
/// Uses lalrpop-generated parser.
pub(super) fn parse_condition(condition: &str) -> Result<Condition, String> {
    let parser = crate::condition::condition::ExprParser::new();
    parser.parse(condition).map_err(|e| e.to_string())
}

/// Evaluate a condition at a specific time index.
///
/// # Arguments
/// * `condition` - The condition to evaluate
/// * `waveform` - The waveform to read from
/// * `signal_cache` - Cache of signal references and loaded signals
/// * `time_idx` - The time index to evaluate at
///
/// # Returns
/// An i64 value where 0 = false and any non-zero value = true.
fn evaluate_condition(
    condition: &Condition,
    waveform: &mut wellen::simple::Waveform,
    signal_cache: &std::collections::HashMap<String, wellen::SignalRef>,
    time_idx: usize,
) -> Result<i64, String> {
    match condition {
        Condition::And(left, right) => {
            let left_val = evaluate_condition(left, waveform, signal_cache, time_idx)?;
            let right_val = evaluate_condition(right, waveform, signal_cache, time_idx)?;
            Ok(if left_val != 0 && right_val != 0 {
                1
            } else {
                0
            })
        }
        Condition::Or(left, right) => {
            let left_val = evaluate_condition(left, waveform, signal_cache, time_idx)?;
            let right_val = evaluate_condition(right, waveform, signal_cache, time_idx)?;
            Ok(if left_val != 0 || right_val != 0 {
                1
            } else {
                0
            })
        }
        Condition::Not(expr) => {
            let val = evaluate_condition(expr, waveform, signal_cache, time_idx)?;
            Ok(if val != 0 { 0 } else { 1 })
        }
        Condition::Signal(path) => {
            // Get signal ref from cache
            let signal_ref = signal_cache
                .get(path)
                .ok_or_else(|| format!("Signal not found in cache: {}", path))?;

            // Get signal from waveform
            let signal = waveform
                .get_signal(*signal_ref)
                .ok_or_else(|| format!("Signal not found in waveform: {}", path))?;

            // Get value at time index
            let time_table_idx: wellen::TimeTableIdx = time_idx
                .try_into()
                .map_err(|_| format!("Time index {} too large", time_idx))?;

            let offset = signal
                .get_offset(time_table_idx)
                .ok_or_else(|| format!("No data for signal {} at time index {}", path, time_idx))?;

            let signal_value = signal.get_value_at(&offset, 0);
            signal_value_to_i64(signal_value)
        }
        Condition::Eq(left, right) => {
            let left_val = evaluate_condition(left, waveform, signal_cache, time_idx)?;
            let right_val = evaluate_condition(right, waveform, signal_cache, time_idx)?;
            Ok(if left_val == right_val { 1 } else { 0 })
        }
        Condition::Neq(left, right) => {
            let left_val = evaluate_condition(left, waveform, signal_cache, time_idx)?;
            let right_val = evaluate_condition(right, waveform, signal_cache, time_idx)?;
            Ok(if left_val != right_val { 1 } else { 0 })
        }
        Condition::Literal(literal) => literal_to_i64(literal),
        Condition::Past(expr) => {
            // If at time 0, there's no previous value
            if time_idx == 0 {
                return Ok(0); // Return false (0) when there's no past
            }
            // Evaluate expression at previous time index
            evaluate_condition(expr, waveform, signal_cache, time_idx - 1)
        }
    }
}

/// Convert a signal value to i64 for comparison.
fn signal_value_to_i64(signal_value: wellen::SignalValue) -> Result<i64, String> {
    match signal_value {
        wellen::SignalValue::Binary(data, _) => {
            let mut value: i64 = 0;
            for (i, &b) in data.iter().enumerate() {
                if b != 0 {
                    value |= (b as i64) << i;
                }
            }
            Ok(value)
        }
        wellen::SignalValue::FourValue(data, _) => {
            let mut value: i64 = 0;
            for (i, &b) in data.iter().enumerate() {
                if b != 0 {
                    value |= (b as i64) << i;
                }
            }
            Ok(value)
        }
        wellen::SignalValue::NineValue(data, _) => {
            let mut value: i64 = 0;
            for (i, &b) in data.iter().enumerate() {
                if b != 0 {
                    value |= (b as i64) << i;
                }
            }
            Ok(value)
        }
        wellen::SignalValue::String(s) => {
            // Handle special string values for boolean context
            if s == "1" || s.eq_ignore_ascii_case("true") {
                Ok(1)
            } else if s == "0" || s.eq_ignore_ascii_case("false") {
                Ok(0)
            } else {
                s.parse()
                    .map_err(|_| format!("Cannot convert string '{}' to integer", s))
            }
        }
        wellen::SignalValue::Real(r) => Ok(r as i64),
        wellen::SignalValue::Event => Err("Event signal cannot be compared".to_string()),
    }
}

/// Convert a literal to i64 for comparison.
fn literal_to_i64(literal: &Literal) -> Result<i64, String> {
    match literal {
        Literal::Binary(bits) => {
            let mut value: i64 = 0;
            for (i, &bit) in bits.iter().rev().enumerate() {
                if bit {
                    value |= 1i64 << i;
                }
            }
            Ok(value)
        }
        Literal::Decimal(v) => Ok(*v as i64),
        Literal::Hexadecimal(v) => Ok(*v as i64),
    }
}

/// Parse a binary literal (e.g., "4'b0101") from the condition grammar.
///
/// This function is called by the lalrpop-generated parser.
pub(super) fn parse_binary_literal(s: &str) -> Literal {
    let lower = s.to_lowercase();
    let parts: Vec<&str> = lower.split('\'').collect();
    if parts.len() != 2 {
        panic!("Invalid binary literal: {}", s);
    }

    let value_str = parts[1].trim_start_matches('b').replace('_', "");
    let mut bits = Vec::new();
    for c in value_str.chars() {
        match c {
            '0' => bits.push(false),
            '1' => bits.push(true),
            _ => panic!("Invalid binary digit: {}", c),
        }
    }
    Literal::Binary(bits)
}

/// Parse a decimal literal (e.g., "3'd2") from the condition grammar.
///
/// This function is called by the lalrpop-generated parser.
pub(super) fn parse_decimal_literal(s: &str) -> Literal {
    let lower = s.to_lowercase();
    let parts: Vec<&str> = lower.split('\'').collect();
    if parts.len() != 2 {
        panic!("Invalid decimal literal: {}", s);
    }

    let value_str = parts[1].trim_start_matches('d').replace('_', "");
    let value: u64 = value_str.parse().expect("Invalid decimal value");
    Literal::Decimal(value)
}

/// Parse a hex literal (e.g., "5'h1A") from the condition grammar.
///
/// This function is called by the lalrpop-generated parser.
pub(super) fn parse_hex_literal(s: &str) -> Literal {
    let lower = s.to_lowercase();
    let parts: Vec<&str> = lower.split('\'').collect();
    if parts.len() != 2 {
        panic!("Invalid hex literal: {}", s);
    }

    let value_str = parts[1].trim_start_matches('h').replace('_', "");
    let value = u64::from_str_radix(&value_str, 16).expect("Invalid hex value");
    Literal::Hexadecimal(value)
}

/// Find events where a condition is satisfied.
///
/// # Arguments
/// * `waveform` - The waveform to read from (must have signals loaded)
/// * `condition` - The condition to evaluate (e.g., "TOP.signal1 && TOP.signal2")
/// * `start_idx` - Starting time index (inclusive)
/// * `end_idx` - Ending time index (inclusive)
/// * `limit` - Maximum number of events to return. Use -1 for unlimited.
///
/// # Returns
/// A vector of formatted event strings, or an error if the operation fails.
pub fn find_conditional_events(
    waveform: &mut wellen::simple::Waveform,
    condition: &str,
    start_idx: usize,
    end_idx: usize,
    limit: isize,
) -> Result<Vec<String>, String> {
    // Get timescale before any mutable operations
    let timescale = waveform.hierarchy().timescale();

    // Parse condition
    let condition_ast = parse_condition(condition)?;

    // Extract all signal names from condition
    let signal_names = extract_signal_names(&condition_ast);

    // Find and load all signals
    let mut signal_cache = std::collections::HashMap::new();
    for signal_name in &signal_names {
        let signal_ref = {
            let hierarchy = waveform.hierarchy();
            find_signal_by_path(hierarchy, signal_name)
                .ok_or_else(|| format!("Signal not found: {}", signal_name))?
        };

        // Load signal
        let signal_refs = vec![signal_ref];
        waveform.load_signals(&signal_refs);

        signal_cache.insert(signal_name.clone(), signal_ref);
    }

    // Get time table after loading signals
    let time_table: Vec<wellen::Time> = waveform.time_table().to_vec();

    let mut events = Vec::new();

    // Scan through time indices
    let end = end_idx.min(time_table.len().saturating_sub(1));
    for (idx, &time_value) in time_table[start_idx..=end].iter().enumerate() {
        let time_idx = start_idx + idx;
        // Evaluate condition at this time index (0 = false, non-zero = true)
        if evaluate_condition(&condition_ast, waveform, &signal_cache, time_idx)? != 0 {
            let formatted_time = format_time(time_value, timescale.as_ref());

            // Build event description with signal values
            let mut signal_values = Vec::new();
            for signal_name in &signal_names {
                if let Some(signal_ref) = signal_cache.get(signal_name) {
                    if let Some(signal) = waveform.get_signal(*signal_ref) {
                        let time_table_idx: wellen::TimeTableIdx = time_idx
                            .try_into()
                            .map_err(|_| format!("Time index {} too large", time_idx))?;

                        if let Some(offset) = signal.get_offset(time_table_idx) {
                            let signal_value = signal.get_value_at(&offset, 0);
                            let value_str = format_signal_value(signal_value);
                            signal_values.push(format!("{} = {}", signal_name, value_str));
                        }
                    }
                }
            }

            events.push(format!(
                "Time index {} ({}): {}",
                time_idx,
                formatted_time,
                signal_values.join(", ")
            ));
        }

        // Check limit
        if limit >= 0 && events.len() >= limit as usize {
            break;
        }
    }

    Ok(events)
}

/// Extract all signal names from a condition AST.
fn extract_signal_names(condition: &Condition) -> Vec<String> {
    let mut names = Vec::new();
    extract_signal_names_recursive(condition, &mut names);
    names
}

fn extract_signal_names_recursive(condition: &Condition, names: &mut Vec<String>) {
    match condition {
        Condition::And(left, right) => {
            extract_signal_names_recursive(left, names);
            extract_signal_names_recursive(right, names);
        }
        Condition::Or(left, right) => {
            extract_signal_names_recursive(left, names);
            extract_signal_names_recursive(right, names);
        }
        Condition::Not(expr) => {
            extract_signal_names_recursive(expr, names);
        }
        Condition::Eq(left, right) => {
            extract_signal_names_recursive(left, names);
            extract_signal_names_recursive(right, names);
        }
        Condition::Neq(left, right) => {
            extract_signal_names_recursive(left, names);
            extract_signal_names_recursive(right, names);
        }
        Condition::Signal(path) => {
            if !names.contains(path) {
                names.push(path.clone());
            }
        }
        Condition::Literal(_) => {
            // Literals don't need to be loaded
        }
        Condition::Past(expr) => {
            extract_signal_names_recursive(expr, names);
        }
    }
}
