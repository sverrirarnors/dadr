use std::{
    borrow::Cow,
    io::{Read, Write},
    net::TcpStream,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use dadrock::DadrError;
use serde::{de::DeserializeOwned, Serialize};

pub(super) const MAX_HEADER_BYTES: usize = 16 * 1024;
pub(super) const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
const REQUEST_READ_DEADLINE: Duration = Duration::from_secs(30);

pub(super) fn json_body<T: DeserializeOwned>(request: &HttpRequest) -> Result<T, ApiFailure> {
    serde_json::from_slice(&request.body).map_err(|err| ApiFailure::bad_request(err.to_string()))
}

pub(super) fn json_response<T: Serialize>(
    status: u16,
    body: &T,
) -> Result<HttpResponse, ApiFailure> {
    let body = serde_json::to_vec(body).map_err(|err| ApiFailure::internal(err.to_string()))?;
    Ok(HttpResponse {
        status,
        content_type: "application/json; charset=utf-8".to_owned(),
        body: Cow::Owned(body),
    })
}

pub(super) fn static_response(
    status: u16,
    content_type: impl Into<String>,
    body: Cow<'static, [u8]>,
) -> HttpResponse {
    HttpResponse {
        status,
        content_type: content_type.into(),
        body,
    }
}

pub(super) fn api_error_response(status: u16, message: String) -> HttpResponse {
    let body = serde_json::to_vec(&ApiError { error: message })
        .unwrap_or_else(|_| br#"{"error":"internal serialization error"}"#.to_vec());
    HttpResponse {
        status,
        content_type: "application/json; charset=utf-8".to_owned(),
        body: Cow::Owned(body),
    }
}

pub(super) fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, ApiFailure> {
    read_http_request_with_deadline(stream, Instant::now() + REQUEST_READ_DEADLINE)
}

pub(super) fn read_http_request_with_deadline(
    stream: &mut impl Read,
    deadline: Instant,
) -> Result<HttpRequest, ApiFailure> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 8192];
    let mut header_end = None;
    let mut content_length = 0usize;

    loop {
        if Instant::now() > deadline {
            return Err(ApiFailure::request_timeout("request read timed out"));
        }
        let read = stream
            .read(&mut chunk)
            .map_err(|err| ApiFailure::bad_request(format!("failed to read request: {err}")))?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if header_end.is_none() && buffer.len() > MAX_HEADER_BYTES {
            return Err(ApiFailure::payload_too_large(format!(
                "request headers exceed {MAX_HEADER_BYTES} bytes"
            )));
        }

        if header_end.is_none() {
            if let Some(end) = find_header_end(&buffer) {
                if end > MAX_HEADER_BYTES {
                    return Err(ApiFailure::payload_too_large(format!(
                        "request headers exceed {MAX_HEADER_BYTES} bytes"
                    )));
                }
                let header_text = String::from_utf8_lossy(&buffer[..end]);
                reject_transfer_encoding(&header_text)?;
                content_length = parse_content_length(&header_text)?;
                if content_length > MAX_BODY_BYTES {
                    return Err(ApiFailure::payload_too_large(format!(
                        "request body exceeds {MAX_BODY_BYTES} bytes"
                    )));
                }
                header_end = Some(end);
            }
        }

        if let Some(end) = header_end {
            if buffer.len() >= end + 4 + content_length {
                break;
            }
        }
    }

    let header_end = header_end
        .ok_or_else(|| ApiFailure::bad_request("request did not include HTTP headers"))?;
    let header_text = String::from_utf8_lossy(&buffer[..header_end]);
    let mut lines = header_text.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| ApiFailure::bad_request("request line is missing"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| ApiFailure::bad_request("request method is missing"))?
        .to_owned();
    let path = parts
        .next()
        .ok_or_else(|| ApiFailure::bad_request("request path is missing"))?
        .to_owned();
    let headers = parse_headers(&header_text)?;
    let body_start = header_end + 4;
    let body_end = body_start + content_length;
    if body_end > buffer.len() {
        return Err(ApiFailure::bad_request(format!(
            "incomplete request body: expected {content_length} bytes, received {}",
            buffer.len().saturating_sub(body_start)
        )));
    }
    let body = buffer[body_start..body_end].to_vec();

    Ok(HttpRequest {
        method,
        path,
        headers,
        body,
    })
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn parse_content_length(headers: &str) -> Result<usize, ApiFailure> {
    let mut content_length = None;
    for line in headers.lines().skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            let parsed = value.trim().parse::<usize>().map_err(|_| {
                ApiFailure::bad_request(format!("invalid Content-Length header `{}`", value.trim()))
            })?;
            if content_length.replace(parsed).is_some() {
                return Err(ApiFailure::bad_request(
                    "multiple Content-Length headers are not supported",
                ));
            }
        }
    }
    Ok(content_length.unwrap_or(0))
}

fn reject_transfer_encoding(headers: &str) -> Result<(), ApiFailure> {
    for line in headers.lines().skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("transfer-encoding") && !value.trim().is_empty() {
            return Err(ApiFailure::not_implemented(
                "Transfer-Encoding is not supported by the preview server",
            ));
        }
    }
    Ok(())
}

fn parse_headers(header_text: &str) -> Result<Vec<HttpHeader>, ApiFailure> {
    let mut headers = Vec::new();
    for line in header_text.lines().skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            return Err(ApiFailure::bad_request(format!(
                "malformed HTTP header `{line}`"
            )));
        };
        headers.push(HttpHeader {
            name: name.trim().to_owned(),
            value: value.trim().to_owned(),
        });
    }
    Ok(headers)
}

pub(super) fn validate_host_header(request: &HttpRequest) -> Result<(), ApiFailure> {
    let Some(host) = request.header("host") else {
        return Err(ApiFailure::bad_request("Host header is required"));
    };
    let host = host.trim();
    let allowed = host == "localhost"
        || host.starts_with("localhost:")
        || host == "127.0.0.1"
        || host.starts_with("127.0.0.1:")
        || host == "[::1]"
        || host.starts_with("[::1]:");
    if allowed {
        Ok(())
    } else {
        Err(ApiFailure::bad_request(format!(
            "Host header `{host}` is not allowed"
        )))
    }
}

pub(super) fn write_http_response(stream: &mut TcpStream, response: HttpResponse) -> Result<()> {
    let reason = reason_phrase(response.status);
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n\r\n",
        response.status,
        reason,
        response.content_type,
        response.body.len()
    )
    .context("failed to write response headers")?;
    stream
        .write_all(&response.body)
        .context("failed to write response body")
}

pub(super) fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        408 => "Request Timeout",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        _ => "",
    }
}

#[derive(Debug)]
pub(super) struct HttpRequest {
    pub(super) method: String,
    pub(super) path: String,
    pub(super) headers: Vec<HttpHeader>,
    pub(super) body: Vec<u8>,
}

impl HttpRequest {
    pub(super) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case(name))
            .map(|header| header.value.as_str())
    }
}

#[derive(Debug)]
pub(super) struct HttpHeader {
    pub(super) name: String,
    pub(super) value: String,
}

#[derive(Debug)]
pub(super) struct HttpResponse {
    pub(super) status: u16,
    pub(super) content_type: String,
    pub(super) body: Cow<'static, [u8]>,
}

#[derive(Debug)]
pub(super) struct ApiFailure {
    pub(super) status: u16,
    pub(super) message: String,
}

impl ApiFailure {
    pub(super) fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: 400,
            message: message.into(),
        }
    }

    fn request_timeout(message: impl Into<String>) -> Self {
        Self {
            status: 408,
            message: message.into(),
        }
    }

    fn payload_too_large(message: impl Into<String>) -> Self {
        Self {
            status: 413,
            message: message.into(),
        }
    }

    fn not_implemented(message: impl Into<String>) -> Self {
        Self {
            status: 501,
            message: message.into(),
        }
    }

    pub(super) fn internal(message: impl Into<String>) -> Self {
        Self {
            status: 500,
            message: message.into(),
        }
    }
}

impl From<DadrError> for ApiFailure {
    fn from(err: DadrError) -> Self {
        Self::bad_request(err.to_string())
    }
}

#[derive(Serialize)]
pub(super) struct ApiError {
    error: String,
}

impl ApiError {
    pub(super) fn not_found() -> Self {
        Self {
            error: "not found".to_owned(),
        }
    }
}
