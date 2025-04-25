//! Definition of generic models used throughout different APIs

use std::str::FromStr;

use borderless_id_types::TxIdentifier;
use http::header::CONTENT_TYPE;
use queries::Pagination;
use serde::Serialize;

use crate::__private::send_http_rq;
use crate::contracts::{Description, Info, Metadata};
use crate::events::CallAction;

pub use http::{HeaderName, HeaderValue, Method, Request, Response, StatusCode, Version};

/// Special trait that bundles the serialization of a type into a request body together with the corresponding content-type
///
/// For empty bodies with no content type `None` should be returned.
pub trait IntoBodyAndContentType {
    fn into_parts(self) -> anyhow::Result<Option<(Vec<u8>, &'static str)>>;
}

/// Wrapper type that automatically serializes the inner value to JSON and sets the content-type to `application/json`, if used together with a request.
///
/// See [`send_request`] function.
pub struct Json<T>(pub T);

impl<T: Serialize> IntoBodyAndContentType for Json<T> {
    fn into_parts(self) -> anyhow::Result<Option<(Vec<u8>, &'static str)>> {
        let body = serde_json::to_vec(&self.0)?;
        Ok(Some((body, "application/json")))
    }
}

/// Wrapper type that automatically serializes the inner value to plain text and sets the content-type to `text/plain`, if used together with a request.
///
/// See [`send_request`] function.
pub struct Text<T>(pub T);

impl<T: ToString> IntoBodyAndContentType for Text<T> {
    fn into_parts(self) -> anyhow::Result<Option<(Vec<u8>, &'static str)>> {
        let body = self.0.to_string().into_bytes();
        Ok(Some((body, "text/plain")))
    }
}

impl IntoBodyAndContentType for String {
    fn into_parts(self) -> anyhow::Result<Option<(Vec<u8>, &'static str)>> {
        let body = self.into_bytes();
        Ok(Some((body, "text/plain")))
    }
}

impl IntoBodyAndContentType for &str {
    fn into_parts(self) -> anyhow::Result<Option<(Vec<u8>, &'static str)>> {
        let body = self.to_string().into_bytes();
        Ok(Some((body, "text/plain")))
    }
}

/// Type that indicates an empty body. In this case no content-type header is set.
///
/// Identical to unit type `()`.
///
/// See [`send_request`] function.
pub struct Empty;

impl IntoBodyAndContentType for Empty {
    fn into_parts(self) -> anyhow::Result<Option<(Vec<u8>, &'static str)>> {
        Ok(None)
    }
}

impl IntoBodyAndContentType for () {
    fn into_parts(self) -> anyhow::Result<Option<(Vec<u8>, &'static str)>> {
        Ok(None)
    }
}

/// Wrapper type that automatically serializes the inner value to bytes and sets the content-type to `application/octet-stream`, if used together with a request.
///
/// See [`send_request`] function.
pub struct Binary<T>(pub T);

impl<T: Into<Vec<u8>>> IntoBodyAndContentType for Binary<T> {
    fn into_parts(self) -> anyhow::Result<Option<(Vec<u8>, &'static str)>> {
        let body = self.0.into();
        Ok(Some((body, "application/octet-stream")))
    }
}

impl IntoBodyAndContentType for Vec<u8> {
    fn into_parts(self) -> anyhow::Result<Option<(Vec<u8>, &'static str)>> {
        Ok(Some((self, "application/octet-stream")))
    }
}

/// Send a http-request from webassembly and receive the response
pub fn send_request<T>(request: Request<T>) -> anyhow::Result<Response<Vec<u8>>>
where
    T: IntoBodyAndContentType,
{
    let (mut parts, body) = request.into_parts();

    // Inject correct content-type ( for empty bodies () we don't set the header value )
    let body_bytes = match body.into_parts()? {
        Some((bytes, content_type)) => {
            parts
                .headers
                .insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
            bytes
        }
        None => Vec::new(),
    };

    // Serialize request head according to protocol
    let mut head = format!("{} {} {:?}\r\n", parts.method, parts.uri, parts.version);
    for (name, value) in parts.headers.iter() {
        head.push_str(&format!("{}: {}\r\n", name, value.to_str().unwrap()));
    }
    head.push_str("\r\n"); // End of headers

    // Perform the ABI call to actually send the request
    let (rs_head, rs_body) = send_http_rq(head, body_bytes).map_err(|e| anyhow::Error::msg(e))?;
    let rs = build_response_from_parts(&rs_head, rs_body)?;
    Ok(rs)
}

/// Helper function to parse a [`Response`] from the raw parts
fn build_response_from_parts(head: &str, body: Vec<u8>) -> anyhow::Result<Response<Vec<u8>>> {
    let mut lines = head.lines();

    // Parse the status line
    let status_line = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("Empty response head"))?;
    let mut status_parts = status_line.splitn(3, ' ');

    let version_str = status_parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("Missing HTTP version"))?;
    let status_code_str = status_parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("Missing status code"))?;
    let _reason_phrase = status_parts.next().unwrap_or(""); // Optional, ignore for now

    let version = match version_str {
        "HTTP/1.0" => Version::HTTP_10,
        "HTTP/1.1" => Version::HTTP_11,
        "HTTP/2.0" | "HTTP/2" => Version::HTTP_2,
        _ => return Err(anyhow::anyhow!("Unsupported HTTP version: {}", version_str)),
    };

    let status_code = StatusCode::from_bytes(status_code_str.as_bytes())?;

    // Build base response;
    let mut response = Response::builder().status(status_code).version(version);

    let headers = response.headers_mut().unwrap();

    // Parse headers
    for line in lines {
        if line.trim().is_empty() {
            continue; // Skip empty lines
        }
        if let Some((name, value)) = line.split_once(':') {
            let header_name = HeaderName::from_str(name.trim())?;
            let header_value = HeaderValue::from_str(value.trim())?;
            headers.insert(header_name, header_value);
        } else {
            return Err(anyhow::anyhow!("Malformed header line: {}", line));
        }
    }
    // Build the response
    Ok(response.body(body)?)
}

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
    mod query_tests {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_response() {
        let head = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nX-Test: 123\r\n\r\n";
        let body = b"Hello, world!".to_vec();

        let response = build_response_from_parts(head, body.clone()).unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.version(), Version::HTTP_11);
        assert_eq!(
            response.headers().get("Content-Type").unwrap(),
            "text/plain"
        );
        assert_eq!(response.headers().get("X-Test").unwrap(), "123");
        assert_eq!(response.body(), &body);
    }

    #[test]
    fn test_valid_http_2_response() {
        let head = "HTTP/2 204 No Content\r\nX-Empty: yes\r\n\r\n";
        let body = Vec::new();

        let response = build_response_from_parts(head, body.clone()).unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(response.version(), Version::HTTP_2);
        assert_eq!(response.headers().get("X-Empty").unwrap(), "yes");
        assert_eq!(response.body(), &body);
    }

    #[test]
    fn test_missing_status_line_parts() {
        let head = "HTTP/1.1\r\nContent-Type: text/plain\r\n\r\n";
        let body = b"Oops".to_vec();

        let result = build_response_from_parts(head, body);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_http_version() {
        let head = "HTTP/3.0 200 OK\r\nContent-Type: text/plain\r\n\r\n";
        let body = b"Invalid".to_vec();

        let result = build_response_from_parts(head, body);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_status_code() {
        let head = "HTTP/1.1 abc OK\r\nContent-Type: text/plain\r\n\r\n";
        let body = b"Invalid".to_vec();

        let result = build_response_from_parts(head, body);
        assert!(result.is_err());
    }

    #[test]
    fn test_malformed_header() {
        let head = "HTTP/1.1 200 OK\r\nBad-Header-Without-Colon\r\n\r\n";
        let body = b"BadHeader".to_vec();

        let result = build_response_from_parts(head, body);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_head() {
        let head = "";
        let body = b"Empty".to_vec();

        let result = build_response_from_parts(head, body);
        assert!(result.is_err());
    }
}
