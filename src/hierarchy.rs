//! Hierarchy navigation and signal finding utilities.

use wellen;

/// Find a signal by its hierarchical path in the waveform hierarchy.
///
/// # Arguments
/// * `hierarchy` - The waveform hierarchy to search
/// * `path` - The hierarchical path to signal (e.g., "top.module.signal")
///
/// # Returns
/// `Some(SignalRef)` if signal is found, `None` otherwise.
pub fn find_signal_by_path(hierarchy: &wellen::Hierarchy, path: &str) -> Option<wellen::SignalRef> {
    for var in hierarchy.iter_vars() {
        let signal_path = var.full_name(hierarchy);
        if signal_path == path {
            return Some(var.signal_ref());
        }
    }
    None
}

/// Find a scope by its hierarchical path in waveform hierarchy.
///
/// # Arguments
/// * `hierarchy` - The waveform hierarchy to search
/// * `path` - The hierarchical path to scope (e.g., "top.module")
///
/// # Returns
/// `Some(ScopeRef)` if scope is found, `None` otherwise.
pub fn find_scope_by_path(hierarchy: &wellen::Hierarchy, path: &str) -> Option<wellen::ScopeRef> {
    for scope_ref in hierarchy.scopes() {
        let scope = &hierarchy[scope_ref];
        let scope_path = scope.full_name(hierarchy);
        if scope_path == path {
            return Some(scope_ref);
        }
        // Recursively check child scopes
        if let Some(child_ref) = find_scope_by_path_recursive(hierarchy, scope_ref, path) {
            return Some(child_ref);
        }
    }
    None
}

fn find_scope_by_path_recursive(
    hierarchy: &wellen::Hierarchy,
    parent_ref: wellen::ScopeRef,
    target_path: &str,
) -> Option<wellen::ScopeRef> {
    let parent = &hierarchy[parent_ref];
    for child_ref in parent.scopes(hierarchy) {
        let child = &hierarchy[child_ref];
        let child_path = child.full_name(hierarchy);
        if child_path == target_path {
            return Some(child_ref);
        }
        // Recursively check child scopes
        if let Some(found) = find_scope_by_path_recursive(hierarchy, child_ref, target_path) {
            return Some(found);
        }
    }
    None
}

/// Collect signals from a scope and optionally its children recursively.
pub(super) fn collect_signals_from_scope(
    hierarchy: &wellen::Hierarchy,
    scope_ref: wellen::ScopeRef,
    recursive: bool,
    name_pattern: Option<&str>,
) -> Vec<String> {
    let mut signals = Vec::new();
    let scope = &hierarchy[scope_ref];

    // Collect variables directly in this scope
    for var_ref in scope.vars(hierarchy) {
        let var = &hierarchy[var_ref];
        let path = var.full_name(hierarchy);

        // Apply name pattern filter if provided
        if let Some(pattern) = name_pattern {
            let pattern_lower = pattern.to_lowercase();
            let path_lower = path.to_lowercase();
            if !path_lower.contains(&pattern_lower) {
                continue;
            }
        }

        signals.push(path);
    }

    // If recursive, also collect from child scopes
    if recursive {
        for child_ref in scope.scopes(hierarchy) {
            signals.extend(collect_signals_from_scope(
                hierarchy,
                child_ref,
                true,
                name_pattern,
            ));
        }
    }

    signals
}
