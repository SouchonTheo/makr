/// Fuzzy match: checks if all characters of the pattern appear in order in the target string.
/// Returns the matched character indices for highlighting.
pub fn fuzzy_match(pattern: &str, target: &str) -> Option<Vec<usize>> {
    let pattern_lower: Vec<char> = pattern.to_lowercase().chars().collect();
    let target_lower: Vec<char> = target.to_lowercase().chars().collect();

    if pattern_lower.is_empty() {
        return Some(vec![]);
    }

    let mut matches = Vec::new();
    let mut pi = 0;

    for (ti, tc) in target_lower.iter().enumerate() {
        if pi < pattern_lower.len() && *tc == pattern_lower[pi] {
            matches.push(ti);
            pi += 1;
        }
    }

    if pi == pattern_lower.len() {
        Some(matches)
    } else {
        None
    }
}

/// Score a fuzzy match — lower is better.
/// Prefers: exact prefix > consecutive chars > spread out chars.
pub fn fuzzy_score(pattern: &str, target: &str) -> Option<u32> {
    let matched = fuzzy_match(pattern, target)?;
    if matched.is_empty() {
        return Some(0);
    }

    let mut score: u32 = 0;

    // Penalize first match not at start
    score += matched[0] as u32 * 10;

    // Penalize gaps between matched chars
    for w in matched.windows(2) {
        let gap = w[1] - w[0] - 1;
        score += gap as u32 * 5;
    }

    // Penalize length difference
    score += (target.len() as u32).saturating_sub(pattern.len() as u32);

    Some(score)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── fuzzy_match ──────────────────────────────────────────────

    #[test]
    fn fuzzy_match_exact() {
        let result = fuzzy_match("clean", "clean");
        assert_eq!(result, Some(vec![0, 1, 2, 3, 4]));
    }

    #[test]
    fn fuzzy_match_partial() {
        // "cln" matches c-l-e-a-n at positions 0, 1, 4  — wait, 'l' is at 1, 'n' at 4
        let result = fuzzy_match("cln", "clean");
        assert_eq!(result, Some(vec![0, 1, 4]));
    }

    #[test]
    fn fuzzy_match_no_match() {
        assert_eq!(fuzzy_match("xyz", "clean"), None);
    }

    #[test]
    fn fuzzy_match_case_insensitive() {
        let result = fuzzy_match("CLEAN", "clean");
        assert_eq!(result, Some(vec![0, 1, 2, 3, 4]));

        let result = fuzzy_match("clean", "CLEAN");
        assert_eq!(result, Some(vec![0, 1, 2, 3, 4]));

        let result = fuzzy_match("ClEaN", "cLeAn");
        assert_eq!(result, Some(vec![0, 1, 2, 3, 4]));
    }

    #[test]
    fn fuzzy_match_empty_pattern() {
        let result = fuzzy_match("", "anything");
        assert_eq!(result, Some(vec![]));
    }

    #[test]
    fn fuzzy_match_empty_target() {
        assert_eq!(fuzzy_match("a", ""), None);
    }

    #[test]
    fn fuzzy_match_both_empty() {
        assert_eq!(fuzzy_match("", ""), Some(vec![]));
    }

    #[test]
    fn fuzzy_match_pattern_longer_than_target() {
        assert_eq!(fuzzy_match("abcdef", "abc"), None);
    }

    // ── fuzzy_score ──────────────────────────────────────────────

    #[test]
    fn fuzzy_score_exact_match_is_best() {
        let exact = fuzzy_score("clean", "clean").unwrap();
        let partial = fuzzy_score("clean", "cleanall").unwrap();
        assert!(
            exact < partial,
            "exact ({exact}) should be < partial ({partial})"
        );
    }

    #[test]
    fn fuzzy_score_prefix_beats_spread() {
        // "cl" at start of "clean" vs "cl" spread in "xclyz" — wait, need a real spread.
        // "ab" matching "ab___" (prefix) vs "a__b_" (spread)
        let prefix = fuzzy_score("ab", "abcde").unwrap();
        let spread = fuzzy_score("ab", "a__b_").unwrap();
        assert!(
            prefix < spread,
            "prefix ({prefix}) should be < spread ({spread})"
        );
    }

    #[test]
    fn fuzzy_score_no_match_returns_none() {
        assert_eq!(fuzzy_score("xyz", "abc"), None);
    }

    #[test]
    fn fuzzy_score_empty_pattern_is_zero() {
        assert_eq!(fuzzy_score("", "anything"), Some(0));
    }

    #[test]
    fn fuzzy_score_exact_is_zero_for_same_length() {
        // first match at 0 (penalty 0), no gaps, length diff 0
        assert_eq!(fuzzy_score("abc", "abc"), Some(0));
    }
}
