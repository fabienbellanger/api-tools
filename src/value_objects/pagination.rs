//! Pagination value object representation

/// Pagination min limit
pub const PAGINATION_MIN_LIMIT: u32 = 10;

/// Pagination max limit
pub const PAGINATION_MAX_LIMIT: u32 = 500;

/// Pagination default limit
pub const PAGINATION_DEFAULT_LIMIT: u32 = 200;

// The default limit must be a valid limit: checked at compile time.
const _: () =
    assert!(PAGINATION_MIN_LIMIT <= PAGINATION_DEFAULT_LIMIT && PAGINATION_DEFAULT_LIMIT <= PAGINATION_MAX_LIMIT);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pagination {
    page: u32,
    limit: u32,
    max_limit: Option<u32>,
}

impl Pagination {
    /// Create new pagination
    ///
    /// `max_limit` is optional and will be clamped between `PAGINATION_MIN_LIMIT` and `PAGINATION_MAX_LIMIT`,
    /// so `limit` always ends up between `PAGINATION_MIN_LIMIT` and `PAGINATION_MAX_LIMIT`.
    ///
    /// # Examples
    ///
    /// ```
    /// use api_tools::value_objects::pagination::{Pagination, PAGINATION_MAX_LIMIT, PAGINATION_MIN_LIMIT};
    ///
    /// let pagination = Pagination::new(1, 100, None);
    /// assert_eq!(pagination.page(), 1);
    /// assert_eq!(pagination.limit(), 100);
    ///
    /// // Invalid page
    /// let pagination = Pagination::new(0, 100, None);
    /// assert_eq!(pagination.page(), 1);
    /// assert_eq!(pagination.limit(), 100);
    ///
    /// // Limit too small
    /// let pagination = Pagination::new(2, 10, None);
    /// assert_eq!(pagination.page(), 2);
    /// assert_eq!(pagination.limit(), PAGINATION_MIN_LIMIT);
    ///
    /// // Limit too big
    /// let pagination = Pagination::new(2, 1_000, None);
    /// assert_eq!(pagination.page(), 2);
    /// assert_eq!(pagination.limit(), PAGINATION_MAX_LIMIT);
    ///
    /// // Limit too big and max limit greater than max
    /// let pagination = Pagination::new(2, 1_000, Some(800));
    /// assert_eq!(pagination.page(), 2);
    /// assert_eq!(pagination.limit(), PAGINATION_MAX_LIMIT);
    ///
    /// // Max limit smaller than min
    /// let pagination = Pagination::new(2, 100, Some(5));
    /// assert_eq!(pagination.limit(), PAGINATION_MIN_LIMIT);
    /// ```
    pub fn new(page: u32, limit: u32, max_limit: Option<u32>) -> Self {
        let page = page.max(1);
        let max_limit = max_limit.map(|max| max.clamp(PAGINATION_MIN_LIMIT, PAGINATION_MAX_LIMIT));
        let limit = limit.clamp(PAGINATION_MIN_LIMIT, max_limit.unwrap_or(PAGINATION_MAX_LIMIT));

        Self { page, limit, max_limit }
    }

    /// Get page
    pub fn page(&self) -> u32 {
        self.page
    }

    /// Get limit
    pub fn limit(&self) -> u32 {
        self.limit
    }

    /// Set a max limit (between `PAGINATION_MIN_LIMIT` and `PAGINATION_MAX_LIMIT`)
    ///
    /// The current `limit` is lowered if it exceeds the new max limit. It is never raised: the
    /// original requested value is not kept.
    ///
    /// # Example
    ///
    /// ```
    /// use api_tools::value_objects::pagination::Pagination;
    ///
    /// let mut pagination = Pagination::new(1, 200, None);
    /// pagination.set_max_limit(50);
    /// assert_eq!(pagination.limit(), 50);
    /// ```
    pub fn set_max_limit(&mut self, max_limit: u32) {
        let max_limit = max_limit.clamp(PAGINATION_MIN_LIMIT, PAGINATION_MAX_LIMIT);
        self.max_limit = Some(max_limit);
        self.limit = self.limit.min(max_limit);
    }
}

impl Default for Pagination {
    /// Default pagination
    fn default() -> Self {
        Self::new(1, PAGINATION_DEFAULT_LIMIT, None)
    }
}

/// Pagination for response
#[derive(Debug, Clone, PartialEq)]

pub struct PaginationResponse {
    pub page: u32,
    pub limit: u32,
    pub total: i64,
}

impl PaginationResponse {
    /// Create a new pagination response
    pub fn new(page: u32, limit: u32, total: i64) -> Self {
        Self { page, limit, total }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_set_max_limit() {
        let mut pagination = Pagination::default();
        assert_eq!(pagination.max_limit, None);

        pagination.set_max_limit(100);
        assert_eq!(pagination.max_limit, Some(100));

        pagination.set_max_limit(300);
        assert_eq!(pagination.max_limit, Some(300));

        pagination.set_max_limit(5);
        assert_eq!(pagination.max_limit, Some(PAGINATION_MIN_LIMIT));

        pagination.set_max_limit(600);
        assert_eq!(pagination.max_limit, Some(PAGINATION_MAX_LIMIT));
    }

    #[test]
    fn test_default() {
        let pagination = Pagination::default();
        assert_eq!(pagination.page(), 1);
        assert_eq!(pagination.limit(), PAGINATION_DEFAULT_LIMIT);
        assert_eq!(pagination.max_limit, None);
    }

    #[test]
    fn new_clamps_zero_page_to_one() {
        let p = Pagination::new(0, 100, None);
        assert_eq!(p.page(), 1);
    }

    #[test]
    fn new_clamps_limit_below_min_to_pagination_min_limit() {
        let p = Pagination::new(1, 1, None);
        assert_eq!(p.limit(), PAGINATION_MIN_LIMIT);
    }

    #[test]
    fn new_clamps_limit_above_max_to_pagination_max_limit() {
        let p = Pagination::new(1, PAGINATION_MAX_LIMIT + 100, None);
        assert_eq!(p.limit(), PAGINATION_MAX_LIMIT);
    }

    #[test]
    fn new_caps_max_limit_argument_at_pagination_max_limit() {
        // A custom max_limit greater than PAGINATION_MAX_LIMIT must itself be
        // capped at PAGINATION_MAX_LIMIT before clamping the limit value.
        let p = Pagination::new(1, 10_000, Some(10_000));
        assert_eq!(p.limit(), PAGINATION_MAX_LIMIT);
    }

    #[test]
    fn new_respects_a_smaller_custom_max_limit() {
        let p = Pagination::new(1, 1_000, Some(100));
        assert_eq!(p.limit(), 100);
    }

    #[test]
    fn pagination_response_new_stores_all_fields() {
        let r = PaginationResponse::new(2, 50, 1_234);
        assert_eq!(r.page, 2);
        assert_eq!(r.limit, 50);
        assert_eq!(r.total, 1_234);
    }

    #[test]
    fn new_keeps_limit_at_exact_bounds() {
        assert_eq!(
            Pagination::new(1, PAGINATION_MIN_LIMIT, None).limit(),
            PAGINATION_MIN_LIMIT
        );
        assert_eq!(
            Pagination::new(1, PAGINATION_MAX_LIMIT, None).limit(),
            PAGINATION_MAX_LIMIT
        );
        assert_eq!(Pagination::new(1, 100, Some(100)).limit(), 100);
    }

    #[test]
    fn new_keeps_valid_page_and_limit() {
        let p = Pagination::new(7, 42, None);
        assert_eq!(p.page(), 7);
        assert_eq!(p.limit(), 42);
    }

    #[test]
    fn new_stores_given_max_limit() {
        assert_eq!(Pagination::new(1, 50, Some(100)).max_limit, Some(100));
        assert_eq!(Pagination::new(1, 50, None).max_limit, None);
    }

    #[test]
    fn default_limit_is_within_bounds() {
        let limit = Pagination::default().limit();
        assert!((PAGINATION_MIN_LIMIT..=PAGINATION_MAX_LIMIT).contains(&limit));
    }

    #[test]
    fn new_clamps_max_limit_below_min_to_pagination_min_limit() {
        let p = Pagination::new(1, 100, Some(5));
        assert_eq!(p.limit(), PAGINATION_MIN_LIMIT);
        assert_eq!(p.max_limit, Some(PAGINATION_MIN_LIMIT));

        let p = Pagination::new(1, 100, Some(0));
        assert_eq!(p.limit(), PAGINATION_MIN_LIMIT);
    }

    #[test]
    fn new_stores_clamped_max_limit() {
        assert_eq!(
            Pagination::new(1, 50, Some(10_000)).max_limit,
            Some(PAGINATION_MAX_LIMIT)
        );
    }

    #[test]
    fn new_limit_is_always_within_bounds() {
        for max_limit in [None, Some(0), Some(5), Some(10), Some(100), Some(500), Some(u32::MAX)] {
            for limit in [0, 1, 10, 100, 500, 1_000, u32::MAX] {
                let p = Pagination::new(1, limit, max_limit);
                assert!(
                    (PAGINATION_MIN_LIMIT..=PAGINATION_MAX_LIMIT).contains(&p.limit()),
                    "limit={limit} max_limit={max_limit:?} gave {}",
                    p.limit()
                );
            }
        }
    }

    #[test]
    fn set_max_limit_lowers_a_greater_limit() {
        let mut p = Pagination::new(1, 200, None);
        p.set_max_limit(50);
        assert_eq!(p.limit(), 50);
    }

    #[test]
    fn set_max_limit_keeps_a_smaller_limit() {
        let mut p = Pagination::new(1, 20, None);
        p.set_max_limit(50);
        assert_eq!(p.limit(), 20);
    }

    #[test]
    fn set_max_limit_never_raises_the_limit() {
        let mut p = Pagination::new(1, 200, None);
        p.set_max_limit(50);
        p.set_max_limit(300);
        assert_eq!(p.limit(), 50);
        assert_eq!(p.max_limit, Some(300));
    }

    #[test]
    fn set_max_limit_below_min_lowers_limit_to_min() {
        let mut p = Pagination::new(1, 200, None);
        p.set_max_limit(1);
        assert_eq!(p.limit(), PAGINATION_MIN_LIMIT);
    }
}
