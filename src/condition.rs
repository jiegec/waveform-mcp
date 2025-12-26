//! Condition parsing and evaluation for conditional event search.

use wellen;

use super::{
    formatting::format_signal_value, formatting::format_time, hierarchy::find_signal_by_path,
};

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
/// This is a prototype implementation for common operations.
pub(super) fn parse_condition(condition: &str) -> Result<Condition, String> {
    let tokens = tokenize_condition(condition)?;
    let (ast, rest) = parse_or(&tokens)?;
    if !rest.is_empty() {
        return Err(format!("Unexpected tokens at end of condition: {:?}", rest));
    }
    Ok(ast)
}

/// Tokenize a condition string into tokens.
fn tokenize_condition(condition: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = condition.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        match c {
            '&' => {
                if i + 1 < chars.len() && chars[i + 1] == '&' {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    tokens.push("&&".to_string());
                    i += 1;
                } else {
                    current.push(c);
                }
            }
            '|' => {
                if i + 1 < chars.len() && chars[i + 1] == '|' {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    tokens.push("||".to_string());
                    i += 1;
                } else {
                    current.push(c);
                }
            }
            '=' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    tokens.push("==".to_string());
                    i += 1;
                } else {
                    return Err(format!("Unexpected single '=' at position {}", i));
                }
            }
            '!' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    tokens.push("!=".to_string());
                    i += 1;
                } else {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    tokens.push("!".to_string());
                }
            }
            '$' => {
                // Handle $past keyword
                if i + 4 < chars.len() {
                    let rest: String = chars[i + 1..i + 5].iter().collect();
                    if rest == "past" {
                        if !current.is_empty() {
                            tokens.push(current.clone());
                            current.clear();
                        }
                        tokens.push("$past".to_string());
                        i += 4;
                    } else {
                        current.push(c);
                    }
                } else {
                    current.push(c);
                }
            }
            '(' | ')' => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                tokens.push(c.to_string());
            }
            ' ' | '\t' => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(c);
            }
        }
        i += 1;
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

/// Parse an OR expression (lowest precedence).
fn parse_or(tokens: &[String]) -> Result<(Condition, &[String]), String> {
    let (mut left, mut rest) = parse_and(tokens)?;
    while !rest.is_empty() {
        if rest.first() == Some(&"||".to_string()) {
            rest = &rest[1..];
            let (right, new_rest) = parse_and(rest)?;
            left = Condition::Or(Box::new(left), Box::new(right));
            rest = new_rest;
        } else {
            break;
        }
    }
    Ok((left, rest))
}

/// Parse an AND expression.
fn parse_and(tokens: &[String]) -> Result<(Condition, &[String]), String> {
    let (mut left, mut rest) = parse_comparison(tokens)?;
    while !rest.is_empty() {
        if rest.first() == Some(&"&&".to_string()) {
            rest = &rest[1..];
            let (right, new_rest) = parse_comparison(rest)?;
            left = Condition::And(Box::new(left), Box::new(right));
            rest = new_rest;
        } else {
            break;
        }
    }
    Ok((left, rest))
}

/// Parse a comparison expression (==, !=).
fn parse_comparison(tokens: &[String]) -> Result<(Condition, &[String]), String> {
    let (mut left, mut rest) = parse_not(tokens)?;
    while !rest.is_empty() {
        if rest.first() == Some(&"==".to_string()) {
            rest = &rest[1..];
            let (right, new_rest) = parse_not(rest)?;
            left = Condition::Eq(Box::new(left), Box::new(right));
            rest = new_rest;
        } else if rest.first() == Some(&"!=".to_string()) {
            rest = &rest[1..];
            let (right, new_rest) = parse_not(rest)?;
            left = Condition::Neq(Box::new(left), Box::new(right));
            rest = new_rest;
        } else {
            break;
        }
    }
    Ok((left, rest))
}

/// Parse a NOT expression.
fn parse_not(tokens: &[String]) -> Result<(Condition, &[String]), String> {
    if tokens.first() == Some(&"!".to_string()) {
        let (expr, rest) = parse_not(&tokens[1..])?;
        Ok((Condition::Not(Box::new(expr)), rest))
    } else {
        parse_primary(tokens)
    }
}

/// Parse a primary expression (signal, literal, $past(), or parenthesized expression).
fn parse_primary(tokens: &[String]) -> Result<(Condition, &[String]), String> {
    if tokens.is_empty() {
        return Err("Unexpected end of tokens".to_string());
    }

    if tokens.first() == Some(&"(".to_string()) {
        let (expr, rest) = parse_or(&tokens[1..])?;
        if rest.first() != Some(&")".to_string()) {
            return Err("Expected closing parenthesis".to_string());
        }
        Ok((expr, &rest[1..]))
    } else if tokens.first() == Some(&"$past".to_string()) {
        // Parse $past(expression)
        if tokens.len() < 3 {
            return Err("Expected expression and closing parenthesis after $past".to_string());
        }
        if tokens[1] != "(" {
            return Err("Expected '(' after $past".to_string());
        }
        // Find matching closing parenthesis
        let mut depth = 1;
        let mut close_pos = 2;
        while close_pos < tokens.len() && depth > 0 {
            if tokens[close_pos] == "(" {
                depth += 1;
            } else if tokens[close_pos] == ")" {
                depth -= 1;
            }
            if depth == 0 {
                break;
            }
            close_pos += 1;
        }
        if depth != 0 {
            return Err("Unmatched parentheses in $past expression".to_string());
        }
        // Parse inner expression (tokens[2..close_pos])
        let (expr, rest) = parse_or(&tokens[2..close_pos])?;
        if !rest.is_empty() {
            return Err(format!("Unexpected tokens in $past(): {:?}", rest));
        }
        Ok((Condition::Past(Box::new(expr)), &tokens[close_pos + 1..]))
    } else {
        // Try to parse as literal first
        if let Ok(literal) = parse_verilog_literal(&tokens[0]) {
            return Ok((Condition::Literal(literal), &tokens[1..]));
        }

        // Must be a signal name
        let signal_name = tokens[0].clone();
        Ok((Condition::Signal(signal_name), &tokens[1..]))
    }
}

/// Parse a Verilog-style literal (e.g., 4'b0101, 3'd2, 5'h1A).
fn parse_verilog_literal(s: &str) -> Result<Literal, String> {
    let lower = s.to_lowercase();

    // Pattern: <width>'<base><value>
    // where base is b (binary), d (decimal), h (hex)
    let parts: Vec<&str> = lower.split('\'').collect();
    if parts.len() != 2 {
        return Err("Invalid literal format".to_string());
    }

    let _width: usize = parts[0]
        .parse()
        .map_err(|_| format!("Invalid width: {}", parts[0]))?;

    let value_part = parts[1];
    if value_part.is_empty() {
        return Err("Missing base specifier".to_string());
    }

    let base = value_part.chars().next().unwrap();
    let value_str = &value_part[1..];

    match base {
        'b' => {
            // Binary literal
            let mut bits = Vec::new();
            for c in value_str.chars() {
                match c {
                    '0' => bits.push(false),
                    '1' => bits.push(true),
                    'x' | 'z' => return Err("X and Z not supported in comparisons".to_string()),
                    '_' => {} // Skip underscores
                    _ => return Err(format!("Invalid binary digit: {}", c)),
                }
            }
            Ok(Literal::Binary(bits))
        }
        'd' => {
            // Decimal literal
            let value: u64 = value_str
                .chars()
                .filter(|c| *c != '_')
                .collect::<String>()
                .parse()
                .map_err(|_| format!("Invalid decimal value: {}", value_str))?;
            Ok(Literal::Decimal(value))
        }
        'h' => {
            // Hexadecimal literal
            let value_str: String = value_str.chars().filter(|c| *c != '_').collect();
            let value = u64::from_str_radix(&value_str, 16)
                .map_err(|_| format!("Invalid hex value: {}", value_str))?;
            Ok(Literal::Hexadecimal(value))
        }
        _ => Err(format!("Unknown base specifier: {}", base)),
    }
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
/// `true` if condition evaluates to true, `false` otherwise.
fn evaluate_condition(
    condition: &Condition,
    waveform: &mut wellen::simple::Waveform,
    signal_cache: &std::collections::HashMap<String, wellen::SignalRef>,
    time_idx: usize,
) -> Result<bool, String> {
    match condition {
        Condition::And(left, right) => {
            let left_val = evaluate_condition(left, waveform, signal_cache, time_idx)?;
            let right_val = evaluate_condition(right, waveform, signal_cache, time_idx)?;
            Ok(left_val && right_val)
        }
        Condition::Or(left, right) => {
            let left_val = evaluate_condition(left, waveform, signal_cache, time_idx)?;
            let right_val = evaluate_condition(right, waveform, signal_cache, time_idx)?;
            Ok(left_val || right_val)
        }
        Condition::Not(expr) => {
            let val = evaluate_condition(expr, waveform, signal_cache, time_idx)?;
            Ok(!val)
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

            // Evaluate as boolean: true if non-zero (for numeric values) or "1"/"true" (for string values)
            Ok(is_signal_true(signal_value))
        }
        Condition::Eq(left, right) => {
            let left_val = evaluate_condition_to_value(left, waveform, signal_cache, time_idx)?;
            let right_val = evaluate_condition_to_value(right, waveform, signal_cache, time_idx)?;
            Ok(left_val == right_val)
        }
        Condition::Neq(left, right) => {
            let left_val = evaluate_condition_to_value(left, waveform, signal_cache, time_idx)?;
            let right_val = evaluate_condition_to_value(right, waveform, signal_cache, time_idx)?;
            Ok(left_val != right_val)
        }
        Condition::Literal(_) => Err("Literals must be used in comparisons".to_string()),
        Condition::Past(expr) => {
            // If at time 0, there's no previous value
            if time_idx == 0 {
                return Ok(false); // Skip evaluation when there's no past
            }
            // Evaluate expression at previous time index
            evaluate_condition(expr, waveform, signal_cache, time_idx - 1)
        }
    }
}

/// Evaluate a condition to a signal value (for comparison purposes).
fn evaluate_condition_to_value(
    condition: &Condition,
    waveform: &mut wellen::simple::Waveform,
    signal_cache: &std::collections::HashMap<String, wellen::SignalRef>,
    time_idx: usize,
) -> Result<i64, String> {
    match condition {
        Condition::Signal(path) => {
            let signal_ref = signal_cache
                .get(path)
                .ok_or_else(|| format!("Signal not found in cache: {}", path))?;

            let signal = waveform
                .get_signal(*signal_ref)
                .ok_or_else(|| format!("Signal not found in waveform: {}", path))?;

            let time_table_idx: wellen::TimeTableIdx = time_idx
                .try_into()
                .map_err(|_| format!("Time index {} too large", time_idx))?;

            let offset = signal
                .get_offset(time_table_idx)
                .ok_or_else(|| format!("No data for signal {} at time index {}", path, time_idx))?;

            let signal_value = signal.get_value_at(&offset, 0);
            signal_value_to_i64(signal_value)
        }
        Condition::Literal(literal) => literal_to_i64(literal),
        Condition::Past(expr) => {
            // If at time 0, there's no previous value
            if time_idx == 0 {
                return Err("No previous value at time index 0".to_string());
            }
            // Evaluate expression at previous time index
            evaluate_condition_to_value(expr, waveform, signal_cache, time_idx - 1)
        }
        _ => Err("Expected signal or literal in comparison".to_string()),
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
        wellen::SignalValue::String(s) => s
            .parse()
            .map_err(|_| format!("Cannot convert string '{}' to integer", s)),
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

/// Check if a signal value is considered "true".
///
/// For binary/four-value/nine-value signals: true if any bit is set
/// For string signals: true if value is "1" or "true"
/// For real signals: true if value is non-zero
fn is_signal_true(signal_value: wellen::SignalValue) -> bool {
    match signal_value {
        wellen::SignalValue::Binary(data, _) => data.iter().any(|&b| b != 0),
        wellen::SignalValue::FourValue(data, _) => data.iter().any(|&b| b != 0),
        wellen::SignalValue::NineValue(data, _) => data.iter().any(|&b| b != 0),
        wellen::SignalValue::String(s) => s == "1" || s.eq_ignore_ascii_case("true"),
        wellen::SignalValue::Real(r) => r != 0.0,
        wellen::SignalValue::Event => false,
    }
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
        // Evaluate condition at this time index
        if evaluate_condition(&condition_ast, waveform, &signal_cache, time_idx)? {
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
