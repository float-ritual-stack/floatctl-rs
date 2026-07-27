//! YAML frontmatter normalization — Rust port of `/opt/float/bbs/scripts/yaml-normalize.py`.
//!
//! Behaviours (preserved verbatim from the Python source-of-truth):
//!
//! 1. **Canonical key rename**: `related|connectsTo|connectTo|connects|backlinks|references → relates`.
//!    On collision with an existing `relates`, values are unioned (dedup, order preserved).
//! 2. **Flow-style enforcement** for `tags` and `relates` — block-list form
//!    (`tags:\n- foo\n- bar`) is rewritten to flow form (`tags: [foo, bar]`).
//! 3. **Doubled-frontmatter merge** — files with two `---` blocks at the top
//!    (with up to 10 lines of marker-line "gap" between them) are collapsed
//!    into a single merged frontmatter, with the gap preserved as body prefix.
//!    Guarded against false positives (mid-body `---` horizontal rules).
//! 4. **Bare wikilink pre-quoting** — `key: [[a]], [[b]]` and `- [[a]]` are
//!    quoted so the YAML parser accepts them.
//! 5. **Colon-in-scalar pre-quoting** — `title: Dispatch: BBS Sweep` is quoted
//!    to prevent the parser from treating it as a nested mapping.
//! 6. **Duplicate list-key merge** — two top-level `relates:` blocks are unioned.
//!
//! The normalizer is **idempotent**: `normalize_str(normalize_str(x)) == normalize_str(x)`.

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
use thiserror::Error;

const DOUBLED_GAP_MAX_LINES: usize = 10;

/// Errors produced by the normalizer.
#[derive(Debug, Error)]
pub enum NormalizeError {
    #[error("YAML parse error: {0}")]
    ParseYaml(#[from] serde_yml::Error),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

/// Outcome of normalizing a single file.
#[derive(Debug, PartialEq, Eq)]
pub enum NormalizeOutcome {
    /// Content was already canonical; no rewrite needed.
    Unchanged,
    /// Content was rewritten. `new_content` carries the normalized text.
    /// When `apply == true`, the file has already been written.
    Changed { new_content: String },
}

// ─── regexes (mirroring the Python module-level definitions) ────────────────

// `key: [[a]], [[b]]` or `key: [[a]] [[b]]` — comma or whitespace separator
static BARE_WIKILINK_LIST_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?m)^(?P<indent>\s*)(?P<key>\w+):[ \t]+(?P<body>\[\[[^\]]+\]\](?:[ \t]*[,\s][ \t]*\[\[[^\]]+\]\])+)[ \t]*$",
    )
    .expect("BARE_WIKILINK_LIST_RE compile")
});

// `  - [[foo]]` — block list item with unquoted wikilink
static BARE_WIKILINK_BLOCK_ITEM_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^(?P<indent>\s*-\s+)(?P<wikilink>\[\[[^\]]+\]\])\s*$")
        .expect("BARE_WIKILINK_BLOCK_ITEM_RE compile")
});

// `key: <unquoted scalar containing ": ">` — invalid YAML
static COLON_IN_SCALAR_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?m)^(?P<indent>[ \t]*)(?P<key>\w+):[ \t]+(?P<val>[^"'\[\{>|#&*\n][^\n]*?:[ \t]+[^\n]*?)[ \t]*$"#,
    )
    .expect("COLON_IN_SCALAR_RE compile")
});

// just-a-wikilink (used to extract individual `[[...]]` tokens from a list body)
static WIKILINK_TOKEN_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\[\[[^\]]+\]\]").expect("WIKILINK_TOKEN_RE compile"));

// Top-level `key: value` line (no indent)
static KEY_LINE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?P<key>\w+):\s*(?P<val>.*)$").expect("KEY_LINE_RE compile"));

static BLOCK_ITEM_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(\s*)-\s").expect("BLOCK_ITEM_RE compile"));

// Canonical key rename map
fn canonical_for(key: &str) -> Option<&'static str> {
    match key {
        "related" | "connectsTo" | "connectTo" | "connects" | "backlinks" | "references" => {
            Some("relates")
        }
        _ => None,
    }
}

const FLOW_KEYS: &[&str] = &["tags", "relates"];

// ─── preprocessors ──────────────────────────────────────────────────────────

fn pre_quote_wikilinks(fm_text: &str) -> String {
    let after_list = BARE_WIKILINK_LIST_RE.replace_all(fm_text, |caps: &regex::Captures| {
        let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
        let key = caps.name("key").map(|m| m.as_str()).unwrap_or("");
        let body = caps.name("body").map(|m| m.as_str()).unwrap_or("");
        let items: Vec<String> = WIKILINK_TOKEN_RE
            .find_iter(body)
            .map(|m| format!("\"{}\"", m.as_str()))
            .collect();
        format!("{}{}: [{}]", indent, key, items.join(", "))
    });

    let after_block = BARE_WIKILINK_BLOCK_ITEM_RE.replace_all(&after_list, |caps: &regex::Captures| {
        let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
        let wikilink = caps.name("wikilink").map(|m| m.as_str()).unwrap_or("");
        format!("{}\"{}\"", indent, wikilink)
    });

    after_block.into_owned()
}

fn pre_quote_colon_scalars(fm_text: &str) -> String {
    COLON_IN_SCALAR_RE
        .replace_all(fm_text, |caps: &regex::Captures| {
            let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
            let key = caps.name("key").map(|m| m.as_str()).unwrap_or("");
            let val = caps.name("val").map(|m| m.as_str()).unwrap_or("").trim_end();
            let escaped = val.replace('\\', "\\\\").replace('"', "\\\"");
            format!("{}{}: \"{}\"", indent, key, escaped)
        })
        .into_owned()
}

/// Split a flow-list body on commas, respecting `[[...]]` and quotes.
fn split_flow(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut buf = String::new();
    let mut depth: i32 = 0;
    let mut in_quote: Option<char> = None;
    for ch in s.chars() {
        if let Some(q) = in_quote {
            buf.push(ch);
            if ch == q {
                in_quote = None;
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            in_quote = Some(ch);
            buf.push(ch);
            continue;
        }
        if ch == '[' {
            depth += 1;
            buf.push(ch);
        } else if ch == ']' {
            depth -= 1;
            buf.push(ch);
        } else if ch == ',' && depth == 0 {
            out.push(std::mem::take(&mut buf));
        } else {
            buf.push(ch);
        }
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    out
}

/// Merge duplicate top-level list-valued keys (e.g. two `relates:` blocks).
/// Only handles top-level (no indent) keys with block-list or flow-list values.
fn merge_duplicate_list_keys(fm_text: &str) -> String {
    let lines: Vec<&str> = fm_text.split('\n').collect();
    // groups: key -> Vec<(start_idx, end_idx_exclusive, items)>
    let mut groups: BTreeMap<String, Vec<(usize, usize, Vec<String>)>> = BTreeMap::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if !line.starts_with(' ') && !line.starts_with('\t') {
            if let Some(caps) = KEY_LINE_RE.captures(line) {
                let key = caps.name("key").unwrap().as_str().to_string();
                let val = caps.name("val").unwrap().as_str().trim().to_string();
                let mut items: Vec<String> = Vec::new();
                let mut j = i + 1;
                if !val.is_empty() {
                    if val.starts_with('[') && val.ends_with(']') {
                        let inner = &val[1..val.len() - 1].trim();
                        if !inner.is_empty() {
                            items = split_flow(inner)
                                .into_iter()
                                .map(|s| s.trim().to_string())
                                .collect();
                        }
                    }
                } else {
                    while j < lines.len() {
                        let nxt = lines[j];
                        let stripped = nxt.trim_start();
                        if stripped.is_empty() {
                            j += 1;
                            continue;
                        }
                        if BLOCK_ITEM_RE.is_match(nxt) {
                            // strip leading "- "
                            let item = stripped.trim_start_matches('-').trim().to_string();
                            items.push(item);
                            j += 1;
                        } else {
                            break;
                        }
                    }
                }
                groups
                    .entry(key)
                    .or_default()
                    .push((i, j, items));
                i = j;
                continue;
            }
        }
        i += 1;
    }

    let dup_keys: Vec<_> = groups
        .iter()
        .filter(|(_, spans)| spans.len() > 1)
        .collect();
    if dup_keys.is_empty() {
        return fm_text.to_string();
    }

    let mut to_drop: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut replacements: BTreeMap<usize, String> = BTreeMap::new();
    for (key, spans) in dup_keys {
        let mut merged: Vec<String> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (_, _, items) in spans {
            for it in items {
                if !it.is_empty() && !seen.contains(it) {
                    seen.insert(it.clone());
                    merged.push(it.clone());
                }
            }
        }
        let (first_start, first_end, _) = &spans[0];
        replacements.insert(*first_start, format!("{}: [{}]", key, merged.join(", ")));
        for idx in (first_start + 1)..*first_end {
            to_drop.insert(idx);
        }
        for (start, end, _) in &spans[1..] {
            for idx in *start..*end {
                to_drop.insert(idx);
            }
        }
    }

    let mut out: Vec<String> = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        if to_drop.contains(&idx) {
            continue;
        }
        if let Some(repl) = replacements.get(&idx) {
            out.push(repl.clone());
        } else {
            out.push((*line).to_string());
        }
    }
    out.join("\n")
}

fn preprocess_fm(fm_text: &str) -> String {
    let s = pre_quote_colon_scalars(fm_text);
    let s = pre_quote_wikilinks(&s);
    merge_duplicate_list_keys(&s)
}

// ─── frontmatter detection ──────────────────────────────────────────────────

fn split_frontmatter(text: &str) -> Option<(String, String)> {
    if !text.starts_with("---\n") {
        return None;
    }
    // find "\n---\n" after the opening
    let after_open = &text[4..];
    let rel = after_open.find("\n---\n")?;
    let fm = &after_open[..rel];
    let rest_start = 4 + rel + 5; // 4 ("---\n") + rel + 5 ("\n---\n")
    let rest = &text[rest_start..];
    Some((fm.to_string(), rest.to_string()))
}

fn detect_doubled_frontmatter(text: &str) -> Option<(String, String, String, String)> {
    if !text.starts_with("---\n") {
        return None;
    }
    let after_open = &text[4..];
    let rel = after_open.find("\n---\n")?;
    let fm1 = &after_open[..rel];
    let after1 = &after_open[rel + 5..];

    let lines: Vec<&str> = after1.split('\n').collect();
    let mut fm2_open_idx: Option<usize> = None;
    for (i, line) in lines.iter().take(DOUBLED_GAP_MAX_LINES).enumerate() {
        if line.trim() == "---" {
            fm2_open_idx = Some(i);
            break;
        }
    }
    let fm2_open_idx = fm2_open_idx?;
    let gap_body = lines[..fm2_open_idx].join("\n");

    let mut fm2_close_idx: Option<usize> = None;
    for (k, line) in lines.iter().enumerate().skip(fm2_open_idx + 1) {
        if line.trim() == "---" {
            fm2_close_idx = Some(k);
            break;
        }
    }
    let fm2_close_idx = fm2_close_idx?;
    let fm2 = lines[(fm2_open_idx + 1)..fm2_close_idx].join("\n");
    let rest = lines[(fm2_close_idx + 1)..].join("\n");

    Some((fm1.to_string(), gap_body, fm2, rest))
}

// ─── core normalization (operates on parsed mapping) ─────────────────────────

/// Apply canonical-key rename + flow-key marking to a YAML mapping. Returns
/// `(changed, mapping)`. Mapping preserves insertion order via `serde_yml::Mapping`.
fn normalize_mapping(mut m: serde_yml::Mapping) -> (bool, serde_yml::Mapping) {
    let mut changed = false;

    // Key rename (with merge on collision).
    let old_keys: Vec<String> = m
        .iter()
        .filter_map(|(k, _)| k.as_str().map(|s| s.to_string()))
        .filter(|k| canonical_for(k).is_some())
        .collect();

    for old in old_keys {
        let new = canonical_for(&old).unwrap().to_string();
        let old_val = m.remove(serde_yml::Value::String(old.clone()));
        changed = true;
        if let Some(old_val) = old_val {
            let existing = m.remove(serde_yml::Value::String(new.clone()));
            let merged_seq = match (existing, old_val) {
                (Some(serde_yml::Value::Sequence(mut a)), serde_yml::Value::Sequence(b)) => {
                    for item in b {
                        if !a.contains(&item) {
                            a.push(item);
                        }
                    }
                    serde_yml::Value::Sequence(a)
                }
                (Some(existing), old_val) => {
                    // existing is non-list (rare); coerce to sequence
                    let mut seq = match existing {
                        serde_yml::Value::Sequence(s) => s,
                        other => vec![other],
                    };
                    match old_val {
                        serde_yml::Value::Sequence(b) => {
                            for item in b {
                                if !seq.contains(&item) {
                                    seq.push(item);
                                }
                            }
                        }
                        other => {
                            if !seq.contains(&other) {
                                seq.push(other);
                            }
                        }
                    }
                    serde_yml::Value::Sequence(seq)
                }
                (None, v) => v,
            };
            m.insert(serde_yml::Value::String(new), merged_seq);
        }
    }

    (changed, m)
}

/// Post-process emitted YAML to convert block-list emission of `tags:` and
/// `relates:` into flow-list form. serde_yml emits block-style by default;
/// ruamel-style per-key flow control isn't exposed by the crate, so we do this
/// in-place with a small line walker.
fn block_to_flow_for_flow_keys(emitted: &str) -> String {
    let lines: Vec<&str> = emitted.split('\n').collect();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        // Top-level key with empty value: `tags:` / `relates:` followed by `- item` lines.
        let trimmed = line;
        let is_flow_key_header = FLOW_KEYS
            .iter()
            .any(|k| trimmed == format!("{}:", k) || trimmed == format!("{}: ", k));
        if is_flow_key_header {
            let key_name = trimmed.trim_end_matches(':').trim_end_matches(' ');
            // Collect following `- ` items at any indent (block list children).
            let mut items: Vec<String> = Vec::new();
            let mut j = i + 1;
            while j < lines.len() {
                let nxt = lines[j];
                let stripped = nxt.trim_start();
                if stripped.starts_with("- ") || stripped == "-" {
                    let item = stripped.trim_start_matches('-').trim().to_string();
                    items.push(item);
                    j += 1;
                } else {
                    break;
                }
            }
            if !items.is_empty() {
                out.push(format!("{}: [{}]", key_name, items.join(", ")));
                i = j;
                continue;
            }
        }
        out.push(line.to_string());
        i += 1;
    }
    out.join("\n")
}

// ─── public API ──────────────────────────────────────────────────────────────

/// Normalize the in-memory content of a markdown file with YAML frontmatter.
/// Returns the normalized content. Idempotent.
pub fn normalize_str(content: &str) -> Result<String, NormalizeError> {
    // Doubled-frontmatter path first (with false-positive guard).
    if let Some((fm1_raw, gap_body, fm2_raw, rest)) = detect_doubled_frontmatter(content) {
        let fm1_pre = preprocess_fm(&fm1_raw);
        let fm2_pre = preprocess_fm(&fm2_raw);
        let parse1 = serde_yml::from_str::<serde_yml::Value>(&fm1_pre);
        let parse2 = serde_yml::from_str::<serde_yml::Value>(&fm2_pre);
        let mapping_pair = match (parse1, parse2) {
            (Ok(serde_yml::Value::Mapping(m1)), Ok(serde_yml::Value::Mapping(m2))) => Some((m1, m2)),
            _ => None,
        };
        if let Some((m1, m2)) = mapping_pair {
            let merged = merge_two_mappings(m1, m2);
            let (_, normalized) = normalize_mapping(merged);
            let emitted = serde_yml::to_string(&serde_yml::Value::Mapping(normalized))?;
            let emitted = emitted.trim_end_matches('\n').to_string();
            let emitted = block_to_flow_for_flow_keys(&emitted);
            let gap_clean = gap_body.trim_matches('\n');
            let new = if !gap_clean.is_empty() {
                format!(
                    "---\n{}\n---\n\n{}\n\n{}",
                    emitted,
                    gap_clean,
                    rest.trim_start_matches('\n')
                )
            } else {
                format!("---\n{}\n---\n{}", emitted, rest)
            };
            return Ok(new);
        }
        // fall through to single-frontmatter handling
    }

    let Some((fm_body, rest)) = split_frontmatter(content) else {
        return Ok(content.to_string());
    };
    let fm_body_fixed = preprocess_fm(&fm_body);
    let parsed: serde_yml::Value = serde_yml::from_str(&fm_body_fixed)?;
    let mapping = match parsed {
        serde_yml::Value::Mapping(m) => m,
        _ => return Ok(content.to_string()),
    };
    if mapping.is_empty() {
        return Ok(content.to_string());
    }
    let (_, normalized) = normalize_mapping(mapping);
    let emitted = serde_yml::to_string(&serde_yml::Value::Mapping(normalized))?;
    let emitted = emitted.trim_end_matches('\n').to_string();
    let emitted = block_to_flow_for_flow_keys(&emitted);
    let new = format!("---\n{}\n---\n{}", emitted, rest);
    Ok(new)
}

/// Merge two mappings using the same rules as the Python `merge_metadata`:
/// list-shaped keys are unioned (dedup, order preserved); scalar collisions
/// prefer `m2` (the second / inner block, typically the post's original FM).
fn merge_two_mappings(
    m1: serde_yml::Mapping,
    m2: serde_yml::Mapping,
) -> serde_yml::Mapping {
    const LIST_SHAPED: &[&str] = &[
        "tags",
        "relates",
        "related",
        "connectsTo",
        "connectTo",
        "connects",
        "backlinks",
        "references",
        "aliases",
        "context",
        "focus_areas",
    ];
    let mut out = serde_yml::Mapping::new();
    let mut keys_in_order: Vec<serde_yml::Value> = Vec::new();
    for (k, _) in m1.iter() {
        keys_in_order.push(k.clone());
    }
    for (k, _) in m2.iter() {
        if !keys_in_order.contains(k) {
            keys_in_order.push(k.clone());
        }
    }
    for k in keys_in_order {
        let v1 = m1.get(&k).cloned();
        let v2 = m2.get(&k).cloned();
        let key_str = k.as_str().map(|s| s.to_string()).unwrap_or_default();
        let is_list_shaped = LIST_SHAPED.contains(&key_str.as_str())
            || matches!(v1, Some(serde_yml::Value::Sequence(_)))
            || matches!(v2, Some(serde_yml::Value::Sequence(_)));
        if is_list_shaped {
            let mut items: Vec<serde_yml::Value> = Vec::new();
            let mut seen: Vec<serde_yml::Value> = Vec::new();
            for v in [v1, v2].into_iter().flatten() {
                match v {
                    serde_yml::Value::Sequence(seq) => {
                        for it in seq {
                            if !seen.contains(&it) {
                                seen.push(it.clone());
                                items.push(it);
                            }
                        }
                    }
                    other => {
                        if !seen.contains(&other) {
                            seen.push(other.clone());
                            items.push(other);
                        }
                    }
                }
            }
            out.insert(k, serde_yml::Value::Sequence(items));
        } else {
            let chosen = match (v2, v1) {
                (Some(v2), _) if !is_empty_value(&v2) => Some(v2),
                (_, Some(v1)) if !is_empty_value(&v1) => Some(v1),
                (Some(v2), _) => Some(v2),
                (_, Some(v1)) => Some(v1),
                (None, None) => None,
            };
            // Drop keys whose only candidate values are Null/empty-string —
            // emitting `key: null` or `key: ""` is noisier than dropping.
            if let Some(v) = chosen {
                if !is_empty_value(&v) {
                    out.insert(k, v);
                }
            }
        }
    }
    out
}

fn is_empty_value(v: &serde_yml::Value) -> bool {
    match v {
        serde_yml::Value::Null => true,
        serde_yml::Value::String(s) => s.is_empty(),
        _ => false,
    }
}

/// Normalize a file on disk. If `apply` is true, the file is rewritten when changed.
pub fn normalize_file(path: &Path, apply: bool) -> Result<NormalizeOutcome, NormalizeError> {
    let original = fs::read_to_string(path)?;
    let new = normalize_str(&original)?;
    if new == original {
        return Ok(NormalizeOutcome::Unchanged);
    }
    if apply {
        fs::write(path, &new)?;
    }
    Ok(NormalizeOutcome::Changed { new_content: new })
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_block_to_flow_tags() {
        let input = "---\ntitle: x\ntags:\n- foo\n- bar\n---\nbody\n";
        let out = normalize_str(input).unwrap();
        assert!(out.contains("tags: [foo, bar]"), "got: {}", out);
        assert!(!out.contains("- foo"), "block list not removed:\n{}", out);
    }

    #[test]
    fn test_block_to_flow_relates_wikilinks() {
        let input = "---\nrelates:\n- \"[[A]]\"\n- \"[[B]]\"\n---\nbody\n";
        let out = normalize_str(input).unwrap();
        assert!(
            out.contains("[[A]]") && out.contains("[[B]]") && out.contains("relates:"),
            "got: {}",
            out
        );
        // Should be a single-line flow list, not block list
        assert!(!out.contains("\n- "), "block list survived:\n{}", out);
    }

    #[test]
    fn test_key_rename_related_to_relates() {
        let input = "---\nrelated:\n- \"[[X]]\"\n---\nbody\n";
        let out = normalize_str(input).unwrap();
        assert!(out.contains("relates:"), "got: {}", out);
        assert!(!out.contains("related:"), "old key left:\n{}", out);
    }

    #[test]
    fn test_key_rename_collision_merge() {
        let input = "---\nrelated:\n- \"[[X]]\"\nrelates:\n- \"[[Y]]\"\n---\nbody\n";
        let out = normalize_str(input).unwrap();
        assert!(out.contains("[[X]]"), "missing X:\n{}", out);
        assert!(out.contains("[[Y]]"), "missing Y:\n{}", out);
        assert!(!out.contains("related:"), "old key left:\n{}", out);
        // single relates key only
        assert_eq!(out.matches("relates:").count(), 1, "duplicate relates:\n{}", out);
    }

    #[test]
    fn test_doubled_frontmatter_merge() {
        let input = "---\ntype: stamp\nproject: x\n---\n\n[week::W18]\n\n---\ntitle: real post\ntags:\n- t1\n---\nbody here\n";
        let out = normalize_str(input).unwrap();
        // Exactly ONE frontmatter close marker — regression guard against the
        // case where merge silently failed and four `---\n` survived.
        let fm_close_count = out.matches("\n---\n").count();
        assert_eq!(
            fm_close_count, 1,
            "expected exactly one FM block, found {}; output:\n{}",
            fm_close_count, out
        );
        assert!(out.contains("[week::W18]"), "gap body lost:\n{}", out);
        assert!(out.contains("title: real post"), "title missing:\n{}", out);
    }

    #[test]
    fn test_doubled_frontmatter_false_positive_horizontal_rule() {
        // Second `---` looks like a horizontal rule and the content after it is not a YAML mapping.
        let input = "---\ntitle: hi\n---\n\nsome prose\n\n---\n\nmore prose, not a mapping\n";
        let out = normalize_str(input).unwrap();
        assert!(out.contains("some prose"), "lost body:\n{}", out);
        assert!(out.contains("more prose"), "lost body 2:\n{}", out);
    }

    #[test]
    fn test_bare_wikilink_list_inline() {
        let input = "---\ntitle: t\nrelates: [[A]], [[B]]\n---\nbody\n";
        let out = normalize_str(input).unwrap();
        assert!(
            out.contains("[[A]]") && out.contains("[[B]]"),
            "got: {}",
            out
        );
    }

    #[test]
    fn test_bare_wikilink_list_whitespace_separator() {
        let input = "---\ntitle: t\nrelates: [[A]] [[B]] [[C]]\n---\nbody\n";
        let out = normalize_str(input).unwrap();
        assert!(out.contains("[[A]]"), "got: {}", out);
        assert!(out.contains("[[C]]"), "got: {}", out);
    }

    #[test]
    fn test_bare_wikilink_block_item() {
        let input = "---\nrelates:\n  - [[A]]\n  - [[B]]\n---\nbody\n";
        let out = normalize_str(input).unwrap();
        assert!(
            out.contains("[[A]]") && out.contains("[[B]]"),
            "got: {}",
            out
        );
    }

    #[test]
    fn test_colon_in_scalar() {
        let input = "---\ntitle: Dispatch: BBS Sweep\n---\nbody\n";
        let out = normalize_str(input).unwrap();
        assert!(out.contains("Dispatch: BBS Sweep"), "got: {}", out);
        assert!(
            out.contains("title: \"Dispatch: BBS Sweep\"") || out.contains("title: 'Dispatch: BBS Sweep'"),
            "value not quoted:\n{}",
            out
        );
    }

    #[test]
    fn test_idempotent_no_op_on_clean_file() {
        let input = "---\ntitle: clean\ntags: [a, b]\nrelates: [\"[[X]]\"]\n---\nbody\n";
        let once = normalize_str(input).unwrap();
        let twice = normalize_str(&once).unwrap();
        assert_eq!(once, twice, "not idempotent:\nfirst:\n{}\nsecond:\n{}", once, twice);
    }

    #[test]
    fn test_no_frontmatter_passthrough() {
        let input = "no frontmatter here\njust body";
        let out = normalize_str(input).unwrap();
        assert_eq!(out, input);
    }

    #[test]
    fn test_all_canonical_renames() {
        for old in &["related", "connectsTo", "connectTo", "connects", "backlinks", "references"] {
            let input = format!(
                "---\ntitle: t\n{}:\n  - \"[[X]]\"\n  - \"[[Y]]\"\n---\nbody\n",
                old
            );
            let out = normalize_str(&input).expect("normalize should succeed");
            assert!(
                out.contains("relates:"),
                "rename to relates: missing for key '{}'; output:\n{}",
                old, out
            );
            assert!(
                !out.contains(&format!("\n{}:", old)),
                "old key '{}' leaked into output; full:\n{}",
                old, out
            );
            // Both wikilinks survived
            assert!(
                out.contains("[[X]]") && out.contains("[[Y]]"),
                "wikilink content lost during rename of '{}'", old
            );
        }
    }

    #[test]
    fn test_doubled_frontmatter_shack_pattern_false_positive() {
        // Body starts with `---` horizontal rule followed by key-shaped marker
        // and key-value lines. Second `---` is a horizontal rule deeper down.
        // This is the shack-tui-daily-widget.md shape — single FM, body abuses ---.
        let input = "---\ntitle: real\n---\n\n[week::W02]\ntitle: TUI Daily\ngithub: https://example.com\npreview: https://example.com\n---\n\nMore body content\n";
        let out = normalize_str(input).expect("normalize should succeed on shack pattern");
        // Title should remain "real" (NOT replaced by the body's "TUI Daily")
        assert!(out.contains("title: real"), "merged false-positive; title overwritten:\n{}", out);
        assert!(out.contains("https://example.com"), "lost body content:\n{}", out);
    }

    #[test]
    fn test_parse_error_propagates() {
        // Genuinely malformed YAML — unbalanced brackets, can't be parsed
        // even after preprocessing. normalize_str must return Err, NOT swallow
        // it as a silent passthrough. write_with_frontmatter relies on this.
        let input = "---\ntitle: [unclosed bracket\nfoo: bar\n---\nbody\n";
        let result = normalize_str(input);
        assert!(
            matches!(result, Err(NormalizeError::ParseYaml(_))),
            "expected ParseYaml error, got: {:?}",
            result
        );
    }

    #[test]
    fn test_empty_value_collision_drops_key() {
        // Doubled-FM where both blocks have a scalar key with empty/null value.
        // Should not emit `key: null` or `key: ""` — drop the key entirely.
        let input = "---\ntitle: real\nstatus:\n---\n\n[week::W18]\n\n---\nproject: x\nstatus: \"\"\n---\nbody\n";
        let out = normalize_str(input).expect("normalize should succeed");
        assert!(
            !out.contains("status: null") && !out.contains("status: ~") && !out.contains("status: ''") && !out.contains("status: \"\""),
            "empty-value collision left noise:\n{}",
            out
        );
        // Title and project should still be present (real values, kept)
        assert!(out.contains("title: real"), "real value dropped:\n{}", out);
        assert!(out.contains("project: x"), "real value dropped:\n{}", out);
    }

    #[test]
    fn test_normalize_file_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.md");
        std::fs::write(&path, "---\nrelated:\n- \"[[A]]\"\n---\nbody\n").unwrap();
        let outcome = normalize_file(&path, true).unwrap();
        assert!(matches!(outcome, NormalizeOutcome::Changed { .. }));
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(after.contains("relates:"), "got: {}", after);
        let outcome2 = normalize_file(&path, true).unwrap();
        assert_eq!(outcome2, NormalizeOutcome::Unchanged);
    }
}
