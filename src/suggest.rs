//! "Did you mean ..." suggestions for unknown waveform ids, signals and scopes.
//!
//! When a lookup fails, echoing back the available names (or the closest ones) is far more
//! useful than a bare "not found". Suggestions combine edit distance (which also handles
//! transposed characters, e.g. `tb.dtu` vs `tb.dut`) with path containment, so both typos and
//! abbreviated paths (`abc_bank` vs `tb.dut.abc_bank`) are suggested.

use std::cmp::Ordering;

use strsim::normalized_damerau_levenshtein;

/// Maximum number of suggestions shown.
const MAX_SUGGESTIONS: usize = 5;

/// Minimum similarity (0.0 - 1.0) for a candidate to be suggested.
const MIN_SCORE: f64 = 0.6;

/// Maximum number of names listed when nothing is close enough.
const MAX_LISTED: usize = 10;

/// Return the candidates most similar to `query`, best match first.
pub fn did_you_mean(query: &str, candidates: &[String]) -> Vec<String> {
    let query_lower = query.to_lowercase();

    let mut scored: Vec<(f64, &String)> = candidates
        .iter()
        .filter_map(|candidate| similarity(&query_lower, candidate).map(|score| (score, candidate)))
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));

    // Keep only candidates that are close to the best match, so one near-perfect hit does not
    // drag in loosely similar names.
    if let Some(best) = scored.first().map(|(score, _)| *score) {
        scored.retain(|(score, _)| *score >= best * 0.9);
    }

    scored
        .into_iter()
        .take(MAX_SUGGESTIONS)
        .map(|(_, candidate)| candidate.clone())
        .collect()
}

/// Similarity between an already lowercased query and a candidate name.
///
/// Returns `None` for candidates that are too different to be worth a full edit-distance
/// calculation, which keeps suggestions cheap even for hierarchies with many signals.
fn similarity(query_lower: &str, candidate: &str) -> Option<f64> {
    let candidate_lower = candidate.to_lowercase();

    if candidate_lower == *query_lower {
        return Some(1.0);
    }

    let query_len = query_lower.chars().count();
    let candidate_len = candidate_lower.chars().count();

    // Hierarchical paths are long, so containment is a stronger signal than edit distance:
    // `abc_bank` should suggest `tb.dut.abc_bank`.
    if candidate_lower.contains(query_lower) {
        return Some(coverage_score(query_len, candidate_len));
    }

    // The leaf name is what people usually type, so match it as well as the full path.
    let leaf = candidate_lower
        .rsplit('.')
        .next()
        .unwrap_or(&candidate_lower);
    let leaf_len = leaf.chars().count();

    if leaf != candidate_lower && leaf.contains(query_lower) {
        return Some(coverage_score(query_len, leaf_len));
    }

    // Characters in order but not adjacent, e.g. `bnk` for `abc_bank`.
    if is_subsequence(query_lower, leaf) {
        return Some(0.55 + 0.35 * (query_len as f64 / leaf_len as f64));
    }

    // Plain edit distance handles the remaining typos. Skip obviously unrelated candidates,
    // both by full path and by leaf name, before running the quadratic algorithm.
    let length_slack = (query_len / 2).max(4);
    if query_len.abs_diff(candidate_len) > length_slack
        && query_len.abs_diff(leaf_len) > length_slack
    {
        return None;
    }

    let score = normalized_damerau_levenshtein(query_lower, &candidate_lower)
        .max(normalized_damerau_levenshtein(query_lower, leaf));
    (score >= MIN_SCORE).then_some(score)
}

/// Score for a candidate that contains the query, scaled by how much of it the query covers.
fn coverage_score(query_len: usize, candidate_len: usize) -> f64 {
    let shorter = query_len.min(candidate_len) as f64;
    let longer = query_len.max(candidate_len).max(1) as f64;
    0.6 + 0.35 * (shorter / longer)
}

/// Whether the characters of `needle` appear in `haystack` in order.
fn is_subsequence(needle: &str, haystack: &str) -> bool {
    let mut haystack = haystack.chars();
    needle.chars().all(|needle_char| {
        haystack
            .by_ref()
            .any(|haystack_char| haystack_char == needle_char)
    })
}

/// Build a "not found" message with suggestions or the available names.
///
/// `kind` is the lowercase name of the thing being looked up, e.g. `"waveform"`.
pub fn not_found_message(kind: &str, query: &str, candidates: &[String]) -> String {
    let suggestions = did_you_mean(query, candidates);

    if suggestions.len() == 1 {
        return format!(
            "{} '{}' not found. Did you mean '{}'?",
            kind, query, suggestions[0]
        );
    }

    if suggestions.len() > 1 {
        let quoted: Vec<String> = suggestions
            .iter()
            .map(|name| format!("'{}'", name))
            .collect();
        return format!(
            "{} '{}' not found. Did you mean one of: {}?",
            kind,
            query,
            quoted.join(", ")
        );
    }

    if candidates.is_empty() {
        return format!("{} '{}' not found (none available)", kind, query);
    }

    let mut sorted: Vec<&String> = candidates.iter().collect();
    sorted.sort();
    let listed: Vec<&str> = sorted
        .iter()
        .take(MAX_LISTED)
        .map(|name| name.as_str())
        .collect();
    let remaining = sorted.len().saturating_sub(listed.len());
    let suffix = if remaining > 0 {
        format!(", ... ({} total)", sorted.len())
    } else {
        String::new()
    };

    format!(
        "{} '{}' not found. Available {}s: {}{}",
        kind,
        query,
        kind,
        listed.join(", "),
        suffix
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn suggests_single_close_match() {
        let candidates = names(&["tb.dut.abc_bank", "tb.dut.abc_ram", "tb.clk"]);
        assert_eq!(
            did_you_mean("tb.dut.abc_bnk", &candidates),
            vec!["tb.dut.abc_bank".to_string()]
        );
    }

    #[test]
    fn suggests_by_path_suffix() {
        let candidates = names(&["tb.dut.abc_bank", "tb.clk"]);
        let suggestions = did_you_mean("abc_bank", &candidates);
        assert_eq!(suggestions, vec!["tb.dut.abc_bank".to_string()]);
    }

    #[test]
    fn suggestions_are_ranked_best_first_and_capped() {
        let candidates = names(&[
            "tb.a.bank",
            "tb.b.bank",
            "tb.c.bank",
            "tb.d.bank",
            "tb.e.bank",
            "tb.f.bank",
            "tb.g.bank",
        ]);
        let suggestions = did_you_mean("tb.c.bank", &candidates);
        assert_eq!(suggestions.first().unwrap(), "tb.c.bank");
        assert!(suggestions.len() <= MAX_SUGGESTIONS);
    }

    #[test]
    fn suggests_leaf_name_typos_and_subsequences() {
        let candidates = names(&["tb.dut.abc_bank", "tb.dut.abc_ram"]);

        // Transposed characters in the leaf name.
        assert_eq!(
            did_you_mean("tb.dut.abc_bnak", &candidates),
            vec!["tb.dut.abc_bank".to_string()]
        );

        // A query with missing characters still finds the leaf name.
        assert_eq!(
            did_you_mean("bnk", &candidates),
            vec!["tb.dut.abc_bank".to_string()]
        );
    }

    #[test]
    fn transposed_scope_beats_short_prefix() {
        let candidates = names(&["tb", "tb.dut"]);
        assert_eq!(
            did_you_mean("tb.dtu", &candidates),
            vec!["tb.dut".to_string()]
        );
    }

    #[test]
    fn no_suggestions_for_unrelated_names() {
        let candidates = names(&["alpha", "beta"]);
        assert!(did_you_mean("zzzz", &candidates).is_empty());
    }

    #[test]
    fn message_prefers_single_suggestion() {
        let message = not_found_message("waveform", "wave", &names(&["wave.vcd", "x.fst"]));
        assert_eq!(
            message,
            "waveform 'wave' not found. Did you mean 'wave.vcd'?"
        );
    }

    #[test]
    fn message_lists_available_names_when_nothing_matches() {
        let message = not_found_message("waveform", "zzzz", &names(&["b.vcd", "a.vcd"]));
        assert_eq!(
            message,
            "waveform 'zzzz' not found. Available waveforms: a.vcd, b.vcd"
        );
    }

    #[test]
    fn message_truncates_long_available_lists() {
        let candidates: Vec<String> = (0..15).map(|index| format!("w{index}")).collect();
        let message = not_found_message("signal", "zzzz", &candidates);
        assert!(message.contains("(15 total)"), "message: {message}");
    }

    #[test]
    fn message_handles_empty_candidate_list() {
        assert_eq!(
            not_found_message("waveform", "wave", &[]),
            "waveform 'wave' not found (none available)"
        );
    }
}
