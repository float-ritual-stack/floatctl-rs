use std::collections::BTreeSet;
use std::fmt;
use std::iter::FromIterator;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

static MARKER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)(ctx::[^\s]+|project::[^\s]+|float\.[^\s]+|lf1m::[^\s]+|karen::[^\s]+|sysop::[^\s]+|qtb::[^\s]+|httm::[^\s]+)",
    )
    .expect("marker regex")
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
    for capture in MARKER_RE.captures_iter(input) {
        if let Some(m) = capture.get(0) {
            set.insert(m.as_str());
        }
    }
    set
}
