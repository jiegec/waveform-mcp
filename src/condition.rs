//! Condition parsing and evaluation for conditional event search.

use super::{
    formatting::format_signal_value, formatting::format_time, hierarchy::find_var_by_path,
};
use lalrpop_util::lalrpop_mod;
use num_bigint::BigUint;
use num_traits::Zero;
use std::ops::{BitAnd, BitOr, BitXor};
use wellen;

// Import generated parser
lalrpop_mod!(condition);

/// Literal value for signal comparison.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Literal {
    Binary(Vec<bool>, u32), // bits, bit width
    Decimal(u64, u32),      // value, bit width
    Hexadecimal(u64, u32),  // value, bit width
}

/// Condition for finding events based on signal values.
#[derive(Debug, Clone)]
pub(super) enum Condition {
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Not(Box<Condition>),
    BitwiseNot(Box<Condition>),
    Signal(String),
    BitExtract(String, Option<u32>, Option<u32>), // signal, msb (optional), lsb (optional)
    Eq(Box<Condition>, Box<Condition>),
    Neq(Box<Condition>, Box<Condition>),
    BitwiseAnd(Box<Condition>, Box<Condition>),
    BitwiseOr(Box<Condition>, Box<Condition>),
    BitwiseXor(Box<Condition>, Box<Condition>),
    Literal(Literal),
    Past(Box<Condition>),
}

/// Parse a simple condition string into a Condition AST.
///
/// Supports:
/// - Signal paths (e.g., "TOP.signal")
/// - Bit extraction: `signal[bit]` for single bit, `signal[msb:lsb]` for range
/// - `&&` for logical AND
/// - `||` for logical OR
/// - `!` for logical NOT
/// - `~` for bitwise NOT
/// - `&` for bitwise AND
/// - `|` for bitwise OR
/// - `^` for bitwise XOR
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
/// * `signal_cache` - Cache of variable references
/// * `time_idx` - The time index to evaluate at
///
/// # Returns
/// A BigUint value where 0 = false and any non-zero value = true.
fn evaluate_condition(
    condition: &Condition,
    waveform: &mut wellen::simple::Waveform,
    signal_cache: &std::collections::HashMap<String, wellen::VarRef>,
    time_idx: usize,
) -> Result<BigUint, String> {
    let (value, _width) =
        evaluate_condition_with_width(condition, waveform, signal_cache, time_idx)?;
    Ok(value)
}

/// Evaluate a condition at a specific time index, returning both value and bit width.
///
/// # Returns
/// A tuple of (value, bit_width) where bit_width is the bit width of the value.
fn evaluate_condition_with_width(
    condition: &Condition,
    waveform: &mut wellen::simple::Waveform,
    signal_cache: &std::collections::HashMap<String, wellen::VarRef>,
    time_idx: usize,
) -> Result<(BigUint, u32), String> {
    match condition {
        Condition::And(left, right) => {
            let left_val = evaluate_condition(left, waveform, signal_cache, time_idx)?;
            let right_val = evaluate_condition(right, waveform, signal_cache, time_idx)?;
            Ok((
                if !left_val.is_zero() && !right_val.is_zero() {
                    BigUint::from(1u32)
                } else {
                    BigUint::from(0u32)
                },
                1, // Logical operations return 1-bit result
            ))
        }
        Condition::Or(left, right) => {
            let left_val = evaluate_condition(left, waveform, signal_cache, time_idx)?;
            let right_val = evaluate_condition(right, waveform, signal_cache, time_idx)?;
            Ok((
                if !left_val.is_zero() || !right_val.is_zero() {
                    BigUint::from(1u32)
                } else {
                    BigUint::from(0u32)
                },
                1, // Logical operations return 1-bit result
            ))
        }
        Condition::BitwiseAnd(left, right) => {
            let (left_val, left_width) =
                evaluate_condition_with_width(left, waveform, signal_cache, time_idx)?;
            let (right_val, right_width) =
                evaluate_condition_with_width(right, waveform, signal_cache, time_idx)?;
            let width = left_width.max(right_width);
            Ok((left_val.bitand(right_val), width))
        }
        Condition::BitwiseOr(left, right) => {
            let (left_val, left_width) =
                evaluate_condition_with_width(left, waveform, signal_cache, time_idx)?;
            let (right_val, right_width) =
                evaluate_condition_with_width(right, waveform, signal_cache, time_idx)?;
            let width = left_width.max(right_width);
            Ok((left_val.bitor(right_val), width))
        }
        Condition::BitwiseXor(left, right) => {
            let (left_val, left_width) =
                evaluate_condition_with_width(left, waveform, signal_cache, time_idx)?;
            let (right_val, right_width) =
                evaluate_condition_with_width(right, waveform, signal_cache, time_idx)?;
            let width = left_width.max(right_width);
            Ok((left_val.bitxor(right_val), width))
        }
        Condition::Not(expr) => {
            let val = evaluate_condition(expr, waveform, signal_cache, time_idx)?;
            Ok((
                if !val.is_zero() {
                    BigUint::from(0u32)
                } else {
                    BigUint::from(1u32)
                },
                1, // Logical NOT returns 1-bit result
            ))
        }
        Condition::BitwiseNot(expr) => {
            let (val, width) =
                evaluate_condition_with_width(expr, waveform, signal_cache, time_idx)?;
            // Create a mask with all bits set for the given width
            let mask = (BigUint::from(1u32) << width) - BigUint::from(1u32);
            // Bitwise NOT is value XOR mask
            Ok((val.bitxor(mask), width))
        }
        Condition::Signal(path) => {
            // Get var ref from cache
            let var_ref = signal_cache
                .get(path)
                .ok_or_else(|| format!("Signal not found in cache: {}", path))?;

            // Get signal width from hierarchy
            let hierarchy = waveform.hierarchy();
            let width = hierarchy[*var_ref]
                .length()
                .ok_or_else(|| format!("Signal {} has no width (string/real type)", path))?;

            // Get signal from waveform
            let signal = waveform
                .get_signal(hierarchy[*var_ref].signal_ref())
                .ok_or_else(|| format!("Signal not found in waveform: {}", path))?;

            // Get value at time index
            let time_table_idx: wellen::TimeTableIdx = time_idx
                .try_into()
                .map_err(|_| format!("Time index {} too large", time_idx))?;

            let offset = signal
                .get_offset(time_table_idx)
                .ok_or_else(|| format!("No data for signal {} at time index {}", path, time_idx))?;

            let signal_value = signal.get_value_at(&offset, 0);
            let value = signal_value_to_biguint(signal_value)?;
            Ok((value, width))
        }
        Condition::BitExtract(path, msb, lsb) => {
            // Get var ref from cache
            let var_ref = signal_cache
                .get(path)
                .ok_or_else(|| format!("Signal not found in cache: {}", path))?;

            // Get signal width from hierarchy
            let hierarchy = waveform.hierarchy();
            let full_width = hierarchy[*var_ref]
                .length()
                .ok_or_else(|| format!("Signal {} has no width (string/real type)", path))?;

            // Get signal from waveform
            let signal = waveform
                .get_signal(hierarchy[*var_ref].signal_ref())
                .ok_or_else(|| format!("Signal not found in waveform: {}", path))?;

            // Get value at time index
            let time_table_idx: wellen::TimeTableIdx = time_idx
                .try_into()
                .map_err(|_| format!("Time index {} too large", time_idx))?;

            let offset = signal
                .get_offset(time_table_idx)
                .ok_or_else(|| format!("No data for signal {} at time index {}", path, time_idx))?;

            let signal_value = signal.get_value_at(&offset, 0);
            let full_value = signal_value_to_biguint(signal_value)?;

            // Extract the specified bits and determine the result width
            let (result, width) = match (msb, lsb) {
                (Some(msb), Some(lsb)) => {
                    if msb < lsb {
                        return Err(format!(
                            "Invalid bit range [{}:{}] - msb must be >= lsb",
                            msb, lsb
                        ));
                    }
                    // Extract bits [msb:lsb] by:
                    // 1. Shift right by lsb positions
                    // 2. Create a mask with (msb - lsb + 1) bits set
                    let num_bits = msb - lsb + 1;
                    let shifted = full_value >> lsb;
                    let mask = (BigUint::from(1u32) << num_bits) - BigUint::from(1u32);
                    (shifted & mask, num_bits)
                }
                _ => (full_value, full_width), // No bit extraction needed
            };
            Ok((result, width))
        }
        Condition::Eq(left, right) => {
            let left_val = evaluate_condition(left, waveform, signal_cache, time_idx)?;
            let right_val = evaluate_condition(right, waveform, signal_cache, time_idx)?;
            Ok((
                if left_val == right_val {
                    BigUint::from(1u32)
                } else {
                    BigUint::from(0u32)
                },
                1, // Comparison operations return 1-bit result
            ))
        }
        Condition::Neq(left, right) => {
            let left_val = evaluate_condition(left, waveform, signal_cache, time_idx)?;
            let right_val = evaluate_condition(right, waveform, signal_cache, time_idx)?;
            Ok((
                if left_val != right_val {
                    BigUint::from(1u32)
                } else {
                    BigUint::from(0u32)
                },
                1, // Comparison operations return 1-bit result
            ))
        }
        Condition::Literal(literal) => literal_to_biguint(literal),
        Condition::Past(expr) => {
            // If at time 0, there's no previous value
            if time_idx == 0 {
                return Ok((BigUint::from(0u32), 1)); // Return false (0) when there's no past
            }
            // Evaluate expression at previous time index
            evaluate_condition_with_width(expr, waveform, signal_cache, time_idx - 1)
        }
    }
}

/// Convert a signal value to BigUint for comparison.
fn signal_value_to_biguint(signal_value: wellen::SignalValue) -> Result<BigUint, String> {
    match signal_value {
        wellen::SignalValue::Binary(data, _) => {
            // wellen stores the value as a Vec<u32> where each element is a chunk of bits
            // We need to combine these chunks into a single BigUint
            let mut value = BigUint::from(0u32);
            for (i, &chunk) in data.iter().enumerate() {
                if chunk != 0 {
                    // Each chunk is at position i * 32 bits
                    let chunk_value = BigUint::from(chunk);
                    value |= chunk_value << (i * 32);
                }
            }
            Ok(value)
        }
        wellen::SignalValue::FourValue(data, _) => {
            // Same as Binary for FourValue
            let mut value = BigUint::from(0u32);
            for (i, &chunk) in data.iter().enumerate() {
                if chunk != 0 {
                    let chunk_value = BigUint::from(chunk);
                    value |= chunk_value << (i * 32);
                }
            }
            Ok(value)
        }
        wellen::SignalValue::NineValue(data, _) => {
            // Same as Binary for NineValue
            let mut value = BigUint::from(0u32);
            for (i, &chunk) in data.iter().enumerate() {
                if chunk != 0 {
                    let chunk_value = BigUint::from(chunk);
                    value |= chunk_value << (i * 32);
                }
            }
            Ok(value)
        }
        wellen::SignalValue::String(s) => {
            // Handle special string values for boolean context
            if s == "1" || s.eq_ignore_ascii_case("true") {
                Ok(BigUint::from(1u32))
            } else if s == "0" || s.eq_ignore_ascii_case("false") {
                Ok(BigUint::from(0u32))
            } else {
                s.parse::<u64>()
                    .map(BigUint::from)
                    .map_err(|_| format!("Cannot convert string '{}' to integer", s))
            }
        }
        wellen::SignalValue::Real(r) => Ok(BigUint::from(r as u64)),
        wellen::SignalValue::Event => Err("Event signal cannot be compared".to_string()),
    }
}

/// Convert a literal to BigUint for comparison.
/// Also returns the bit width.
fn literal_to_biguint(literal: &Literal) -> Result<(BigUint, u32), String> {
    match literal {
        Literal::Binary(bits, width) => {
            let mut value = BigUint::from(0u32);
            for (i, &bit) in bits.iter().rev().enumerate() {
                if bit {
                    value.set_bit(i as u64, true);
                }
            }
            Ok((value, *width))
        }
        Literal::Decimal(v, width) => Ok((BigUint::from(*v), *width)),
        Literal::Hexadecimal(v, width) => Ok((BigUint::from(*v), *width)),
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

    let width: u32 = parts[0].parse().expect("Invalid bit width");
    let value_str = parts[1].trim_start_matches('b').replace('_', "");
    let mut bits = Vec::new();
    for c in value_str.chars() {
        match c {
            '0' => bits.push(false),
            '1' => bits.push(true),
            _ => panic!("Invalid binary digit: {}", c),
        }
    }
    Literal::Binary(bits, width)
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

    let width: u32 = parts[0].parse().expect("Invalid bit width");
    let value_str = parts[1].trim_start_matches('d').replace('_', "");
    let value: u64 = value_str.parse().expect("Invalid decimal value");
    Literal::Decimal(value, width)
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

    let width: u32 = parts[0].parse().expect("Invalid bit width");
    let value_str = parts[1].trim_start_matches('h').replace('_', "");
    let value = u64::from_str_radix(&value_str, 16).expect("Invalid hex value");
    Literal::Hexadecimal(value, width)
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
    let hierarchy = waveform.hierarchy();

    // Collect all signal_refs first
    let mut signal_refs = Vec::new();
    for signal_name in &signal_names {
        let var_ref = find_var_by_path(hierarchy, signal_name)
            .ok_or_else(|| format!("Signal not found: {}", signal_name))?;
        let signal_ref = hierarchy[var_ref].signal_ref();
        signal_cache.insert(signal_name.clone(), var_ref);
        signal_refs.push(signal_ref);
    }

    // Load all signals at once
    waveform.load_signals(&signal_refs);

    // Get time table after loading signals
    let time_table: Vec<wellen::Time> = waveform.time_table().to_vec();

    let mut events = Vec::new();

    // Scan through time indices
    let end = end_idx.min(time_table.len().saturating_sub(1));
    for (idx, &time_value) in time_table[start_idx..=end].iter().enumerate() {
        let time_idx = start_idx + idx;
        // Evaluate condition at this time index (0 = false, non-zero = true)
        if !evaluate_condition(&condition_ast, waveform, &signal_cache, time_idx)?.is_zero() {
            let formatted_time = format_time(time_value, timescale.as_ref());

            // Build event description with signal values
            let mut signal_values = Vec::new();
            for signal_name in &signal_names {
                if let Some(var_ref) = signal_cache.get(signal_name) {
                    let hierarchy = waveform.hierarchy();
                    let signal_ref = hierarchy[*var_ref].signal_ref();
                    if let Some(signal) = waveform.get_signal(signal_ref) {
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
        Condition::BitwiseNot(expr) => {
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
        Condition::BitwiseAnd(left, right) => {
            extract_signal_names_recursive(left, names);
            extract_signal_names_recursive(right, names);
        }
        Condition::BitwiseOr(left, right) => {
            extract_signal_names_recursive(left, names);
            extract_signal_names_recursive(right, names);
        }
        Condition::BitwiseXor(left, right) => {
            extract_signal_names_recursive(left, names);
            extract_signal_names_recursive(right, names);
        }
        Condition::Signal(path) => {
            if !names.contains(path) {
                names.push(path.clone());
            }
        }
        Condition::BitExtract(path, _, _) => {
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
