use std::collections::BTreeSet;
use std::fmt;
use std::iter::FromIterator;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Matches `ctx::` markers with their full timestamp format:
///   ctx::2026-03-21 @ 10:10:38 PM [project::X] [mode::Y] summary text
/// The ctx:: marker captures through end of line since the whole line is the marker.
static CTX_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)ctx::[^\n]+").expect("ctx regex")
});

/// Matches bracketed markers like [project::floatctl-rs], [mode::digest], [session::abc123]
static BRACKET_MARKER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[([a-zA-Z_][a-zA-Z0-9_]*::[^\]]+)\]").expect("bracket marker regex")
});

/// Matches bare word::value markers.
/// We strip backticks and code fences before applying this, so no lookaround needed.
/// Captures group 1: the full marker (word::value).
static BARE_MARKER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:^|[\s\[(\-])([a-zA-Z_][a-zA-Z0-9_]*::[^\s,\]\)]+)")
        .expect("bare marker regex")
});

/// Matches code fences to skip their contents during marker extraction
static CODE_FENCE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?ms)^```.*?^```").expect("code fence regex")
});

/// Matches inline code to skip during marker extraction
static INLINE_CODE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"`[^`]+`").expect("inline code regex")
});

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarkerSet {
    items: BTreeSet<String>,
}

impl MarkerSet {
    pub fn insert(&mut self, marker: &str) {
        self.items.insert(marker.to_ascii_lowercase());
    }

    pub fn extend(&mut self, other: &MarkerSet) {
        self.items.extend(other.items.iter().cloned());
    }

    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.items.iter()
    }
}

impl fmt::Debug for MarkerSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.items.iter()).finish()
    }
}

impl FromIterator<String> for MarkerSet {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        let mut set = MarkerSet::default();
        for item in iter {
            set.insert(&item);
        }
        set
    }
}

pub fn extract_markers(input: &str) -> MarkerSet {
    let mut set = MarkerSet::default();

    // Strip code fences and inline code so we don't extract markers from code
    let stripped = CODE_FENCE_RE.replace_all(input, " ");
    let stripped = INLINE_CODE_RE.replace_all(&stripped, " ");

    // 1. Extract full ctx:: lines (these contain embedded markers + timestamp + summary)
    for m in CTX_RE.find_iter(&stripped) {
        set.insert(m.as_str().trim());
    }

    // 2. Extract bracketed markers [project::X], [mode::Y], etc.
    for caps in BRACKET_MARKER_RE.captures_iter(&stripped) {
        if let Some(m) = caps.get(1) {
            set.insert(m.as_str());
        }
    }

    // 3. Extract bare word::value markers (skip ctx:: since we already grabbed the full line)
    for caps in BARE_MARKER_RE.captures_iter(&stripped) {
        if let Some(m) = caps.get(1) {
            let marker = m.as_str();
            if !marker.starts_with("ctx::") {
                set.insert(marker);
            }
        }
    }

    set
}
