use http::request::Parts;
use serde::{Deserialize, Serialize};

//#[deprecated(note = "replaced by trait function")]
pub fn to_payload<T>(value: &T, path: &str) -> anyhow::Result<Option<String>>
where
    T: Serialize,
{
    // Different Approach:
    let value = serde_json::to_value(value)?;

    // Instantly return the value
    if path.is_empty() {
        return Ok(Some(value.to_string()));
    }

    // Search sub-fields based on path
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

// TODO: Let's only use this for batch-requests; just to keep the usage of postcard to a minimum
#[derive(Serialize, Deserialize)]
pub struct SerRq {
    pub method: String,
    pub uri: String,
    pub headers: String,
    pub body: Vec<u8>,
}

impl SerRq {
    pub fn to_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        postcard::to_allocvec(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, postcard::Error> {
        postcard::from_bytes(bytes)
    }

    pub fn from_parts(parts: Parts, body: Vec<u8>) -> Self {
        let mut headers = String::with_capacity(16);
        for (name, value) in parts.headers.iter() {
            headers.push_str(name.as_str());
            headers.push_str(": ");
            headers.push_str(value.to_str().unwrap_or_default());
            headers.push_str("\r\n");
        }
        Self {
            method: parts.method.to_string(),
            uri: parts.uri.to_string(),
            headers,
            body,
        }
    }
}
