//! Search and fuzzy filtering for the TUI
//!
//! Provides fuzzy matching for list items and search result management.

use crate::tui::app::ListItem;

/// Search state management
#[derive(Debug, Default)]
pub struct SearchState {
    /// Current search query
    pub query: String,
    /// Filtered/matched items with scores
    pub results: Vec<SearchResult>,
    /// Whether search is active
    pub active: bool,
    /// Search scope description
    pub scope: String,
}

/// A search result with match information
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The matched item
    pub item: ListItem,
    /// Match score (higher is better)
    pub score: i64,
    /// Matched indices in the title (for highlighting)
    pub match_indices: Vec<usize>,
}

impl SearchState {
    /// Create a new search state
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new search
    pub fn start(&mut self, scope: impl Into<String>) {
        self.active = true;
        self.query.clear();
        self.results.clear();
        self.scope = scope.into();
    }

    /// Update search with new query
    pub fn update_query(&mut self, query: impl Into<String>, items: &[ListItem]) {
        self.query = query.into();
        self.results = fuzzy_filter(&self.query, items);
    }

    /// Clear search state
    pub fn clear(&mut self) {
        self.active = false;
        self.query.clear();
        self.results.clear();
        self.scope.clear();
    }

    /// Get filtered items (or all items if no query)
    pub fn get_filtered(&self, items: &[ListItem]) -> Vec<ListItem> {
        if self.query.is_empty() {
            items.to_vec()
        } else {
            self.results.iter().map(|r| r.item.clone()).collect()
        }
    }
}

/// Fuzzy filter items by query
pub fn fuzzy_filter(query: &str, items: &[ListItem]) -> Vec<SearchResult> {
    if query.is_empty() {
        return items
            .iter()
            .map(|item| SearchResult {
                item: item.clone(),
                score: 0,
                match_indices: vec![],
            })
            .collect();
    }

    let query_lower = query.to_lowercase();
    let query_chars: Vec<char> = query_lower.chars().collect();

    let mut results: Vec<SearchResult> = items
        .iter()
        .filter_map(|item| {
            let (score, indices) = fuzzy_match(&query_chars, &item.title);
            if score > 0 {
                Some(SearchResult {
                    item: item.clone(),
                    score,
                    match_indices: indices,
                })
            } else {
                // Also check subtitle
                if let Some(ref subtitle) = item.subtitle {
                    let (sub_score, _) = fuzzy_match(&query_chars, subtitle);
                    if sub_score > 0 {
                        return Some(SearchResult {
                            item: item.clone(),
                            score: sub_score / 2, // Subtitle matches score lower
                            match_indices: vec![],
                        });
                    }
                }
                None
            }
        })
        .collect();

    // Sort by score descending
    results.sort_by(|a, b| b.score.cmp(&a.score));

    results
}

/// Fuzzy match a query against a target string
/// Returns (score, matched_indices)
fn fuzzy_match(query_chars: &[char], target: &str) -> (i64, Vec<usize>) {
    let target_lower = target.to_lowercase();
    let target_chars: Vec<char> = target_lower.chars().collect();

    if query_chars.is_empty() {
        return (0, vec![]);
    }

    let mut score: i64 = 0;
    let mut indices = Vec::new();
    let mut query_idx = 0;
    let mut prev_match_idx: Option<usize> = None;

    for (target_idx, target_char) in target_chars.iter().enumerate() {
        if query_idx < query_chars.len() && *target_char == query_chars[query_idx] {
            indices.push(target_idx);

            // Scoring:
            // - Base match: +10
            // - Consecutive match: +15 bonus
            // - Word boundary match: +10 bonus
            // - First char match: +20 bonus
            score += 10;

            // Consecutive match bonus
            if let Some(prev) = prev_match_idx {
                if target_idx == prev + 1 {
                    score += 15;
                }
            }

            // First character bonus
            if target_idx == 0 {
                score += 20;
            }

            // Word boundary bonus (after space, _, -, or uppercase in camelCase)
            if target_idx > 0 {
                let prev_char = target_chars[target_idx - 1];
                if prev_char == ' ' || prev_char == '_' || prev_char == '-' {
                    score += 10;
                }
                // CamelCase boundary
                if target.chars().nth(target_idx).map(|c| c.is_uppercase()).unwrap_or(false) {
                    score += 10;
                }
            }

            prev_match_idx = Some(target_idx);
            query_idx += 1;
        }
    }

    // All query chars must match
    if query_idx == query_chars.len() {
        // Length penalty - shorter matches are better
        let length_penalty = (target_chars.len() as i64 - query_chars.len() as i64) / 2;
        score = score.saturating_sub(length_penalty);
        (score.max(1), indices)
    } else {
        (0, vec![])
    }
}

/// Highlight matched characters in a string
pub fn highlight_matches(text: &str, indices: &[usize]) -> Vec<(String, bool)> {
    if indices.is_empty() {
        return vec![(text.to_string(), false)];
    }

    let chars: Vec<char> = text.chars().collect();
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_match = false;

    for (idx, ch) in chars.iter().enumerate() {
        let is_match = indices.contains(&idx);

        if is_match != in_match {
            if !current.is_empty() {
                result.push((current.clone(), in_match));
                current.clear();
            }
            in_match = is_match;
        }

        current.push(*ch);
    }

    if !current.is_empty() {
        result.push((current, in_match));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::ItemKind;

    fn make_item(title: &str) -> ListItem {
        ListItem {
            id: title.to_string(),
            title: title.to_string(),
            subtitle: None,
            kind: ItemKind::File,
            has_children: false,
            meta: None,
        }
    }

    #[test]
    fn test_fuzzy_match_exact() {
        let query: Vec<char> = "test".chars().collect();
        let (score, indices) = fuzzy_match(&query, "test");
        assert!(score > 0);
        assert_eq!(indices, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_fuzzy_match_partial() {
        let query: Vec<char> = "tst".chars().collect();
        let (score, indices) = fuzzy_match(&query, "test");
        assert!(score > 0);
        assert_eq!(indices, vec![0, 2, 3]);
    }

    #[test]
    fn test_fuzzy_match_no_match() {
        let query: Vec<char> = "xyz".chars().collect();
        let (score, _) = fuzzy_match(&query, "test");
        assert_eq!(score, 0);
    }

    #[test]
    fn test_fuzzy_filter() {
        let items = vec![
            make_item("test file"),
            make_item("another thing"),
            make_item("testing 123"),
        ];

        let results = fuzzy_filter("test", &items);
        assert_eq!(results.len(), 2);
        // "test file" should score higher due to first word match
        assert_eq!(results[0].item.title, "test file");
    }

    #[test]
    fn test_highlight_matches() {
        let result = highlight_matches("test", &[0, 2]);
        assert_eq!(result, vec![
            ("t".to_string(), true),
            ("e".to_string(), false),
            ("s".to_string(), true),
            ("t".to_string(), false),
        ]);
    }
}
