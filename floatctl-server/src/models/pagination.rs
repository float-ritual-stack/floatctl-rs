//! Pagination types - Spec 1.3

use serde::{Deserialize, Serialize};

/// Maximum items per page
const MAX_PER_PAGE: u32 = 100;

/// Default items per page
const DEFAULT_PER_PAGE: u32 = 20;

/// Pagination parameters
#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    /// Page number (1-indexed)
    pub page: u32,
    /// Items per page (max 100)
    pub per_page: u32,
}

impl Pagination {
    /// Create pagination with validation.
    ///
    /// - Page is clamped to minimum of 1
    /// - Per page is clamped to 1..=100
    pub fn new(page: u32, per_page: u32) -> Self {
        Self {
            page: page.max(1),
            per_page: per_page.clamp(1, MAX_PER_PAGE),
        }
    }

    /// Calculate SQL OFFSET value.
    pub fn offset(&self) -> u64 {
        ((self.page - 1) * self.per_page) as u64
    }

    /// Get LIMIT value.
    pub fn limit(&self) -> u32 {
        self.per_page
    }
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: DEFAULT_PER_PAGE,
        }
    }
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paginated<T> {
    /// Items for current page
    pub items: Vec<T>,
    /// Total count across all pages
    pub total: i64,
    /// Current page number
    pub page: u32,
    /// Items per page
    pub per_page: u32,
}

impl<T> Paginated<T> {
    /// Calculate total number of pages.
    pub fn total_pages(&self) -> u32 {
        if self.total == 0 {
            1
        } else {
            ((self.total as u32 + self.per_page - 1) / self.per_page).max(1)
        }
    }

    /// Check if there's a next page.
    pub fn has_next(&self) -> bool {
        self.page < self.total_pages()
    }

    /// Check if there's a previous page.
    pub fn has_prev(&self) -> bool {
        self.page > 1
    }
}

/// Query parameters for pagination
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

impl From<PaginationParams> for Pagination {
    fn from(params: PaginationParams) -> Self {
        Self::new(
            params.page.unwrap_or(1),
            params.per_page.unwrap_or(DEFAULT_PER_PAGE),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_calculation() {
        let p = Pagination::new(1, 10);
        assert_eq!(p.offset(), 0);

        let p = Pagination::new(2, 10);
        assert_eq!(p.offset(), 10);

        let p = Pagination::new(3, 25);
        assert_eq!(p.offset(), 50);
    }

    #[test]
    fn clamps_page() {
        let p = Pagination::new(0, 10);
        assert_eq!(p.page, 1);
    }

    #[test]
    fn clamps_per_page() {
        let p = Pagination::new(1, 0);
        assert_eq!(p.per_page, 1);

        let p = Pagination::new(1, 999);
        assert_eq!(p.per_page, 100);
    }

    #[test]
    fn total_pages() {
        let paginated: Paginated<()> = Paginated {
            items: vec![],
            total: 0,
            page: 1,
            per_page: 10,
        };
        assert_eq!(paginated.total_pages(), 1);

        let paginated: Paginated<()> = Paginated {
            items: vec![],
            total: 25,
            page: 1,
            per_page: 10,
        };
        assert_eq!(paginated.total_pages(), 3);

        let paginated: Paginated<()> = Paginated {
            items: vec![],
            total: 100,
            page: 1,
            per_page: 10,
        };
        assert_eq!(paginated.total_pages(), 10);
    }

    #[test]
    fn has_next_prev() {
        let paginated: Paginated<()> = Paginated {
            items: vec![],
            total: 30,
            page: 1,
            per_page: 10,
        };
        assert!(paginated.has_next());
        assert!(!paginated.has_prev());

        let paginated: Paginated<()> = Paginated {
            items: vec![],
            total: 30,
            page: 2,
            per_page: 10,
        };
        assert!(paginated.has_next());
        assert!(paginated.has_prev());

        let paginated: Paginated<()> = Paginated {
            items: vec![],
            total: 30,
            page: 3,
            per_page: 10,
        };
        assert!(!paginated.has_next());
        assert!(paginated.has_prev());
    }
}
