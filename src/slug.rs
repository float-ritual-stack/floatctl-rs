use std::collections::HashMap;

use crate::model::Conversation;

const MAX_SLUG_LEN: usize = 80;

#[derive(Default)]
pub struct SlugState {
    counts: HashMap<String, usize>,
}

impl SlugState {
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }

    pub fn next_slug(&mut self, base: &str) -> String {
        let entry = self.counts.entry(base.to_string()).or_insert(0);
        let slug = if *entry == 0 {
            base.to_string()
        } else {
            format!("{base}-{entry:03}")
        };
        *entry += 1;
        slug
    }
}

pub fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;

    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if ch.is_ascii() {
            if !slug.is_empty() && !last_was_dash {
                slug.push('-');
                last_was_dash = true;
            }
        }
        // Non-ASCII characters are skipped entirely.
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.len() > MAX_SLUG_LEN {
        slug.truncate(MAX_SLUG_LEN);
        while slug.ends_with('-') {
            slug.pop();
        }
    }

    slug
}

pub fn strip_leading_date(input: &str) -> String {
    let trimmed = input.trim_start();
    if trimmed.len() < 10 {
        return trimmed.to_string();
    }

    let bytes = trimmed.as_bytes();
    if bytes.len() < 10 || !bytes[..10].iter().all(|b| b.is_ascii()) {
        return trimmed.to_string();
    }
    if !trimmed.is_char_boundary(10) {
        return trimmed.to_string();
    }

    let candidate = &trimmed[..10];
    if !is_date_prefix(candidate) {
        return trimmed.to_string();
    }

    let remainder = trimmed[10..]
        .trim_start_matches([' ', '-', '_'])
        .trim_start();
    remainder.to_string()
}

fn is_date_prefix(candidate: &str) -> bool {
    candidate.len() == 10
        && candidate.chars().enumerate().all(|(idx, ch)| match idx {
            4 | 7 => ch == '-',
            _ => ch.is_ascii_digit(),
        })
}

pub fn conversation_base_name(conversation: &Conversation) -> String {
    let title = conversation.title.as_deref().unwrap_or("").trim();
    if title.is_empty() {
        return "untitled".to_string();
    }

    let stripped = strip_leading_date(title);
    let trimmed = stripped.trim();
    if trimmed.is_empty() {
        "untitled".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_leading_date_variants() {
        assert_eq!(strip_leading_date("2025-01-02 Foo"), "Foo");
        assert_eq!(strip_leading_date("2025-01-02_Foo"), "Foo");
        assert_eq!(strip_leading_date("2025-01-02-Foo"), "Foo");
        assert_eq!(strip_leading_date("No date here"), "No date here");
    }

    #[test]
    fn slugify_basic_cases() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("emoji 😀 test"), "emoji-test");
        assert_eq!(slugify("foo/bar\\baz"), "foo-bar-baz");
    }

    #[test]
    fn slugify_truncates_and_cleans() {
        let long = "a".repeat(100);
        let slug = slugify(&long);
        assert_eq!(slug.len(), MAX_SLUG_LEN);
        assert!(slug.chars().all(|c| c == 'a'));
    }
}
