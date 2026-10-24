//! Signal reading and querying utilities.

use wellen;

use super::{
    formatting::format_signal_value, formatting::format_time, hierarchy::all_signal_paths,
    hierarchy::collect_signals_from_scope, hierarchy::find_scope_by_path,
    hierarchy::signal_not_found_message, matcher::MatchMode, matcher::SignalMatcher,
    matcher::glob_hint, matcher::pattern_seed, suggest::did_you_mean,
};

/// List signals in a waveform hierarchy with optional filtering.
///
/// # Arguments
/// * `hierarchy` - The waveform hierarchy to search
/// * `matcher` - Compiled path patterns. Multiple patterns are OR-ed, and an empty
///   matcher matches every signal. Patterns are matched against the full signal path
///   (e.g. `tb.dut.abc_bank`).
/// * `hierarchy_prefix` - Optional hierarchy path prefix to filter signals (must match a scope)
/// * `recursive` - If true, list all signals recursively; if false, only list signals at specified level
/// * `limit` - Optional maximum number of signals to return. Use -1 for unlimited.
///
/// # Returns
/// A vector of signal paths.
pub fn list_signals(
    hierarchy: &wellen::Hierarchy,
    matcher: &SignalMatcher,
    hierarchy_prefix: Option<&str>,
    recursive: bool,
    limit: Option<isize>,
) -> Vec<String> {
    let mut signals = Vec::new();

    if let Some(prefix) = hierarchy_prefix {
        // Find scope by path
        if let Some(scope_ref) = find_scope_by_path(hierarchy, prefix) {
            // Collect signals from this scope (and children if recursive)
            signals = collect_signals_from_scope(hierarchy, scope_ref, recursive, matcher);
        }
        // If scope not found, return empty signals
    } else {
        // No hierarchy prefix - start from top-level scopes
        for scope_ref in hierarchy.scopes() {
            signals.extend(collect_signals_from_scope(
                hierarchy, scope_ref, recursive, matcher,
            ));
        }
    }

    // Apply limit if provided and not -1 (unlimited)
    if let Some(limit) = limit
        && limit >= 0
    {
        signals.truncate(limit as usize);
    }

    signals
}

/// Build the user-facing text for a `list_signals` query.
///
/// When nothing matches, the patterns are turned into fuzzy-search seeds and the closest signal
/// paths are suggested. That catches abbreviations (`abc_bank` vs `tb.dut.abc_bank`), typos, and
/// patterns that were written with glob wildcards but are used in substring mode.
///
/// # Arguments
/// * `hierarchy` - The waveform hierarchy
/// * `patterns` - The `(pattern, match mode)` pairs the user asked for
/// * `signals` - The signals that actually matched
pub fn format_signal_query_result(
    hierarchy: &wellen::Hierarchy,
    patterns: &[(String, MatchMode)],
    signals: &[String],
) -> String {
    if !signals.is_empty() {
        return format!("Found {} signals:\n{}", signals.len(), signals.join("\n"));
    }

    let mut output = String::from("No signals matched.");

    let suggestions = fuzzy_signal_suggestions(hierarchy, patterns);
    if !suggestions.is_empty() {
        let quoted: Vec<String> = suggestions
            .iter()
            .map(|suggestion| format!("'{}'", suggestion))
            .collect();
        output.push_str(&format!("\nDid you mean: {}?", quoted.join(", ")));
    }

    if let Some(hint) = glob_hint(patterns) {
        output.push_str("\n\n");
        output.push_str(&hint);
    }

    output
}

/// Fuzzy-match the query patterns against all signal paths, best matches first.
fn fuzzy_signal_suggestions(
    hierarchy: &wellen::Hierarchy,
    patterns: &[(String, MatchMode)],
) -> Vec<String> {
    let mut seeds: Vec<String> = Vec::new();
    for (pattern, mode) in patterns {
        let seed = pattern_seed(pattern, *mode);
        if !seed.is_empty() && !seeds.contains(&seed) {
            seeds.push(seed);
        }
    }

    if seeds.is_empty() {
        return Vec::new();
    }

    let paths = all_signal_paths(hierarchy);
    let mut suggestions: Vec<String> = Vec::new();

    for seed in seeds {
        for suggestion in did_you_mean(&seed, &paths) {
            if !suggestions.contains(&suggestion) {
                suggestions.push(suggestion);
            }
            if suggestions.len() >= 5 {
                return suggestions;
            }
        }
    }

    suggestions
}

/// Read signal values at specific time indices.
///
/// # Arguments
/// * `waveform` - The waveform to read from (must have signal loaded)
/// * `signal_ref` - The signal reference to read
/// * `time_indices` - The time indices to read values at
///
/// # Returns
/// A vector of formatted signal value strings, or an error if the operation fails.
pub fn read_signal_values(
    waveform: &wellen::simple::Waveform,
    signal_ref: wellen::SignalRef,
    time_indices: &[usize],
) -> Result<Vec<String>, String> {
    let time_table = waveform.time_table();
    let timescale = waveform.hierarchy().timescale();

    let signal = waveform
        .get_signal(signal_ref)
        .ok_or("Signal not found after loading")?;

    let mut results = Vec::new();

    for time_idx in time_indices {
        if *time_idx >= time_table.len() {
            results.push(format!(
                "Time index {} out of range (max: {})",
                time_idx,
                time_table.len() - 1
            ));
            continue;
        }

        let time_value = time_table[*time_idx];
        let formatted_time = format_time(time_value, timescale.as_ref());

        let time_table_idx: wellen::TimeTableIdx = (*time_idx)
            .try_into()
            .map_err(|_| format!("Time index {} exceeds maximum value", time_idx))?;

        let offset = signal
            .get_offset(time_table_idx)
            .ok_or("No data available for this time index")?;

        let signal_value = signal.get_value_at(&offset, 0);
        let value_str = format_signal_value(signal_value);

        results.push(format!(
            "Time index {} ({}): {}",
            time_idx, formatted_time, value_str
        ));
    }

    Ok(results)
}

/// Get metadata about a signal.
///
/// # Arguments
/// * `hierarchy` - The waveform hierarchy
/// * `signal_path` - The hierarchical path to the signal
///
/// # Returns
/// A formatted string containing signal metadata, or an error if the signal is not found.
pub fn get_signal_metadata(
    hierarchy: &wellen::Hierarchy,
    signal_path: &str,
) -> Result<String, String> {
    // Find VarRef from path
    let parts: Vec<&str> = signal_path.split('.').collect();
    let var_ref = if parts.len() > 1 {
        let path = &parts[..parts.len() - 1];
        let name = parts[parts.len() - 1];
        hierarchy
            .lookup_var(path, name)
            .ok_or_else(|| signal_not_found_message(hierarchy, signal_path))?
    } else {
        hierarchy
            .lookup_var(&[], signal_path)
            .ok_or_else(|| signal_not_found_message(hierarchy, signal_path))?
    };

    let var = &hierarchy[var_ref];

    let width_info = match var.length(hierarchy) {
        Some(len) => format!("{} bits", len),
        None => "variable length (string/real)".to_string(),
    };

    let index_info = match var.index() {
        Some(idx) => format!("[{}:{}]", idx.msb(), idx.lsb()),
        None => "N/A".to_string(),
    };

    let info = format!(
        "Signal: {}\nType: {:?}\nWidth: {}\nIndex: {}",
        signal_path,
        var.var_type(),
        width_info,
        index_info
    );

    Ok(info)
}

/// Validate a time index range against the waveform's time table.
///
/// Shared by `find_signal_events` and `find_conditional_events` so that out-of-range and
/// inverted ranges are reported consistently instead of indexing out of bounds.
pub(crate) fn validate_time_range(
    time_table_len: usize,
    start_idx: usize,
    end_idx: usize,
) -> Result<(), String> {
    let max_idx = time_table_len.saturating_sub(1);

    if start_idx > max_idx {
        return Err(format!(
            "start time index {} out of range (max: {})",
            start_idx, max_idx
        ));
    }

    if start_idx > end_idx {
        return Err(format!(
            "start time index {} is after end time index {}",
            start_idx, end_idx
        ));
    }

    Ok(())
}

/// Find events (changes) of a signal within a time range.
///
/// # Arguments
/// * `waveform` - The waveform to read from (must have signal loaded)
/// * `signal_ref` - The signal reference to analyze
/// * `start_idx` - Starting time index (inclusive)
/// * `end_idx` - Ending time index (inclusive)
/// * `limit` - Maximum number of events to return. Use -1 for unlimited.
///
/// # Returns
/// A vector of formatted event strings, or an error if the operation fails.
pub fn find_signal_events(
    waveform: &wellen::simple::Waveform,
    signal_ref: wellen::SignalRef,
    start_idx: usize,
    end_idx: usize,
    limit: isize,
) -> Result<Vec<String>, String> {
    let time_table = waveform.time_table();
    let timescale = waveform.hierarchy().timescale();

    validate_time_range(time_table.len(), start_idx, end_idx)?;

    let signal = waveform
        .get_signal(signal_ref)
        .ok_or("Signal not found after loading")?;

    let mut events = Vec::new();

    for (time_idx, signal_value) in signal.iter_changes() {
        let time_idx = time_idx as usize;

        // Check if within time range
        if time_idx < start_idx || time_idx > end_idx {
            continue;
        }

        // Check limit (unless unlimited with -1)
        if limit >= 0 && events.len() >= limit as usize {
            break;
        }

        let time_value = time_table[time_idx];
        let formatted_time = format_time(time_value, timescale.as_ref());
        let value_str = format_signal_value(signal_value);

        events.push(format!(
            "Time index {} ({}): {}",
            time_idx, formatted_time, value_str
        ));
    }

    Ok(events)
}
