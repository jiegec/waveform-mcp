//! Hierarchy navigation and signal finding utilities.

use wellen;

struct HierarchyRenderer {
    lines: Vec<String>,
    limit: Option<usize>,
    truncated: bool,
}

impl HierarchyRenderer {
    fn new(limit: Option<isize>) -> Self {
        Self {
            lines: Vec::new(),
            limit: limit.and_then(|value| (value >= 0).then_some(value as usize)),
            truncated: false,
        }
    }

    fn is_full(&self) -> bool {
        self.limit.is_some_and(|limit| self.lines.len() >= limit)
    }

    fn push_line(&mut self, line: String) -> bool {
        if self.is_full() {
            self.truncated = true;
            return false;
        }

        self.lines.push(line);
        true
    }

    fn finish(mut self) -> Vec<String> {
        if self.truncated
            && let Some(limit) = self.limit
        {
            self.lines
                .push(format!("... truncated after {} items", limit));
        }

        self.lines
    }
}

/// Find a variable (VarRef) by its hierarchical path in waveform hierarchy.
///
/// # Arguments
/// * `hierarchy` - The waveform hierarchy to search
/// * `path` - The hierarchical path to signal (e.g., "top.module.signal")
///
/// # Returns
/// `Some(VarRef)` if signal is found, `None` otherwise.
pub fn find_var_by_path(hierarchy: &wellen::Hierarchy, path: &str) -> Option<wellen::VarRef> {
    match hierarchy.lookup_item_by_name(path)? {
        wellen::ItemRef::Var(v) => Some(v),
        wellen::ItemRef::Scope(_) => None,
    }
}

/// Find a signal by its hierarchical path in the waveform hierarchy.
///
/// # Arguments
/// * `hierarchy` - The waveform hierarchy to search
/// * `path` - The hierarchical path to signal (e.g., "top.module.signal")
///
/// # Returns
/// `Some(SignalRef)` if signal is found, `None` otherwise.
pub fn find_signal_by_path(hierarchy: &wellen::Hierarchy, path: &str) -> Option<wellen::SignalRef> {
    find_var_by_path(hierarchy, path).map(|var_ref| hierarchy[var_ref].signal_ref())
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
    match hierarchy.lookup_item_by_name(path)? {
        wellen::ItemRef::Scope(s) => Some(s),
        wellen::ItemRef::Var(_) => None,
    }
}

/// Read the waveform module hierarchy as an indented tree.
///
/// # Arguments
/// * `hierarchy` - The waveform hierarchy to read
/// * `scope_path` - Optional root scope path to start from
/// * `recursive` - If true, include all descendant modules; if false, include only one level below the selected scope
/// * `limit` - Optional maximum number of modules to return. Use -1 for unlimited.
///
/// # Returns
/// A vector of formatted module hierarchy lines, or an error if the scope path is invalid.
pub fn read_hierarchy(
    hierarchy: &wellen::Hierarchy,
    scope_path: Option<&str>,
    recursive: bool,
    limit: Option<isize>,
) -> Result<Vec<String>, String> {
    let mut renderer = HierarchyRenderer::new(limit);

    match scope_path {
        Some(path) => {
            let scope_ref = find_scope_by_path(hierarchy, path)
                .ok_or_else(|| format!("Scope not found: {}", path))?;
            let child_depth = if recursive { usize::MAX } else { 1 };
            render_scope(hierarchy, scope_ref, 0, child_depth, true, &mut renderer);
        }
        None => {
            let child_depth = if recursive { usize::MAX } else { 0 };
            for item in hierarchy.items() {
                if renderer.is_full() {
                    renderer.truncated = true;
                    break;
                }

                render_item(hierarchy, item, 0, child_depth, true, &mut renderer);
            }
        }
    }

    Ok(renderer.finish())
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

fn render_item(
    hierarchy: &wellen::Hierarchy,
    item: wellen::ItemRef,
    depth: usize,
    child_depth: usize,
    show_full_name: bool,
    renderer: &mut HierarchyRenderer,
) {
    if let wellen::ItemRef::Scope(scope_ref) = item {
        render_scope(
            hierarchy,
            scope_ref,
            depth,
            child_depth,
            show_full_name,
            renderer,
        );
    }
}

fn render_scope(
    hierarchy: &wellen::Hierarchy,
    scope_ref: wellen::ScopeRef,
    depth: usize,
    child_depth: usize,
    show_full_name: bool,
    renderer: &mut HierarchyRenderer,
) {
    let scope = &hierarchy[scope_ref];
    let is_module = matches!(scope.scope_type(), wellen::ScopeType::Module);

    let mut next_depth = depth;
    let mut next_child_depth = child_depth;
    let mut next_show_full_name = show_full_name;

    if is_module {
        let scope_name = if show_full_name {
            scope.full_name(hierarchy)
        } else {
            scope.name(hierarchy).to_string()
        };

        if !renderer.push_line(format!("{}{}", "  ".repeat(depth), scope_name)) {
            return;
        }

        if child_depth == 0 {
            return;
        }

        next_depth += 1;
        next_child_depth = child_depth.saturating_sub(1);
        next_show_full_name = false;
    }
    for item in scope.items(hierarchy) {
        if renderer.is_full() {
            renderer.truncated = true;
            break;
        }

        render_item(
            hierarchy,
            item,
            next_depth,
            next_child_depth,
            next_show_full_name,
            renderer,
        );
    }
}
