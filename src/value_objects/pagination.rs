//! Pagination value object representation

/// Pagination min limit
pub const PAGINATION_MIN_LIMIT: u32 = 10;

/// Pagination max limit
pub const PAGINATION_MAX_LIMIT: u32 = 500;

/// Pagination default limit
pub const PAGINATION_DEFAULT_LIMIT: u32 = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pagination {
    page: u32,
    limit: u32,
    max_limit: Option<u32>,
}

impl Pagination {
    /// Create new pagination
    ///
    /// `max_limit` is optional and will be clamped between `PAGINATION_MIN_LIMIT` and `PAGINATION_MAX_LIMIT`
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
    /// ```
    pub fn new(page: u32, limit: u32, max_limit: Option<u32>) -> Self {
        let page = if page == 0 { 1 } else { page };

        let mut max = max_limit.unwrap_or(PAGINATION_MAX_LIMIT);

        if max > PAGINATION_MAX_LIMIT {
            max = PAGINATION_MAX_LIMIT;
        }

        let limit = if limit > max {
            max
        } else if limit < PAGINATION_MIN_LIMIT {
            PAGINATION_MIN_LIMIT
        } else {
            limit
        };

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
    pub fn set_max_limit(&mut self, max_limit: u32) {
        let max_limit = max_limit.clamp(PAGINATION_MIN_LIMIT, PAGINATION_MAX_LIMIT);
        self.max_limit = Some(max_limit);
    }
}

impl Default for Pagination {
    /// Default pagination
    fn default() -> Self {
        let default = if PAGINATION_DEFAULT_LIMIT > PAGINATION_MAX_LIMIT {
            PAGINATION_MAX_LIMIT
        } else {
            PAGINATION_DEFAULT_LIMIT
        };
        Self::new(1, default, None)
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

        pagination.set_max_limit(20);
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
}
