//! Query sorts value object representation

use std::fmt::Display;

/// Filter sort field
pub type QuerySortField = String;

/// Filter sort direction (ASC or DESC)
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum QuerySortDirection {
    /// Ascending sort (`'+'` prefix)
    /// Example: `?sort=+id`
    #[default]
    Asc,

    /// Descending sort (`'-'` prefix)
    /// Example: `?sort=-name`
    Desc,
}

impl Display for QuerySortDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Asc => "ASC",
                Self::Desc => "DESC",
            }
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuerySort {
    pub field: QuerySortField,
    pub direction: QuerySortDirection,
}

impl QuerySort {
    /// Create a new query sort
    pub fn new(field: QuerySortField, direction: QuerySortDirection) -> Self {
        Self { field, direction }
    }
}

/// Filter sorts
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuerySorts(pub Vec<QuerySort>);

impl From<&str> for QuerySorts {
    /// Create a new query sorting from a string
    ///
    /// # Example
    /// ```
    /// use api_tools::value_objects::query_sort::{QuerySort, QuerySortDirection, QuerySorts};
    ///
    /// let sorts = QuerySorts::from("+id,-name");
    /// assert_eq!(
    ///     sorts.0,
    ///     vec![
    ///         QuerySort {
    ///             field: "id".to_string(),
    ///             direction: QuerySortDirection::Asc
    ///         },
    ///         QuerySort {
    ///             field: "name".to_string(),
    ///             direction: QuerySortDirection::Desc
    ///         },
    ///     ]
    /// );
    /// ```
    fn from(value: &str) -> Self {
        let mut sorts = Vec::new();
        let parts = value.split(',');

        for part in parts {
            let prefix = part.chars().next();
            if let Some(prefix) = prefix {
                if prefix == '+' {
                    sorts.push(QuerySort::new(part[1..].to_string(), QuerySortDirection::Asc));
                } else if prefix == '-' {
                    sorts.push(QuerySort::new(part[1..].to_string(), QuerySortDirection::Desc));
                }
            }
        }

        Self(sorts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_sort_direction_default() {
        assert_eq!(QuerySortDirection::default(), QuerySortDirection::Asc);
    }

    #[test]
    fn test_query_sort_direction_display() {
        assert_eq!("ASC", QuerySortDirection::Asc.to_string());
        assert_eq!("DESC", QuerySortDirection::Desc.to_string());
    }

    #[test]
    fn test_filter_sorts_from_str() {
        let sorts = QuerySorts::from("");
        assert!(sorts.0.is_empty());

        let sorts = QuerySorts::from("+id,-name");
        assert_eq!(sorts.0.len(), 2);
        assert_eq!(
            sorts.0,
            vec![
                QuerySort {
                    field: "id".to_string(),
                    direction: QuerySortDirection::Asc
                },
                QuerySort {
                    field: "name".to_string(),
                    direction: QuerySortDirection::Desc
                },
            ]
        );

        let sorts = QuerySorts::from("id");
        assert!(sorts.0.is_empty());
    }
}
