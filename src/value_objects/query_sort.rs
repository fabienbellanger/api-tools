//! Query sorts value object representation

use std::fmt::Display;

/// Filter sort field
pub type QuerySortField = String;

/// Filter sort direction (ASC or DESC)
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum QuerySortDirection {
    /// Ascending sort (`'+'` prefix or no prefix)
    /// Example: `?sort=id` or `?sort=%2Bid`
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
///
/// Field names come straight from the request: check them against an allowlist before using them
/// in a query (e.g. in an SQL `ORDER BY`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuerySorts(pub Vec<QuerySort>);

impl From<&str> for QuerySorts {
    /// Create a new query sorting from a string
    ///
    /// Parts are separated by `,` and trimmed. A `-` prefix sorts descending; a `+` prefix or no
    /// prefix sorts ascending. In a query string, a raw `+` is decoded as a space, so `?sort=+id`
    /// arrives as `" id"` and is still read as `id` ascending. Empty fields are ignored.
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
        let sorts = value
            .split(',')
            .map(str::trim)
            .filter_map(|part| {
                let (field, direction) = match part.strip_prefix('-') {
                    Some(field) => (field, QuerySortDirection::Desc),
                    None => (part.strip_prefix('+').unwrap_or(part), QuerySortDirection::Asc),
                };
                let field = field.trim();

                (!field.is_empty()).then(|| QuerySort::new(field.to_string(), direction))
            })
            .collect();

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
        assert_eq!(sorts.0, vec![QuerySort::new("id".to_string(), QuerySortDirection::Asc)]);
    }

    #[test]
    fn test_query_sort_new() {
        let sort = QuerySort::new("name".to_string(), QuerySortDirection::Desc);
        assert_eq!(sort.field, "name");
        assert_eq!(sort.direction, QuerySortDirection::Desc);
    }

    #[test]
    fn test_query_sort_default() {
        let sort = QuerySort::default();
        assert_eq!(sort.field, "");
        assert_eq!(sort.direction, QuerySortDirection::Asc);
    }

    #[test]
    fn test_filter_sorts_from_str_without_prefix_is_ascending() {
        let sorts = QuerySorts::from("+id,name,,-created_at");
        assert_eq!(
            sorts,
            QuerySorts(vec![
                QuerySort::new("id".to_string(), QuerySortDirection::Asc),
                QuerySort::new("name".to_string(), QuerySortDirection::Asc),
                QuerySort::new("created_at".to_string(), QuerySortDirection::Desc),
            ])
        );
    }

    #[test]
    fn test_filter_sorts_from_str_keeps_order_and_duplicates() {
        let sorts = QuerySorts::from("-b,+a,-b");
        let fields: Vec<_> = sorts.0.iter().map(|s| s.field.as_str()).collect();
        assert_eq!(fields, ["b", "a", "b"]);
    }

    /// Fields are sliced after a one-byte ASCII prefix, so non-ASCII field
    /// names must not panic.
    #[test]
    fn test_filter_sorts_from_str_with_non_ascii_field() {
        let sorts = QuerySorts::from("-prénom,é");
        assert_eq!(
            sorts.0,
            vec![
                QuerySort::new("prénom".to_string(), QuerySortDirection::Desc),
                QuerySort::new("é".to_string(), QuerySortDirection::Asc),
            ]
        );
    }

    #[test]
    fn test_filter_sorts_from_str_trims_whitespace() {
        let sorts = QuerySorts::from(" +id , -name ,  date ");
        assert_eq!(
            sorts.0,
            vec![
                QuerySort::new("id".to_string(), QuerySortDirection::Asc),
                QuerySort::new("name".to_string(), QuerySortDirection::Desc),
                QuerySort::new("date".to_string(), QuerySortDirection::Asc),
            ]
        );
    }

    #[test]
    fn test_filter_sorts_from_str_trims_whitespace_after_prefix() {
        let sorts = QuerySorts::from("- name,+ id");
        assert_eq!(
            sorts.0,
            vec![
                QuerySort::new("name".to_string(), QuerySortDirection::Desc),
                QuerySort::new("id".to_string(), QuerySortDirection::Asc),
            ]
        );
    }

    #[test]
    fn test_filter_sorts_from_str_ignores_empty_fields() {
        assert!(QuerySorts::from("+,-, ,,").0.is_empty());
        assert!(QuerySorts::from("   ").0.is_empty());
    }

    /// A raw `+` in a query string is decoded as a space: `?sort=+id,-name`
    /// must still sort on both fields.
    #[test]
    fn test_filter_sorts_from_url_decoded_query() {
        #[derive(serde::Deserialize)]
        struct Params {
            sort: String,
        }

        let params: Params = serde_urlencoded::from_str("sort=+id,-name").unwrap();
        assert_eq!(params.sort, " id,-name");
        assert_eq!(
            QuerySorts::from(params.sort.as_str()).0,
            vec![
                QuerySort::new("id".to_string(), QuerySortDirection::Asc),
                QuerySort::new("name".to_string(), QuerySortDirection::Desc),
            ]
        );

        // Percent-encoded `+` keeps working.
        let params: Params = serde_urlencoded::from_str("sort=%2Bid").unwrap();
        assert_eq!(
            QuerySorts::from(params.sort.as_str()).0,
            vec![QuerySort::new("id".to_string(), QuerySortDirection::Asc)]
        );
    }

    /// Only the first `-` is a prefix: the rest belongs to the field name.
    #[test]
    fn test_filter_sorts_from_str_keeps_inner_dashes() {
        assert_eq!(
            QuerySorts::from("--created-at").0,
            vec![QuerySort::new("-created-at".to_string(), QuerySortDirection::Desc)]
        );
    }
}
