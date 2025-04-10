use serde::{Deserialize, Serialize};

/// Very simple and basic response type
///
/// Since webassembly is quite basic and the webserver lives on the host anyway,
/// the only two things we have to communicate back to the host are the status-code
/// and the payload of the response (if successful).
///
/// The payload is always json encoded, as all contracts generate a REST-API.
#[derive(Serialize, Deserialize)]
pub struct Response {
    pub status: u16,
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
}

impl Response {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, postcard::Error> {
        postcard::from_bytes(bytes)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        postcard::to_allocvec(self)
    }
}

#[derive(Serialize, Deserialize)]
pub enum Method {
    GET = 0,
    POST = 1,
}

/// Very simple and basic request type
#[derive(Serialize, Deserialize)]
pub struct Request {
    pub method: Method,
    pub path: String,
    pub query: Option<String>,
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
}

impl Request {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, postcard::Error> {
        postcard::from_bytes(bytes)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        postcard::to_allocvec(self)
    }
}

// NOTE: We need something different, that works with our storage implementation.
// For now, we can try fiddling around with this.
pub fn to_payload<T>(value: &T, path: &str) -> anyhow::Result<Option<String>>
where
    T: Serialize,
{
    // Different Approach:
    let value = serde_json::to_value(value)?;

    let mut current = &value;
    for seg in path
        .split('/')
        .flat_map(|s| if s.is_empty() { None } else { Some(s) })
    {
        current = match current.get(seg) {
            Some(v) => v,
            None => return Ok(None),
        };
    }
    // FCK my life, this works so great...
    Ok(Some(current.to_string()))
}
