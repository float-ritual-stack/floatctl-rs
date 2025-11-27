//! FloatQL Parser - Extracts FLOAT patterns from natural language queries.
//!
//! Ported from Python implementation at github.com/float-ritual-stack/floatctl
//! Uses progressive extraction: extract patterns → remove from query → remaining = text terms

use chrono::{Datelike, Duration, Local, NaiveDate};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;

/// Core FLOAT patterns (:: notation)
static FLOAT_PATTERNS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "ctx", "highlight", "signal", "mode", "project", "bridge",
        "dispatch", "float", "redux", "uid", "claude", "status",
        "priority", "type", "content_type", "context_type",
    ]
    .into_iter()
    .collect()
});

/// Persona patterns ([persona::] notation)
static PERSONA_PATTERNS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    ["sysop", "karen", "qtb", "evna", "lf1m", "littlefucker"]
        .into_iter()
        .collect()
});

/// Compiled regex patterns
static BRIDGE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"CB-\d{8}-\d{4}-[A-Z0-9]{4}").unwrap());
static FLOAT_MARKER_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(\w+)::").unwrap());
static PERSONA_MARKER_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\[(\w+)::\]").unwrap());

// Temporal patterns
static TODAY_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)\btoday\b").unwrap());
static YESTERDAY_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)\byesterday\b").unwrap());
static LAST_HOURS_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\blast\s+(\d+)\s+hours?\b").unwrap());
static LAST_DAYS_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\blast\s+(\d+)\s+days?\b").unwrap());
static LAST_WEEK_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\blast\s+week\b").unwrap());
static THIS_WEEK_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\bthis\s+week\b").unwrap());
static ISO_DATE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(\d{4}-\d{2}-\d{2})\b").unwrap());

// Type patterns
static EXPLICIT_TYPE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\btype:\s*(\w+)\b").unwrap());
static IMPLICIT_TYPE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(logs?|conversations?|bridges?|dispatches?|highlights?)\b").unwrap());

// Scrying Paper patterns (extensions for literate documents)
static WIKILINK_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\[\[([^\]!][^\]]*)\]\]").unwrap());
static COMMAND_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\[\[!([^\]]+)\]\]").unwrap());
static DIRECTIVE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^::(\w+)(?:\s+->?\s*(.+))?$").unwrap());

/// Temporal filter type
#[derive(Debug, Clone)]
pub enum TemporalFilter {
    Date(NaiveDate),
    Since(chrono::DateTime<Local>),
}

/// Parsed FloatQL query result
#[derive(Debug, Default)]
pub struct ParsedQuery {
    /// Plain text search terms (after all patterns extracted)
    pub text_terms: Vec<String>,
    /// Detected :: patterns (ctx, highlight, etc.)
    pub float_patterns: Vec<String>,
    /// Detected [persona::] patterns
    pub persona_patterns: Vec<String>,
    /// Bridge IDs (CB-YYYYMMDD-HHMM-XXXX)
    pub bridge_ids: Vec<String>,
    /// Time-based filters
    pub temporal_filter: Option<TemporalFilter>,
    /// Type filters (log, conversation, bridge, etc.)
    pub type_filters: Vec<String>,
    /// Wikilinks [[...]] (for scrying paper)
    pub wikilinks: Vec<String>,
    /// Commands [[!...]] (for scrying paper)
    pub commands: Vec<String>,
    /// Directives ::... (for scrying paper)
    pub directives: Vec<(String, Option<String>)>,
    /// Original query string
    pub raw_query: String,
}

/// FloatQL Parser
///
/// Recognizes FLOAT patterns:
/// - :: notation (ctx::, highlight::, signal::, etc.)
/// - [persona::] notation (sysop, karen, qtb, evna, lf1m)
/// - Bridge IDs (CB-YYYYMMDD-HHMM-XXXX)
/// - Temporal filters (today, yesterday, last week, etc.)
/// - Type filters (log, conversation, bridge, etc.)
/// - Wikilinks [[...]] (scrying paper extension)
/// - Commands [[!...]] (scrying paper extension)
/// - Directives ::dispatch, ::pipe (scrying paper extension)
pub struct FloatQLParser;

impl FloatQLParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse a FloatQL query into structured components.
    ///
    /// Uses progressive extraction: extract patterns in priority order,
    /// remove from query as we go, remaining text = search terms.
    pub fn parse(&self, query: &str) -> ParsedQuery {
        let mut result = ParsedQuery {
            raw_query: query.to_string(),
            ..Default::default()
        };

        // Working copy for progressive extraction
        let mut remaining = query.to_string();

        // 1. Extract bridge IDs
        for cap in BRIDGE_PATTERN.find_iter(query) {
            result.bridge_ids.push(cap.as_str().to_string());
        }
        remaining = BRIDGE_PATTERN.replace_all(&remaining, "").to_string();

        // 2. Extract wikilinks [[...]] (before commands to avoid overlap)
        for cap in WIKILINK_PATTERN.captures_iter(query) {
            result.wikilinks.push(cap[1].to_string());
        }
        remaining = WIKILINK_PATTERN.replace_all(&remaining, "").to_string();

        // 3. Extract commands [[!...]]
        for cap in COMMAND_PATTERN.captures_iter(query) {
            result.commands.push(cap[1].to_string());
        }
        remaining = COMMAND_PATTERN.replace_all(&remaining, "").to_string();

        // 4. Extract persona patterns [persona::]
        for cap in PERSONA_MARKER_PATTERN.captures_iter(&remaining.clone()) {
            let persona = cap[1].to_lowercase();
            if PERSONA_PATTERNS.contains(persona.as_str()) {
                result.persona_patterns.push(persona);
            }
        }
        remaining = PERSONA_MARKER_PATTERN.replace_all(&remaining, "").to_string();

        // 5. Extract FLOAT :: patterns
        for cap in FLOAT_MARKER_PATTERN.captures_iter(&remaining.clone()) {
            let pattern = cap[1].to_lowercase();
            // Include both known and custom patterns
            result.float_patterns.push(pattern);
        }
        remaining = FLOAT_MARKER_PATTERN.replace_all(&remaining, "").to_string();

        // 6. Extract temporal filters
        remaining = self.extract_temporal_filters(&remaining, &mut result);

        // 7. Extract type filters
        remaining = self.extract_type_filters(&remaining, &mut result);

        // 8. Extract directives (line by line for multi-line documents)
        let mut non_directive_lines = Vec::new();
        for line in remaining.lines() {
            if let Some(cap) = DIRECTIVE_PATTERN.captures(line.trim()) {
                let directive = cap[1].to_string();
                let arg = cap.get(2).map(|m| m.as_str().trim().to_string());
                result.directives.push((directive, arg));
            } else {
                non_directive_lines.push(line);
            }
        }
        remaining = non_directive_lines.join("\n");

        // 9. Remaining text becomes search terms
        result.text_terms = remaining
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        result
    }

    fn extract_temporal_filters(&self, query: &str, result: &mut ParsedQuery) -> String {
        let mut remaining = query.to_string();
        let now = Local::now();

        // Today
        if TODAY_PATTERN.is_match(&remaining) {
            result.temporal_filter = Some(TemporalFilter::Date(now.date_naive()));
            remaining = TODAY_PATTERN.replace_all(&remaining, "").to_string();
        }
        // Yesterday
        else if YESTERDAY_PATTERN.is_match(&remaining) {
            result.temporal_filter = Some(TemporalFilter::Date(
                (now - Duration::days(1)).date_naive(),
            ));
            remaining = YESTERDAY_PATTERN.replace_all(&remaining, "").to_string();
        }
        // Last N hours
        else if let Some(cap) = LAST_HOURS_PATTERN.captures(&remaining) {
            let hours: i64 = cap[1].parse().unwrap_or(1);
            result.temporal_filter = Some(TemporalFilter::Since(now - Duration::hours(hours)));
            remaining = LAST_HOURS_PATTERN.replace_all(&remaining, "").to_string();
        }
        // Last N days
        else if let Some(cap) = LAST_DAYS_PATTERN.captures(&remaining) {
            let days: i64 = cap[1].parse().unwrap_or(1);
            result.temporal_filter = Some(TemporalFilter::Since(now - Duration::days(days)));
            remaining = LAST_DAYS_PATTERN.replace_all(&remaining, "").to_string();
        }
        // Last week
        else if LAST_WEEK_PATTERN.is_match(&remaining) {
            result.temporal_filter = Some(TemporalFilter::Since(now - Duration::days(7)));
            remaining = LAST_WEEK_PATTERN.replace_all(&remaining, "").to_string();
        }
        // This week
        else if THIS_WEEK_PATTERN.is_match(&remaining) {
            let weekday = now.weekday().num_days_from_monday() as i64;
            result.temporal_filter = Some(TemporalFilter::Since(now - Duration::days(weekday)));
            remaining = THIS_WEEK_PATTERN.replace_all(&remaining, "").to_string();
        }
        // ISO date
        else if let Some(cap) = ISO_DATE_PATTERN.captures(&remaining) {
            if let Ok(date) = NaiveDate::parse_from_str(&cap[1], "%Y-%m-%d") {
                result.temporal_filter = Some(TemporalFilter::Date(date));
            }
            remaining = ISO_DATE_PATTERN.replace_all(&remaining, "").to_string();
        }

        remaining
    }

    fn extract_type_filters(&self, query: &str, result: &mut ParsedQuery) -> String {
        let mut remaining = query.to_string();

        // Explicit type:value
        for cap in EXPLICIT_TYPE_PATTERN.captures_iter(&remaining.clone()) {
            let type_value = cap[1].to_lowercase();
            if !result.type_filters.contains(&type_value) {
                result.type_filters.push(type_value);
            }
        }
        remaining = EXPLICIT_TYPE_PATTERN.replace_all(&remaining, "").to_string();

        // Implicit types (logs, conversations, etc.) - just detect, don't remove
        for cap in IMPLICIT_TYPE_PATTERN.captures_iter(&remaining) {
            let mut type_value = cap[1].to_lowercase();
            // Normalize plural to singular
            if type_value.ends_with('s') {
                type_value.pop();
            }
            if !result.type_filters.contains(&type_value) {
                result.type_filters.push(type_value);
            }
        }

        remaining
    }

    /// Check if query contains FloatQL patterns
    pub fn is_floatql_query(&self, query: &str) -> bool {
        // :: patterns
        if query.contains("::") {
            return true;
        }
        // Bridge IDs
        if BRIDGE_PATTERN.is_match(query) {
            return true;
        }
        // Temporal keywords
        let temporal_keywords = ["today", "yesterday", "last", "week", "hours", "days"];
        if temporal_keywords
            .iter()
            .any(|kw| query.to_lowercase().contains(kw))
        {
            return true;
        }
        // Type filters
        if query.to_lowercase().contains("type:") {
            return true;
        }
        // Scrying paper patterns
        if query.contains("[[") || query.starts_with("::") {
            return true;
        }
        false
    }

    /// Suggest collections based on parsed query components
    pub fn get_suggested_collections(&self, parsed: &ParsedQuery) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Bridge queries -> bridge collections
        if !parsed.bridge_ids.is_empty() {
            suggestions.extend([
                "float_bridges".to_string(),
                "rangle_bridges".to_string(),
                "archived_float_bridges".to_string(),
            ]);
        }

        // Context patterns -> active streams
        if parsed.float_patterns.contains(&"ctx".to_string()) {
            suggestions.extend([
                "active_context_stream".to_string(),
                "daily_context_hotcache".to_string(),
            ]);
        }

        // Highlight patterns -> highlight collections
        if parsed.float_patterns.contains(&"highlight".to_string()) {
            suggestions.extend([
                "float_highlights".to_string(),
                "conversation_highlights".to_string(),
            ]);
        }

        // Dispatch patterns -> dispatch collections
        if parsed.float_patterns.contains(&"dispatch".to_string()) {
            suggestions.extend([
                "float_dispatch_bay".to_string(),
                "dispatch_bay".to_string(),
            ]);
        }

        // Conversation type -> conversation collections
        if parsed.type_filters.contains(&"conversation".to_string()) {
            suggestions.extend([
                "float_conversations_active".to_string(),
                "my_conversations".to_string(),
            ]);
        }

        // Bridge type -> bridge collections
        if parsed.type_filters.contains(&"bridge".to_string()) {
            suggestions.extend([
                "float_bridges".to_string(),
                "rangle_bridges".to_string(),
            ]);
        }

        // Recent temporal filters -> active collections
        if parsed.temporal_filter.is_some() {
            suggestions.extend([
                "active_context_stream".to_string(),
                "daily_context_hotcache".to_string(),
            ]);
        }

        // Default if no specific patterns
        if suggestions.is_empty() {
            suggestions = vec![
                "active_context_stream".to_string(),
                "float_bridges".to_string(),
                "float_highlights".to_string(),
            ];
        }

        // Deduplicate while preserving order
        let mut seen = HashSet::new();
        suggestions.retain(|x| seen.insert(x.clone()));

        suggestions
    }

    /// Extract a basic search query from parsed components
    pub fn extract_search_terms(&self, parsed: &ParsedQuery) -> String {
        let mut terms = Vec::new();

        // Include text terms
        terms.extend(parsed.text_terms.iter().cloned());

        // Include pattern values as search terms
        for pattern in &parsed.float_patterns {
            terms.push(format!("{}::", pattern));
        }

        for persona in &parsed.persona_patterns {
            terms.push(format!("[{}::]", persona));
        }

        // Include bridge IDs
        terms.extend(parsed.bridge_ids.iter().cloned());

        terms.join(" ")
    }
}

impl Default for FloatQLParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_query() {
        let parser = FloatQLParser::new();
        let result = parser.parse("ctx:: meeting with nick");

        assert!(result.float_patterns.contains(&"ctx".to_string()));
        assert!(result.text_terms.contains(&"meeting".to_string()));
    }

    #[test]
    fn test_parse_bridge_id() {
        let parser = FloatQLParser::new();
        let result = parser.parse("bridge::CB-20250713-0130-M3SS");

        assert_eq!(result.bridge_ids, vec!["CB-20250713-0130-M3SS"]);
    }

    #[test]
    fn test_parse_persona() {
        let parser = FloatQLParser::new();
        let result = parser.parse("[sysop::] infrastructure updates");

        assert!(result.persona_patterns.contains(&"sysop".to_string()));
    }

    #[test]
    fn test_parse_temporal() {
        let parser = FloatQLParser::new();
        let result = parser.parse("highlights from yesterday");

        assert!(matches!(result.temporal_filter, Some(TemporalFilter::Date(_))));
    }

    #[test]
    fn test_parse_wikilink() {
        let parser = FloatQLParser::new();
        let result = parser.parse("looking at [[2025-11-27]] notes");

        assert_eq!(result.wikilinks, vec!["2025-11-27"]);
    }

    #[test]
    fn test_parse_command() {
        let parser = FloatQLParser::new();
        let result = parser.parse("check [[!tail -20 stream.jsonl]] for patterns");

        assert_eq!(result.commands, vec!["tail -20 stream.jsonl"]);
    }

    #[test]
    fn test_parse_directive() {
        let parser = FloatQLParser::new();
        let result = parser.parse("some content\n::dispatch -> techcraft");

        assert_eq!(
            result.directives,
            vec![("dispatch".to_string(), Some("techcraft".to_string()))]
        );
    }
}
