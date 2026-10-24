//! Signal path matching.
//!
//! Signal paths (`tb.dut.abc_bank`) can be filtered in three ways:
//!
//! * [`MatchMode::Substring`] (default): case-insensitive substring match
//! * [`MatchMode::Glob`]: case-insensitive glob over the whole path (`*`, `?`,
//!   `[..]`, `{a,b}`, see the `globset` crate)
//! * [`MatchMode::Regex`]: regular expression, case-insensitive unless the
//!   pattern opts out with `(?-i)`
//!
//! Multiple patterns are combined with OR: a path is kept if *any* pattern
//! matches. Hierarchy scoping (`hierarchy_prefix`) is a separate AND filter.

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use regex::RegexSetBuilder;

/// How a signal path pattern is interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MatchMode {
    /// Case-insensitive substring match (default).
    #[default]
    Substring,
    /// Case-insensitive glob match over the whole path.
    Glob,
    /// Regular expression match (case-insensitive by default).
    Regex,
}

impl MatchMode {
    /// Parse a match mode name, accepting a few common aliases.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "substring" | "substr" | "contains" | "literal" => Ok(Self::Substring),
            "glob" | "wildcard" => Ok(Self::Glob),
            "regex" | "regexp" | "re" => Ok(Self::Regex),
            other => Err(format!(
                "Unknown match mode '{}'; expected 'substring', 'glob', or 'regex'",
                other
            )),
        }
    }

    /// Canonical name of the mode.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Substring => "substring",
            Self::Glob => "glob",
            Self::Regex => "regex",
        }
    }
}

impl std::fmt::Display for MatchMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A set of signal path patterns that are OR-ed together.
///
/// An empty matcher matches every path.
#[derive(Debug, Clone, Default)]
pub struct SignalMatcher {
    /// Lowercased substring needles.
    substring: Vec<String>,
    /// Union of all glob patterns.
    glob: Option<GlobSet>,
    /// Union of all regex patterns.
    regex: Option<regex::RegexSet>,
}

impl SignalMatcher {
    /// Compile a list of `(pattern, mode)` pairs.
    ///
    /// Patterns of the same mode are compiled into a single set, and the whole
    /// matcher matches when any of them matches. Invalid glob or regex patterns
    /// are reported as errors instead of silently matching nothing.
    pub fn new(patterns: &[(String, MatchMode)]) -> Result<Self, String> {
        let mut substring = Vec::new();
        let mut glob_patterns = Vec::new();
        let mut regex_patterns = Vec::new();

        for (pattern, mode) in patterns {
            match mode {
                MatchMode::Substring => substring.push(pattern.to_lowercase()),
                MatchMode::Glob => glob_patterns.push(pattern),
                MatchMode::Regex => regex_patterns.push(pattern),
            }
        }

        let glob = if glob_patterns.is_empty() {
            None
        } else {
            let mut builder = GlobSetBuilder::new();
            for pattern in &glob_patterns {
                let glob = GlobBuilder::new(pattern)
                    .case_insensitive(true)
                    .build()
                    .map_err(|e| format!("Invalid glob pattern '{}': {}", pattern, e))?;
                builder.add(glob);
            }
            Some(
                builder
                    .build()
                    .map_err(|e| format!("Invalid glob pattern set: {}", e))?,
            )
        };

        let regex = if regex_patterns.is_empty() {
            None
        } else {
            let mut builder = RegexSetBuilder::new(&regex_patterns);
            builder.case_insensitive(true);
            Some(
                builder
                    .build()
                    .map_err(|e| format!("Invalid regex pattern: {}", e))?,
            )
        };

        Ok(Self {
            substring,
            glob,
            regex,
        })
    }

    /// Build a matcher from a single pattern.
    pub fn single(pattern: &str, mode: MatchMode) -> Result<Self, String> {
        Self::new(&[(pattern.to_string(), mode)])
    }

    /// A matcher that accepts every path.
    pub fn all() -> Self {
        Self::default()
    }

    /// Whether the matcher is empty, i.e. accepts every path.
    pub fn is_empty(&self) -> bool {
        self.substring.is_empty() && self.glob.is_none() && self.regex.is_none()
    }

    /// Check whether `path` matches any of the patterns.
    pub fn is_match(&self, path: &str) -> bool {
        if self.is_empty() {
            return true;
        }

        if !self.substring.is_empty() {
            let path_lower = path.to_lowercase();
            if self
                .substring
                .iter()
                .any(|needle| path_lower.contains(needle.as_str()))
            {
                return true;
            }
        }

        if let Some(glob) = &self.glob
            && glob.is_match(path)
        {
            return true;
        }

        if let Some(regex) = &self.regex
            && regex.is_match(path)
        {
            return true;
        }

        false
    }
}

/// Turn a pattern into a literal seed usable for fuzzy suggestions.
///
/// Glob and regex metacharacters are stripped, so `*bank*` or `^tb\..*_bank$` become
/// `bank` / `tb._bank`, which can be fuzzy-matched against real signal paths.
pub fn pattern_seed(pattern: &str, mode: MatchMode) -> String {
    let metacharacters: &[char] = match mode {
        MatchMode::Substring => &[],
        MatchMode::Glob => &['*', '?', '[', ']', '{', '}', ',', '!'],
        MatchMode::Regex => &[
            '^', '$', '*', '+', '?', '(', ')', '[', ']', '{', '}', '|', '\\',
        ],
    };

    pattern
        .chars()
        .filter(|c| !metacharacters.contains(c))
        .collect::<String>()
        .trim_matches(|c: char| c == '.' || c.is_whitespace())
        .to_string()
}

/// Detect substring patterns that look like globs, e.g. `*bank*`.
///
/// Returns a hint string when the caller probably wanted glob matching.
pub fn glob_hint(patterns: &[(String, MatchMode)]) -> Option<String> {
    let suspicious: Vec<String> = patterns
        .iter()
        .filter(|(pattern, mode)| {
            *mode == MatchMode::Substring && (pattern.contains('*') || pattern.contains('?'))
        })
        .map(|(pattern, _)| format!("'{}'", pattern))
        .collect();

    if suspicious.is_empty() {
        return None;
    }

    Some(format!(
        "hint: {} used with substring mode, where '*' and '?' are literal characters; \
         use glob mode for wildcards (CLI: --glob, MCP: match_mode=glob)",
        suspicious.join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matcher(patterns: &[(&str, MatchMode)]) -> SignalMatcher {
        let patterns: Vec<(String, MatchMode)> =
            patterns.iter().map(|(p, m)| (p.to_string(), *m)).collect();
        SignalMatcher::new(&patterns).expect("patterns should compile")
    }

    #[test]
    fn empty_matcher_matches_everything() {
        let m = SignalMatcher::all();
        assert!(m.is_empty());
        assert!(m.is_match("tb.dut.abc_bank"));
    }

    #[test]
    fn substring_is_case_insensitive() {
        let m = matcher(&[("BaNk", MatchMode::Substring)]);
        assert!(m.is_match("tb.dut.abc_bank"));
        assert!(!m.is_match("tb.dut.abc_ram"));
    }

    #[test]
    fn substring_star_is_literal() {
        let m = matcher(&[("*bank*", MatchMode::Substring)]);
        assert!(!m.is_match("tb.dut.abc_bank"));
    }

    #[test]
    fn glob_matches_across_hierarchy() {
        let m = matcher(&[("*bank*", MatchMode::Glob)]);
        assert!(m.is_match("tb.dut.abc_bank"));
        assert!(m.is_match("tb.bank.din"));
        assert!(!m.is_match("tb.dut.abc_ram"));
    }

    #[test]
    fn glob_question_mark_and_class() {
        let m = matcher(&[("tb.dut.bank[0-9]", MatchMode::Glob)]);
        assert!(m.is_match("tb.dut.bank3"));
        assert!(!m.is_match("tb.dut.bankx"));
        let m = matcher(&[("tb.dut.b?nk", MatchMode::Glob)]);
        assert!(m.is_match("tb.dut.bank"));
    }

    #[test]
    fn glob_is_case_insensitive() {
        let m = matcher(&[("*BANK*", MatchMode::Glob)]);
        assert!(m.is_match("tb.dut.abc_bank"));
    }

    #[test]
    fn glob_is_anchored_to_the_whole_path() {
        let m = matcher(&[("bank", MatchMode::Glob)]);
        assert!(!m.is_match("tb.dut.abc_bank"));
        let m = matcher(&[("*bank", MatchMode::Glob)]);
        assert!(m.is_match("tb.dut.abc_bank"));
    }

    #[test]
    fn regex_is_case_insensitive_by_default() {
        let m = matcher(&[("^tb\\.dut\\..*_bank\\[3:0\\]$", MatchMode::Regex)]);
        assert!(m.is_match("TB.dut.abc_BANK[3:0]"));
        assert!(!m.is_match("tb.other.abc_bank[3:0]"));
    }

    #[test]
    fn regex_can_opt_out_of_case_insensitivity() {
        let m = matcher(&[("(?-i)tb\\.dut", MatchMode::Regex)]);
        assert!(m.is_match("tb.dut.abc_bank"));
        assert!(!m.is_match("TB.dut.abc_bank"));
    }

    #[test]
    fn multiple_patterns_are_or_ed() {
        let m = matcher(&[
            ("bank", MatchMode::Substring),
            ("*_ram", MatchMode::Glob),
            ("(?-i)^tb\\.TOP$", MatchMode::Regex),
        ]);
        assert!(m.is_match("tb.dut.abc_bank"));
        assert!(m.is_match("tb.dut.abc_ram"));
        assert!(m.is_match("tb.TOP"));
        assert!(!m.is_match("tb.dut.other"));
    }

    #[test]
    fn invalid_regex_is_reported() {
        let err = SignalMatcher::single("(unclosed", MatchMode::Regex).unwrap_err();
        assert!(err.contains("Invalid regex"), "unexpected error: {err}");
    }

    #[test]
    fn invalid_glob_is_reported() {
        let err = SignalMatcher::single("a[", MatchMode::Glob).unwrap_err();
        assert!(err.contains("Invalid glob"), "unexpected error: {err}");
    }

    #[test]
    fn pattern_seed_strips_metacharacters() {
        assert_eq!(pattern_seed("bank", MatchMode::Substring), "bank");
        assert_eq!(pattern_seed("*bank*", MatchMode::Glob), "bank");
        assert_eq!(pattern_seed("tb.*.clk", MatchMode::Glob), "tb..clk");
        assert_eq!(
            pattern_seed("^tb\\..*_bank$", MatchMode::Regex),
            "tb.._bank"
        );
        assert_eq!(pattern_seed(".*", MatchMode::Regex), "");
    }

    #[test]
    fn match_mode_parsing() {
        assert_eq!(MatchMode::parse("SUBSTRING").unwrap(), MatchMode::Substring);
        assert_eq!(MatchMode::parse("glob").unwrap(), MatchMode::Glob);
        assert_eq!(MatchMode::parse("regexp").unwrap(), MatchMode::Regex);
        assert!(MatchMode::parse("fuzzy").is_err());
    }

    #[test]
    fn glob_hint_only_for_substring_patterns() {
        let patterns = vec![("*bank*".to_string(), MatchMode::Substring)];
        assert!(glob_hint(&patterns).is_some());

        let patterns = vec![("*bank*".to_string(), MatchMode::Glob)];
        assert!(glob_hint(&patterns).is_none());

        let patterns = vec![("bank".to_string(), MatchMode::Substring)];
        assert!(glob_hint(&patterns).is_none());
    }
}
