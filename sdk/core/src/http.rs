//! Definition of generic models used throughout different APIs

use borderless_id_types::TxIdentifier;
use queries::Pagination;
use serde::Serialize;

use crate::contract::{Description, Info, Metadata};
use crate::events::CallAction;

/// Default return type for all routes that return lists.
///
/// Since we never want to have an infinitely large list returned from an endpoint,
/// the number of entries an endpoint returns by default is limited.
///
/// However, the user must know, how many elements there are in total,
/// as this information is crucial for building pagination elements in a frontend.
///
/// This type serves as a wrapper around `Vec<T>` (which will be serialized to a list in json),
/// that also includes how many elements there are in total.
#[derive(Serialize)]
pub struct PaginatedElements<T>
where
    T: Serialize,
{
    pub elements: Vec<T>,
    pub total_elements: usize,
    #[serde(flatten)]
    pub pagination: Pagination,
}

/// Wrapper to connect contract-actions with their tx-identifier and the related timestamp
#[derive(Debug, Clone, Serialize)]
pub struct TxAction {
    /// Transaction identifier
    pub tx_id: TxIdentifier,
    /// Serializable action object
    pub action: CallAction,
    pub commited: u64,
}

/// Json description of a contract
///
/// Groups the most relevant information around a contract in a single datastructure.
#[derive(Debug, Clone, Serialize)]
pub struct ContractInfo {
    pub info: Option<Info>,
    pub desc: Option<Description>,
    pub meta: Option<Metadata>,
}

pub mod queries {
    use std::{
        collections::{HashMap, HashSet},
        fmt::Display,
        str::FromStr,
    };

    use borderless_id_types::{AgentId, ContractId};
    use serde::Serialize;

    pub struct Query {
        /// Key-Value pairs of the query, where the key is one of the following keywords:
        /// - page, per_page
        /// - sort, order
        /// - action
        ///
        /// These are handled seperately, because we my build a [`Pagination`] or [`Sorting`] object from it.
        items: HashMap<String, String>,
        /// Other Key-Value pairs, that will be used in a generic "where" clause
        other: HashSet<String>,
    }

    impl Query {
        /// Parses the query from a string
        pub fn parse<S: AsRef<str>>(query_str: S) -> Query {
            let mut items = HashMap::new();
            let mut other = HashSet::new();
            // Split items at '&'
            for encoded in query_str.as_ref().split('&') {
                // Decode url pattern
                // let key_value = urlencoding::decode(encoded).unwrap_or_default();
                let key_value = encoded; // TODO: urlencoding !

                // First, we have to check if there is a '<' or '>' sign in the string,
                // because in this case we have to handle it differently
                if key_value.contains(['<', '>']) {
                    // In this case we just remember the entire statement as whole
                    other.insert(key_value.replace('+', " "));
                } else {
                    // Check for key-value pairs
                    let mut iter = key_value.splitn(2, '=');
                    let key = iter.next().unwrap_or_default();
                    let value = iter.next().unwrap_or_default();
                    // Ignore empty values
                    if !value.is_empty() && !key.is_empty() {
                        match key {
                            // Check weather or not we have a special keyword
                            "page" | "per_page" | "sort" | "order" | "action" => {
                                items.insert(key.to_string(), value.to_string());
                            }
                            // Check for contract_id and process_id, as they require special parsing
                            "contract_id" | "contract-id" => {
                                if let Ok(id) = ContractId::parse_str(value) {
                                    other.insert(format!("contract_id={}", id));
                                }
                            }
                            "agent_id" | "agent-id" | "process_id" | "process-id" => {
                                if let Ok(id) = AgentId::parse_str(value) {
                                    other.insert(format!("agent_id={}", id));
                                }
                            }
                            // Otherwise just remember the entire statement as whole
                            _ => {
                                other.insert(format!("{}={}", key, value.replace('+', " "),));
                            }
                        }
                    }
                }
            }
            Query { items, other }
        }

        /// Returns pagination element if present
        pub fn pagination(&self) -> Option<Pagination> {
            let page_item = self.items.get("page")?;
            let per_page_item = self.items.get("per_page")?;
            let page = usize::from_str(page_item).ok()?;
            let per_page = usize::from_str(per_page_item).ok()?;
            Some(Pagination { page, per_page })
        }

        /// Returns sorting element if present
        pub fn sorting(&self) -> Option<Sorting> {
            let sort_by = self.items.get("sort")?.clone();
            let order_item = match self.items.get("order") {
                Some(item) => item,
                None => {
                    return Some(Sorting {
                        sort_by,
                        order: Order::Ascending,
                    })
                }
            };
            let order = match order_item.to_ascii_lowercase().as_ref() {
                "ascending" | "asc" => Order::Ascending,
                "descending" | "desc" => Order::Descending,
                // Everything else is invalid
                _ => return None,
            };
            Some(Sorting { sort_by, order })
        }

        /// Returns action-query if present
        pub fn action_query(&self) -> Option<String> {
            let _action = self.items.get("action")?;
            todo!("re-implement this for new action system")
        }

        pub fn contains_other(&self) -> bool {
            !self.other.is_empty()
        }

        pub fn other(&self) -> impl Iterator<Item = &str> {
            self.other.iter().map(|s| s.as_str())
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    pub enum Order {
        Ascending,
        Descending,
    }

    impl AsRef<str> for Order {
        fn as_ref(&self) -> &str {
            match self {
                Order::Ascending => "ASC",
                Order::Descending => "DESC",
            }
        }
    }

    impl Display for Order {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.as_ref())
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    pub struct Sorting {
        pub sort_by: String,
        pub order: Order,
    }

    /// Simple struct that is used to add pagination to some endpoint like: `/endpoint?page=1&per_page=10`
    ///
    /// The page numbers start at "1", so they match what you would display in a frontend.
    ///
    /// The default implementation returns you `page=1` and `per_page=1000`.
    #[derive(Serialize, Clone)]
    pub struct Pagination {
        pub page: usize,
        pub per_page: usize,
    }

    impl Default for Pagination {
        fn default() -> Self {
            Self {
                page: 1,
                per_page: 100,
            }
        }
    }

    impl Pagination {
        /// Extracts a pagination from some query string (if any).
        ///
        /// The query can contain other elements aswell and there is no necessity for the pieces
        /// `page` and `per_page` to be two consecutive elements in the query.
        ///
        /// As long as there is a `page={}` element and a `per_page={}` element,
        /// this function will successfully parse and return the `Pagination` struct.
        pub fn from_query(query: Option<&str>) -> Option<Pagination> {
            let query = query?;
            let mut page_str: Option<&str> = None;
            let mut per_page_str: Option<&str> = None;
            for piece in query.split('&') {
                if piece.starts_with("page=") {
                    page_str = Some(piece);
                } else if piece.starts_with("per_page=") || piece.starts_with("per-page") {
                    per_page_str = Some(piece);
                }
                if page_str.is_some() && per_page_str.is_some() {
                    break;
                }
            }
            // NOTE: We want the per_page or page to be set to the value that Pagionation::default() assigns.
            // Using clippys suggestion would overwrite the value with the default for the type (which is 0).
            #[allow(clippy::field_reassign_with_default)]
            match (page_str, per_page_str) {
                (Some(page_str), Some(per_page_str)) => {
                    let page_num: &str = page_str.split('=').nth(1)?;
                    let per_page_num: &str = per_page_str.split('=').nth(1)?;
                    let page = usize::from_str(page_num).ok()?;
                    let per_page = usize::from_str(per_page_num).ok()?;
                    Some(Pagination { page, per_page })
                }
                (Some(page_str), None) => {
                    let page_num: &str = page_str.split('=').nth(1)?;
                    let page = usize::from_str(page_num).ok()?;
                    let mut pagination = Pagination::default();
                    pagination.page = page;
                    Some(pagination)
                }
                (None, Some(per_page_str)) => {
                    let per_page_num: &str = per_page_str.split('=').nth(1)?;
                    let per_page = usize::from_str(per_page_num).ok()?;
                    let mut pagination = Pagination::default();
                    pagination.per_page = per_page;
                    Some(pagination)
                }
                _ => None,
            }
        }

        /// Converts the pagination into a range to iterate over
        ///
        /// Note: The index of the range starts at "0" and not at "1",
        /// like the pagination does. No manual conversion needed.
        pub fn to_range(&self) -> std::ops::Range<usize> {
            self.clone().into()
        }
    }

    impl From<Pagination> for std::ops::Range<usize> {
        fn from(value: Pagination) -> Self {
            let start = value.page.saturating_sub(1) * value.per_page;
            let end = value.page * value.per_page;
            Self { start, end }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn pagination() {
            let queries = [
                "page=10&per_page=2312",                     // Simple query
                "per_page=2312&page=10", // Simple query, but fields are reversed
                "something_else=null&page=10&per_page=2312", // Complex query that contains a pagination element
                "page=10&something_else=null&per_page=2312", // Complex query that contains a pagination element
            ];
            for query in queries {
                let pagination = Pagination::from_query(Some(query));
                assert!(pagination.is_some());
                let pagination = pagination.unwrap();
                assert_eq!(pagination.page, 10);
                assert_eq!(pagination.per_page, 2312);
                // The Query object must produce the same result
                let pagination = Query::parse(query).pagination();
                assert!(pagination.is_some());
                let pagination = pagination.unwrap();
                assert_eq!(pagination.page, 10);
                assert_eq!(pagination.per_page, 2312);
            }
            assert!(Pagination::from_query(None).is_none());
            let bad_queries = [
                "page=10&per_page=",    // Missing argument
                "per_page=2312&page=",  // Missing argument, but fields are reversed
                "page=id&perpage=2312", // Misspelling
                "something_else=null",  // Something else
            ];
            for query in bad_queries {
                let pagination = Pagination::from_query(Some(query));
                assert!(
                    pagination.is_none(),
                    "This query should not work: {}",
                    query
                );
                // The Query object must produce the same result
                let pagination = Query::parse(query).pagination();
                assert!(
                    pagination.is_none(),
                    "This query should not work: {}",
                    query
                );
            }
        }

        #[test]
        fn keyvalue() {
            let good_queries = [
                "key=id&value=2312",                     // Simple query
                "value=2312&key=id",                     // Simple query, but fields are reversed
                "something_else=null&key=id&value=2312", // Complex query
                "key=id&something_else=null&value=2312", // Complex query
            ];
            for query in good_queries {
                let key_value = Query::parse(query);
                assert!(key_value.contains_other());
                assert!(key_value.other().any(|s| s == "key=id"));
                assert!(key_value.other().any(|s| s == "value=2312"));
            }
            let bad_queries = [
                "key=&value=",         // Missing argument
                "value=&key=",         // Missing argument, but fields are reversed
                "ky=id&vaue=2312",     // Misspelling
                "something_else=null", // Something else
            ];
            for query in bad_queries {
                let key_value = Query::parse(query);
                assert!(!key_value.other().any(|s| s == "key=id"));
                assert!(!key_value.other().any(|s| s == "value=2312"));
            }
        }

        #[test]
        fn sorting() {
            let good_queries_asc = [
                "sort=sensor_id&order=asc",
                "sort=sensor_id&order=ascending",
                "order=ascending&sort=sensor_id",
                "order=asc&sort=sensor_id",
                "sort=sensor_id&order=aSc",
                "sort=sensor_id&order=ASCending",
                "order=Ascending&sort=sensor_id",
                "order=Asc&sort=sensor_id",
                "sort=sensor_id", // if we don't specify the order, it is ascending by default
                "sort=sensor_id&order", // defaults to ascending, because order is ignored
            ];
            for query in good_queries_asc {
                let sorting = Query::parse(query).sorting();
                assert!(sorting.is_some());
                let sorting = sorting.unwrap();
                assert_eq!(
                    sorting,
                    Sorting {
                        sort_by: "sensor_id".to_string(),
                        order: Order::Ascending
                    }
                );
            }
            let good_queries_desc = [
                "sort=sensor_id&order=desc",
                "sort=sensor_id&order=descending",
                "order=descending&sort=sensor_id",
                "order=desc&sort=sensor_id",
                "sort=sensor_id&order=dESc",
                "sort=sensor_id&order=Descending",
                "order=Descending&sort=sensor_id",
                "order=Desc&sort=sensor_id",
            ];
            for query in good_queries_desc {
                let sorting = Query::parse(query).sorting();
                assert!(sorting.is_some());
                let sorting = sorting.unwrap();
                assert_eq!(
                    sorting,
                    Sorting {
                        sort_by: "sensor_id".to_string(),
                        order: Order::Descending
                    }
                );
            }
            let bad_queries = [
                "sort=sensor_id&order=something", // Wrong order
                "sort=&order=descending",         // Missing key
                "order=ascending",                // Missing sort
            ];
            for query in bad_queries {
                let sorting = Query::parse(query).sorting();
                assert!(sorting.is_none(), "This query should not work: {}", query);
            }
        }

        #[test]
        fn parse_contract_id() {
            let contract_ids = [
                "contract-id=c073e869-4ae1-892c-aba7-2ad8318d5c12",
                "contract_id=c073e869-4ae1-892c-aba7-2ad8318d5c12",
            ];
            for id in contract_ids {
                let query = Query::parse(id);
                assert!(query.contains_other(), "Query should contain contract-id");
                let cid = query.other().next().unwrap();
                assert_eq!(cid, "contract_id=c073e869-4ae1-892c-aba7-2ad8318d5c12");
            }
            let agent_ids = [
                "process_id=a073e869-4ae1-892c-aba7-2ad8318d5c12",
                "process-id=a073e869-4ae1-892c-aba7-2ad8318d5c12",
                "agent-id=a073e869-4ae1-892c-aba7-2ad8318d5c12",
                "agent_id=a073e869-4ae1-892c-aba7-2ad8318d5c12",
            ];
            for id in agent_ids {
                let query = Query::parse(id);
                assert!(query.contains_other(), "Query should contain contract-id");
                let cid = query.other().next().unwrap();
                assert_eq!(cid, "agent_id=a073e869-4ae1-892c-aba7-2ad8318d5c12");
            }
        }
    }
}
