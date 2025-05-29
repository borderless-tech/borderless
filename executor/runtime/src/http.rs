use bytes::Bytes;
use http::request::Parts;
use http::{header::CONTENT_TYPE, HeaderValue, StatusCode};
use mime::{APPLICATION_JSON, TEXT_PLAIN_UTF_8};
use serde::Serialize;
pub use tower::Service;

#[cfg(feature = "contracts")]
pub mod contract;

#[cfg(feature = "agents")]
pub mod agent;

pub type Request<T = Bytes> = http::Request<T>;
pub type Response<T = Bytes> = http::Response<T>;

pub fn reject_404() -> Response {
    let mut resp = Response::new(Bytes::new());
    *resp.status_mut() = StatusCode::NOT_FOUND;
    resp
}

pub fn method_not_allowed() -> Response {
    let mut resp = Response::new(Bytes::new());
    *resp.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
    resp
}

pub fn unsupported_media_type() -> Response {
    let mut resp = Response::new(Bytes::new());
    *resp.status_mut() = StatusCode::UNSUPPORTED_MEDIA_TYPE;
    resp
}

fn bad_request(err: String) -> Response {
    let mut resp = Response::new(err.into_bytes().into());
    *resp.status_mut() = StatusCode::BAD_REQUEST;
    resp
}

pub fn err_response(status: StatusCode, err_msg: String) -> Response {
    let mut resp = Response::new(err_msg.into_bytes().into());
    *resp.status_mut() = status;
    resp
}

pub fn json_response<S: Serialize>(value: &S) -> Response<Bytes> {
    let bytes = serde_json::to_vec(value).unwrap();
    json_body(bytes)
}

pub fn json_body(bytes: Vec<u8>) -> Response<Bytes> {
    let mut resp = Response::new(bytes.into());
    resp.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static(APPLICATION_JSON.as_ref()),
    );
    resp
}

pub fn check_json_content(parts: &Parts) -> bool {
    if let Some(content_type) = parts.headers.get(CONTENT_TYPE) {
        content_type.as_bytes() == APPLICATION_JSON.as_ref().as_bytes()
    } else {
        false
    }
}

/// Converts any error-type into a http-response with status-code 500
///
/// The response has content-type set to `text/plain; charset=utf-8`
/// and the body contains the error message as string.
pub fn into_server_error<E: ToString>(error: E) -> Response {
    let mut resp = Response::new(error.to_string().into_bytes().into());
    resp.headers_mut().append(
        CONTENT_TYPE,
        HeaderValue::from_static(TEXT_PLAIN_UTF_8.as_ref()),
    );
    *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    resp
}
