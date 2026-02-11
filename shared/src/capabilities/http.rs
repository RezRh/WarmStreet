use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;
use url::Url;

use crux_http::Http;

use crate::event::Event;

pub type HttpCapability = Http<Event>;

pub const MAX_URL_LENGTH: usize = 2048;
pub const MAX_REQUEST_BODY_SIZE: usize = 50 * 1024 * 1024;
pub const MAX_RESPONSE_BODY_SIZE: usize = 100 * 1024 * 1024;
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;
pub const MAX_TIMEOUT_MS: u64 = 300_000;
pub const MAX_HEADER_NAME_LENGTH: usize = 256;
pub const MAX_HEADER_VALUE_LENGTH: usize = 8192;
pub const MAX_HEADERS_COUNT: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValidatedUrl {
    url: String,
    scheme: String,
    host: String,
}

impl ValidatedUrl {
    pub fn new(url: impl Into<String>) -> Result<Self, HttpError> {
        let url = url.into();
        Self::validate(&url)?;

        let parsed = Url::parse(&url).map_err(|e| HttpError::InvalidUrl {
            url: Self::truncate_url(&url),
            reason: e.to_string(),
        })?;

        let scheme = parsed.scheme().to_lowercase();
        let host = parsed
            .host_str()
            .ok_or_else(|| HttpError::InvalidUrl {
                url: Self::truncate_url(&url),
                reason: "missing host".to_string(),
            })?
            .to_lowercase();

        Ok(Self {
            url: parsed.to_string(),
            scheme,
            host,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.url
    }

    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    fn validate(url: &str) -> Result<(), HttpError> {
        if url.is_empty() {
            return Err(HttpError::InvalidUrl {
                url: String::new(),
                reason: "URL cannot be empty".to_string(),
            });
        }

        if url.len() > MAX_URL_LENGTH {
            return Err(HttpError::InvalidUrl {
                url: Self::truncate_url(url),
                reason: format!("URL exceeds maximum length of {} bytes", MAX_URL_LENGTH),
            });
        }

        let trimmed = url.trim();
        if trimmed.is_empty() {
            return Err(HttpError::InvalidUrl {
                url: url.to_string(),
                reason: "URL cannot be only whitespace".to_string(),
            });
        }

        let parsed = Url::parse(url).map_err(|e| HttpError::InvalidUrl {
            url: Self::truncate_url(url),
            reason: e.to_string(),
        })?;

        let scheme = parsed.scheme().to_lowercase();
        if scheme != "http" && scheme != "https" {
            return Err(HttpError::InvalidUrl {
                url: Self::truncate_url(url),
                reason: format!(
                    "invalid scheme '{}', only 'http' and 'https' are allowed",
                    scheme
                ),
            });
        }

        if parsed.host_str().is_none() {
            return Err(HttpError::InvalidUrl {
                url: Self::truncate_url(url),
                reason: "URL must have a host".to_string(),
            });
        }

        let host = parsed.host_str().unwrap().to_lowercase();
        if Self::is_private_host(&host, &parsed) {
            return Err(HttpError::PrivateNetworkBlocked {
                url: Self::truncate_url(url),
                host,
            });
        }

        if parsed.username() != "" || parsed.password().is_some() {
            return Err(HttpError::InvalidUrl {
                url: Self::truncate_url(url),
                reason: "credentials in URL are not allowed".to_string(),
            });
        }

        Ok(())
    }

    fn is_private_host(host: &str, parsed: &Url) -> bool {
        if host == "localhost"
            || host == "127.0.0.1"
            || host == "::1"
            || host == "[::1]"
            || host == "0.0.0.0"
        {
            return true;
        }

        if host.ends_with(".local")
            || host.ends_with(".localhost")
            || host.ends_with(".internal")
        {
            return true;
        }

        if host.starts_with("10.")
            || host.starts_with("192.168.")
            || host.starts_with("172.16.")
            || host.starts_with("172.17.")
            || host.starts_with("172.18.")
            || host.starts_with("172.19.")
            || host.starts_with("172.20.")
            || host.starts_with("172.21.")
            || host.starts_with("172.22.")
            || host.starts_with("172.23.")
            || host.starts_with("172.24.")
            || host.starts_with("172.25.")
            || host.starts_with("172.26.")
            || host.starts_with("172.27.")
            || host.starts_with("172.28.")
            || host.starts_with("172.29.")
            || host.starts_with("172.30.")
            || host.starts_with("172.31.")
        {
            return true;
        }

        if host == "169.254.169.254" || host.starts_with("169.254.") {
            return true;
        }

        if host.starts_with("fd") || host.starts_with("fe80:") {
            return true;
        }

        if let Some(port) = parsed.port() {
            if port == 22 || port == 23 || port == 25 || port == 6379 || port == 11211 {
                return true;
            }
        }

        false
    }

    fn truncate_url(url: &str) -> String {
        if url.len() <= 100 {
            url.to_string()
        } else {
            format!("{}...", &url[..100])
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpHeaders {
    headers: Vec<(String, String)>,
}

impl HttpHeaders {
    pub fn new() -> Self {
        Self {
            headers: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            headers: Vec::with_capacity(capacity.min(MAX_HEADERS_COUNT)),
        }
    }

    pub fn insert(
        &mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<(), HttpError> {
        if self.headers.len() >= MAX_HEADERS_COUNT {
            return Err(HttpError::TooManyHeaders {
                count: self.headers.len(),
                max: MAX_HEADERS_COUNT,
            });
        }

        let name = name.into();
        let value = value.into();

        Self::validate_header_name(&name)?;
        Self::validate_header_value(&value)?;

        let name_lower = name.to_lowercase();
        self.headers.retain(|(n, _)| n.to_lowercase() != name_lower);
        self.headers.push((name, value));

        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        let name_lower = name.to_lowercase();
        self.headers
            .iter()
            .find(|(n, _)| n.to_lowercase() == name_lower)
            .map(|(_, v)| v.as_str())
    }

    pub fn get_all(&self, name: &str) -> Vec<&str> {
        let name_lower = name.to_lowercase();
        self.headers
            .iter()
            .filter(|(n, _)| n.to_lowercase() == name_lower)
            .map(|(_, v)| v.as_str())
            .collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.headers.iter().map(|(n, v)| (n.as_str(), v.as_str()))
    }

    pub fn len(&self) -> usize {
        self.headers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.headers.is_empty()
    }

    pub fn into_vec(self) -> Vec<(String, String)> {
        self.headers
    }

    fn validate_header_name(name: &str) -> Result<(), HttpError> {
        if name.is_empty() {
            return Err(HttpError::InvalidHeader {
                name: name.to_string(),
                reason: "header name cannot be empty".to_string(),
            });
        }

        if name.len() > MAX_HEADER_NAME_LENGTH {
            return Err(HttpError::InvalidHeader {
                name: format!("{}...", &name[..50]),
                reason: format!(
                    "header name exceeds maximum length of {} bytes",
                    MAX_HEADER_NAME_LENGTH
                ),
            });
        }

        for c in name.chars() {
            if !c.is_ascii_alphanumeric() && c != '-' && c != '_' {
                return Err(HttpError::InvalidHeader {
                    name: name.to_string(),
                    reason: format!("invalid character '{}' in header name", c),
                });
            }
        }

        let lower = name.to_lowercase();
        if lower == "host" || lower == "content-length" || lower == "transfer-encoding" {
            return Err(HttpError::InvalidHeader {
                name: name.to_string(),
                reason: "this header is managed automatically".to_string(),
            });
        }

        Ok(())
    }

    fn validate_header_value(value: &str) -> Result<(), HttpError> {
        if value.len() > MAX_HEADER_VALUE_LENGTH {
            return Err(HttpError::InvalidHeader {
                name: String::new(),
                reason: format!(
                    "header value exceeds maximum length of {} bytes",
                    MAX_HEADER_VALUE_LENGTH
                ),
            });
        }

        for c in value.chars() {
            if c == '\r' || c == '\n' || c == '\0' {
                return Err(HttpError::InvalidHeader {
                    name: String::new(),
                    reason: "header value contains invalid characters (CR, LF, or NULL)"
                        .to_string(),
                });
            }
        }

        Ok(())
    }
}

impl Default for HttpHeaders {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Vec<(String, String)>> for HttpHeaders {
    fn from(headers: Vec<(String, String)>) -> Self {
        Self { headers }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        }
    }

    pub fn is_idempotent(&self) -> bool {
        matches!(
            self,
            HttpMethod::Get
                | HttpMethod::Put
                | HttpMethod::Delete
                | HttpMethod::Head
                | HttpMethod::Options
        )
    }

    pub fn has_request_body(&self) -> bool {
        matches!(
            self,
            HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch
        )
    }

    pub fn has_response_body(&self) -> bool {
        !matches!(self, HttpMethod::Head)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentType {
    Json,
    FormUrlEncoded,
    Multipart,
    OctetStream,
    Text,
    Custom(String),
}

impl ContentType {
    pub fn as_str(&self) -> &str {
        match self {
            ContentType::Json => "application/json",
            ContentType::FormUrlEncoded => "application/x-www-form-urlencoded",
            ContentType::Multipart => "multipart/form-data",
            ContentType::OctetStream => "application/octet-stream",
            ContentType::Text => "text/plain",
            ContentType::Custom(s) => s.as_str(),
        }
    }

    pub fn from_header(value: &str) -> Self {
        let lower = value.to_lowercase();
        if lower.starts_with("application/json") {
            ContentType::Json
        } else if lower.starts_with("application/x-www-form-urlencoded") {
            ContentType::FormUrlEncoded
        } else if lower.starts_with("multipart/form-data") {
            ContentType::Multipart
        } else if lower.starts_with("application/octet-stream") {
            ContentType::OctetStream
        } else if lower.starts_with("text/plain") {
            ContentType::Text
        } else {
            ContentType::Custom(value.to_string())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub retryable_status_codes: Vec<u16>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 1000,
            max_backoff_ms: 30000,
            retryable_status_codes: vec![408, 429, 500, 502, 503, 504],
        }
    }
}

impl RetryConfig {
    pub fn none() -> Self {
        Self {
            max_retries: 0,
            ..Default::default()
        }
    }

    pub fn is_retryable_status(&self, status: u16) -> bool {
        self.retryable_status_codes.contains(&status)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequest {
    method: HttpMethod,
    url: ValidatedUrl,
    headers: HttpHeaders,
    body: Option<Vec<u8>>,
    timeout_ms: u64,
    retry_config: Option<RetryConfig>,
    request_id: String,
    max_response_size: usize,
}

impl HttpRequest {
    pub fn new(method: HttpMethod, url: ValidatedUrl) -> Self {
        Self {
            method,
            url,
            headers: HttpHeaders::new(),
            body: None,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            retry_config: None,
            request_id: uuid::Uuid::new_v4().to_string(),
            max_response_size: MAX_RESPONSE_BODY_SIZE,
        }
    }

    pub fn get(url: impl Into<String>) -> Result<Self, HttpError> {
        Ok(Self::new(HttpMethod::Get, ValidatedUrl::new(url)?))
    }

    pub fn post(url: impl Into<String>) -> Result<Self, HttpError> {
        Ok(Self::new(HttpMethod::Post, ValidatedUrl::new(url)?))
    }

    pub fn put(url: impl Into<String>) -> Result<Self, HttpError> {
        Ok(Self::new(HttpMethod::Put, ValidatedUrl::new(url)?))
    }

    pub fn patch(url: impl Into<String>) -> Result<Self, HttpError> {
        Ok(Self::new(HttpMethod::Patch, ValidatedUrl::new(url)?))
    }

    pub fn delete(url: impl Into<String>) -> Result<Self, HttpError> {
        Ok(Self::new(HttpMethod::Delete, ValidatedUrl::new(url)?))
    }

    pub fn head(url: impl Into<String>) -> Result<Self, HttpError> {
        Ok(Self::new(HttpMethod::Head, ValidatedUrl::new(url)?))
    }

    pub fn options(url: impl Into<String>) -> Result<Self, HttpError> {
        Ok(Self::new(HttpMethod::Options, ValidatedUrl::new(url)?))
    }

    pub fn with_header(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, HttpError> {
        self.headers.insert(name, value)?;
        Ok(self)
    }

    pub fn with_body(mut self, body: Vec<u8>) -> Result<Self, HttpError> {
        if !self.method.has_request_body() {
            return Err(HttpError::InvalidRequest {
                reason: format!("{} requests cannot have a body", self.method.as_str()),
            });
        }

        if body.len() > MAX_REQUEST_BODY_SIZE {
            return Err(HttpError::BodyTooLarge {
                size: body.len(),
                max: MAX_REQUEST_BODY_SIZE,
            });
        }

        self.body = Some(body);
        Ok(self)
    }

    pub fn with_json<T: serde::Serialize>(mut self, value: &T) -> Result<Self, HttpError> {
        if !self.method.has_request_body() {
            return Err(HttpError::InvalidRequest {
                reason: format!("{} requests cannot have a body", self.method.as_str()),
            });
        }

        let body = serde_json::to_vec(value).map_err(|e| HttpError::SerializationError {
            message: e.to_string(),
        })?;

        if body.len() > MAX_REQUEST_BODY_SIZE {
            return Err(HttpError::BodyTooLarge {
                size: body.len(),
                max: MAX_REQUEST_BODY_SIZE,
            });
        }

        self.headers
            .insert("Content-Type", "application/json")
            .ok();
        self.body = Some(body);
        Ok(self)
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Result<Self, HttpError> {
        let ms = timeout.as_millis() as u64;
        if ms == 0 {
            return Err(HttpError::InvalidRequest {
                reason: "timeout cannot be zero".to_string(),
            });
        }
        if ms > MAX_TIMEOUT_MS {
            return Err(HttpError::InvalidRequest {
                reason: format!("timeout exceeds maximum of {}ms", MAX_TIMEOUT_MS),
            });
        }
        self.timeout_ms = ms;
        Ok(self)
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Result<Self, HttpError> {
        if timeout_ms == 0 {
            return Err(HttpError::InvalidRequest {
                reason: "timeout cannot be zero".to_string(),
            });
        }
        if timeout_ms > MAX_TIMEOUT_MS {
            return Err(HttpError::InvalidRequest {
                reason: format!("timeout exceeds maximum of {}ms", MAX_TIMEOUT_MS),
            });
        }
        self.timeout_ms = timeout_ms;
        Ok(self)
    }

    pub fn with_retry(mut self, config: RetryConfig) -> Self {
        if !self.method.is_idempotent() && config.max_retries > 0 {
            self.retry_config = None;
        } else {
            self.retry_config = Some(config);
        }
        self
    }

    pub fn with_max_response_size(mut self, max_bytes: usize) -> Self {
        self.max_response_size = max_bytes.min(MAX_RESPONSE_BODY_SIZE);
        self
    }

    pub fn method(&self) -> HttpMethod {
        self.method
    }

    pub fn url(&self) -> &ValidatedUrl {
        &self.url
    }

    pub fn headers(&self) -> &HttpHeaders {
        &self.headers
    }

    pub fn body(&self) -> Option<&[u8]> {
        self.body.as_deref()
    }

    pub fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }

    pub fn retry_config(&self) -> Option<&RetryConfig> {
        self.retry_config.as_ref()
    }

    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    pub fn max_response_size(&self) -> usize {
        self.max_response_size
    }

    pub fn is_idempotent(&self) -> bool {
        self.method.is_idempotent()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HttpOperation {
    Execute(HttpRequest),
}

impl HttpOperation {
    pub fn get(url: impl Into<String>) -> Result<Self, HttpError> {
        Ok(Self::Execute(HttpRequest::get(url)?))
    }

    pub fn post(url: impl Into<String>) -> Result<Self, HttpError> {
        Ok(Self::Execute(HttpRequest::post(url)?))
    }

    pub fn put(url: impl Into<String>) -> Result<Self, HttpError> {
        Ok(Self::Execute(HttpRequest::put(url)?))
    }

    pub fn patch(url: impl Into<String>) -> Result<Self, HttpError> {
        Ok(Self::Execute(HttpRequest::patch(url)?))
    }

    pub fn delete(url: impl Into<String>) -> Result<Self, HttpError> {
        Ok(Self::Execute(HttpRequest::delete(url)?))
    }
}

#[derive(Debug, Clone, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum HttpError {
    #[error("invalid URL '{url}': {reason}")]
    InvalidUrl { url: String, reason: String },

    #[error("private network access blocked: {url} resolves to {host}")]
    PrivateNetworkBlocked { url: String, host: String },

    #[error("invalid header '{name}': {reason}")]
    InvalidHeader { name: String, reason: String },

    #[error("too many headers: {count} exceeds maximum of {max}")]
    TooManyHeaders { count: usize, max: usize },

    #[error("request body too large: {size} bytes exceeds maximum of {max} bytes")]
    BodyTooLarge { size: usize, max: usize },

    #[error("response body too large: {size} bytes exceeds maximum of {max} bytes")]
    ResponseTooLarge { size: usize, max: usize },

    #[error("invalid request: {reason}")]
    InvalidRequest { reason: String },

    #[error("serialization error: {message}")]
    SerializationError { message: String },

    #[error("DNS resolution failed for {host}: {message}")]
    DnsError { host: String, message: String },

    #[error("connection failed to {host}: {message}")]
    ConnectionError { host: String, message: String },

    #[error("TLS error for {host}: {message}")]
    TlsError { host: String, message: String },

    #[error("timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64, request_id: String },

    #[error("HTTP error {status}: {message}")]
    HttpStatus {
        status: u16,
        message: String,
        request_id: String,
        retryable: bool,
    },

    #[error("request cancelled")]
    Cancelled { request_id: String },

    #[error("too many redirects (max: {max})")]
    TooManyRedirects { max: u32 },

    #[error("invalid response: {reason}")]
    InvalidResponse { reason: String, request_id: String },
}

impl HttpError {
    pub fn is_retryable(&self) -> bool {
        match self {
            HttpError::Timeout { .. } => true,
            HttpError::ConnectionError { .. } => true,
            HttpError::DnsError { .. } => true,
            HttpError::HttpStatus { retryable, .. } => *retryable,
            HttpError::TooManyRedirects { .. } => false,
            HttpError::Cancelled { .. } => false,
            _ => false,
        }
    }

    pub fn request_id(&self) -> Option<&str> {
        match self {
            HttpError::Timeout { request_id, .. } => Some(request_id),
            HttpError::HttpStatus { request_id, .. } => Some(request_id),
            HttpError::Cancelled { request_id } => Some(request_id),
            HttpError::InvalidResponse { request_id, .. } => Some(request_id),
            _ => None,
        }
    }

    pub fn is_client_error(&self) -> bool {
        matches!(self, HttpError::HttpStatus { status, .. } if (400..500).contains(status))
    }

    pub fn is_server_error(&self) -> bool {
        matches!(self, HttpError::HttpStatus { status, .. } if (500..600).contains(status))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HttpResponse {
    status: u16,
    headers: HttpHeaders,
    body: Vec<u8>,
    request_id: String,
    duration_ms: u64,
}

impl HttpResponse {
    pub fn new(
        status: u16,
        headers: HttpHeaders,
        body: Vec<u8>,
        request_id: String,
        duration_ms: u64,
    ) -> Self {
        Self {
            status,
            headers,
            body,
            request_id,
            duration_ms,
        }
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub fn is_redirect(&self) -> bool {
        (300..400).contains(&self.status)
    }

    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status)
    }

    pub fn is_server_error(&self) -> bool {
        (500..600).contains(&self.status)
    }

    pub fn headers(&self) -> &HttpHeaders {
        &self.headers
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name)
    }

    pub fn content_type(&self) -> Option<ContentType> {
        self.headers.get("content-type").map(ContentType::from_header)
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn body_string(&self) -> Result<String, HttpError> {
        String::from_utf8(self.body.clone()).map_err(|e| HttpError::InvalidResponse {
            reason: format!("body is not valid UTF-8: {}", e),
            request_id: self.request_id.clone(),
        })
    }

    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, HttpError> {
        serde_json::from_slice(&self.body).map_err(|e| HttpError::InvalidResponse {
            reason: format!("failed to parse JSON: {}", e),
            request_id: self.request_id.clone(),
        })
    }

    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    pub fn duration_ms(&self) -> u64 {
        self.duration_ms
    }
}

pub type HttpOutput = HttpResponse;
pub type HttpResult = Result<HttpResponse, HttpError>;

pub struct AllowedHosts {
    patterns: Vec<HostPattern>,
}

enum HostPattern {
    Exact(String),
    Suffix(String),
    Any,
}

impl AllowedHosts {
    pub fn any() -> Self {
        Self {
            patterns: vec![HostPattern::Any],
        }
    }

    pub fn none() -> Self {
        Self { patterns: vec![] }
    }

    pub fn new(patterns: Vec<String>) -> Self {
        let patterns = patterns
            .into_iter()
            .map(|p| {
                if p == "*" {
                    HostPattern::Any
                } else if p.starts_with("*.") {
                    HostPattern::Suffix(p[1..].to_lowercase())
                } else {
                    HostPattern::Exact(p.to_lowercase())
                }
            })
            .collect();
        Self { patterns }
    }

    pub fn is_allowed(&self, host: &str) -> bool {
        let host_lower = host.to_lowercase();
        for pattern in &self.patterns {
            match pattern {
                HostPattern::Any => return true,
                HostPattern::Exact(h) => {
                    if &host_lower == h {
                        return true;
                    }
                }
                HostPattern::Suffix(suffix) => {
                    if host_lower.ends_with(suffix) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

impl Default for AllowedHosts {
    fn default() -> Self {
        Self::any()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_validation_empty() {
        let result = ValidatedUrl::new("");
        assert!(result.is_err());
    }

    #[test]
    fn test_url_validation_whitespace() {
        let result = ValidatedUrl::new("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_url_validation_invalid_scheme() {
        let result = ValidatedUrl::new("ftp://example.com");
        assert!(result.is_err());
        assert!(matches!(result, Err(HttpError::InvalidUrl { .. })));
    }

    #[test]
    fn test_url_validation_javascript() {
        let result = ValidatedUrl::new("javascript:alert(1)");
        assert!(result.is_err());
    }

    #[test]
    fn test_url_validation_data() {
        let result = ValidatedUrl::new("data:text/html,<script>x</script>");
        assert!(result.is_err());
    }

    #[test]
    fn test_url_validation_file() {
        let result = ValidatedUrl::new("file:///etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_url_validation_localhost_blocked() {
        let result = ValidatedUrl::new("http://localhost/api");
        assert!(result.is_err());
        assert!(matches!(result, Err(HttpError::PrivateNetworkBlocked { .. })));
    }

    #[test]
    fn test_url_validation_127_blocked() {
        let result = ValidatedUrl::new("http://127.0.0.1/api");
        assert!(result.is_err());
    }

    #[test]
    fn test_url_validation_private_ip_blocked() {
        let result = ValidatedUrl::new("http://192.168.1.1/admin");
        assert!(result.is_err());

        let result = ValidatedUrl::new("http://10.0.0.1/internal");
        assert!(result.is_err());

        let result = ValidatedUrl::new("http://172.16.0.1/secret");
        assert!(result.is_err());
    }

    #[test]
    fn test_url_validation_aws_metadata_blocked() {
        let result = ValidatedUrl::new("http://169.254.169.254/latest/meta-data/");
        assert!(result.is_err());
    }

    #[test]
    fn test_url_validation_credentials_blocked() {
        let result = ValidatedUrl::new("http://user:pass@example.com/");
        assert!(result.is_err());
    }

    #[test]
    fn test_url_validation_valid() {
        let result = ValidatedUrl::new("https://api.example.com/v1/users");
        assert!(result.is_ok());
        let url = result.unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host(), "api.example.com");
    }

    #[test]
    fn test_url_validation_too_long() {
        let long_url = format!("https://example.com/{}", "a".repeat(MAX_URL_LENGTH));
        let result = ValidatedUrl::new(long_url);
        assert!(result.is_err());
    }

    #[test]
    fn test_header_validation_empty_name() {
        let mut headers = HttpHeaders::new();
        let result = headers.insert("", "value");
        assert!(result.is_err());
    }

    #[test]
    fn test_header_validation_invalid_chars() {
        let mut headers = HttpHeaders::new();
        let result = headers.insert("Header:Name", "value");
        assert!(result.is_err());
    }

    #[test]
    fn test_header_validation_crlf_injection() {
        let mut headers = HttpHeaders::new();
        let result = headers.insert("X-Custom", "value\r\nEvil: header");
        assert!(result.is_err());
    }

    #[test]
    fn test_header_validation_reserved() {
        let mut headers = HttpHeaders::new();
        let result = headers.insert("Host", "evil.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_header_case_insensitive() {
        let mut headers = HttpHeaders::new();
        headers.insert("Content-Type", "application/json").unwrap();
        assert_eq!(headers.get("content-type"), Some("application/json"));
        assert_eq!(headers.get("CONTENT-TYPE"), Some("application/json"));
    }

    #[test]
    fn test_header_deduplication() {
        let mut headers = HttpHeaders::new();
        headers.insert("Accept", "text/html").unwrap();
        headers.insert("accept", "application/json").unwrap();
        assert_eq!(headers.len(), 1);
        assert_eq!(headers.get("Accept"), Some("application/json"));
    }

    #[test]
    fn test_request_builder() {
        let request = HttpRequest::post("https://api.example.com/data")
            .unwrap()
            .with_header("Authorization", "Bearer token123")
            .unwrap()
            .with_json(&serde_json::json!({"key": "value"}))
            .unwrap()
            .with_timeout_ms(5000)
            .unwrap();

        assert_eq!(request.method(), HttpMethod::Post);
        assert_eq!(request.timeout_ms(), 5000);
        assert!(request.body().is_some());
    }

    #[test]
    fn test_request_body_on_get_fails() {
        let result = HttpRequest::get("https://example.com")
            .unwrap()
            .with_body(vec![1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_request_body_size_limit() {
        let large_body = vec![0u8; MAX_REQUEST_BODY_SIZE + 1];
        let result = HttpRequest::post("https://example.com")
            .unwrap()
            .with_body(large_body);
        assert!(result.is_err());
        assert!(matches!(result, Err(HttpError::BodyTooLarge { .. })));
    }

    #[test]
    fn test_timeout_validation() {
        let result = HttpRequest::get("https://example.com")
            .unwrap()
            .with_timeout_ms(0);
        assert!(result.is_err());

        let result = HttpRequest::get("https://example.com")
            .unwrap()
            .with_timeout_ms(MAX_TIMEOUT_MS + 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_method_properties() {
        assert!(HttpMethod::Get.is_idempotent());
        assert!(!HttpMethod::Post.is_idempotent());
        assert!(HttpMethod::Put.is_idempotent());
        assert!(HttpMethod::Delete.is_idempotent());

        assert!(!HttpMethod::Get.has_request_body());
        assert!(HttpMethod::Post.has_request_body());
        assert!(HttpMethod::Patch.has_request_body());

        assert!(HttpMethod::Get.has_response_body());
        assert!(!HttpMethod::Head.has_response_body());
    }

    #[test]
    fn test_retry_config_non_idempotent() {
        let request = HttpRequest::post("https://example.com")
            .unwrap()
            .with_retry(RetryConfig::default());

        assert!(request.retry_config().is_none());
    }

    #[test]
    fn test_retry_config_idempotent() {
        let request = HttpRequest::get("https://example.com")
            .unwrap()
            .with_retry(RetryConfig::default());

        assert!(request.retry_config().is_some());
    }

    #[test]
    fn test_error_retryable() {
        assert!(HttpError::Timeout {
            timeout_ms: 1000,
            request_id: "x".into()
        }
        .is_retryable());

        assert!(HttpError::ConnectionError {
            host: "x".into(),
            message: "y".into()
        }
        .is_retryable());

        assert!(HttpError::HttpStatus {
            status: 503,
            message: "x".into(),
            request_id: "y".into(),
            retryable: true
        }
        .is_retryable());

        assert!(!HttpError::HttpStatus {
            status: 400,
            message: "x".into(),
            request_id: "y".into(),
            retryable: false
        }
        .is_retryable());

        assert!(!HttpError::InvalidUrl {
            url: "x".into(),
            reason: "y".into()
        }
        .is_retryable());
    }

    #[test]
    fn test_response_helpers() {
        let response = HttpResponse::new(
            200,
            HttpHeaders::new(),
            b"test".to_vec(),
            "req-1".into(),
            100,
        );

        assert!(response.is_success());
        assert!(!response.is_client_error());
        assert!(!response.is_server_error());
    }

    #[test]
    fn test_response_json_parsing() {
        let body = serde_json::to_vec(&serde_json::json!({"id": 123})).unwrap();
        let response = HttpResponse::new(200, HttpHeaders::new(), body, "req-1".into(), 100);

        let parsed: serde_json::Value = response.json().unwrap();
        assert_eq!(parsed["id"], 123);
    }

    #[test]
    fn test_allowed_hosts() {
        let allowed = AllowedHosts::new(vec![
            "api.example.com".into(),
            "*.trusted.com".into(),
        ]);

        assert!(allowed.is_allowed("api.example.com"));
        assert!(allowed.is_allowed("API.EXAMPLE.COM"));
        assert!(allowed.is_allowed("sub.trusted.com"));
        assert!(allowed.is_allowed("deep.sub.trusted.com"));
        assert!(!allowed.is_allowed("evil.com"));
        assert!(!allowed.is_allowed("example.com"));
    }

    #[test]
    fn test_content_type_parsing() {
        assert!(matches!(
            ContentType::from_header("application/json; charset=utf-8"),
            ContentType::Json
        ));
        assert!(matches!(
            ContentType::from_header("text/plain"),
            ContentType::Text
        ));
    }
}