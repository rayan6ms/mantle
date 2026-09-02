use std::fmt;
use std::io::{self, Read};
use std::str::FromStr;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use ureq::http::header::{ACCEPT_ENCODING, CONTENT_ENCODING, RETRY_AFTER};
use ureq::http::{HeaderMap, HeaderName, HeaderValue, Uri};
use ureq::{Agent, Error as UreqError};

use crate::http_input::{
    MAX_CONFIGURED_REDIRECTS, MAX_CONFIGURED_RETRIES, create_agent_with_route_policy,
    is_blocked_destination_error,
};
use crate::{HttpNetworkAccess, MediaCancellation, OutboundRoutePolicy};

const RETRY_WAIT_POLL_INTERVAL: Duration = Duration::from_millis(10);
const MAX_CONFIGURED_REQUEST_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CONFIGURED_RESPONSE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CONFIGURED_HEADER_BYTES: usize = 1024 * 1024;
const MAX_CONFIGURED_SOCKET_BUFFER_BYTES: usize = 1024 * 1024;
const MAX_CONFIGURED_TIMEOUT: Duration = Duration::from_mins(5);
const MAX_CONFIGURED_RETRY_DELAY: Duration = Duration::from_mins(5);

/// Resource, network, and retry policy for remote-source control-plane requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemoteHttpOptions {
    pub max_request_bytes: u64,
    pub max_response_bytes: u64,
    pub max_response_header_bytes: usize,
    pub socket_buffer_bytes: usize,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub max_redirects: u32,
    pub max_retries: u32,
    pub retry_base_delay: Duration,
    pub retry_max_delay: Duration,
    pub retry_after_max_delay: Duration,
    pub network_access: HttpNetworkAccess,
}

impl Default for RemoteHttpOptions {
    fn default() -> Self {
        Self {
            max_request_bytes: 1024 * 1024,
            max_response_bytes: 8 * 1024 * 1024,
            max_response_header_bytes: 32 * 1024,
            socket_buffer_bytes: 64 * 1024,
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
            max_redirects: 5,
            max_retries: 2,
            retry_base_delay: Duration::from_millis(250),
            retry_max_delay: Duration::from_secs(5),
            retry_after_max_delay: Duration::from_secs(30),
            network_access: HttpNetworkAccess::PublicInternetOnly,
        }
    }
}

impl RemoteHttpOptions {
    fn validate(self) -> Result<Self, RemoteHttpError> {
        if self.max_request_bytes == 0
            || self.max_request_bytes > MAX_CONFIGURED_REQUEST_BYTES
            || self.max_response_bytes == 0
            || self.max_response_bytes > MAX_CONFIGURED_RESPONSE_BYTES
            || self.max_response_header_bytes < 1024
            || self.max_response_header_bytes > MAX_CONFIGURED_HEADER_BYTES
            || self.socket_buffer_bytes < 1024
            || self.socket_buffer_bytes > MAX_CONFIGURED_SOCKET_BUFFER_BYTES
            || self.connect_timeout.is_zero()
            || self.connect_timeout > MAX_CONFIGURED_TIMEOUT
            || self.request_timeout.is_zero()
            || self.request_timeout > MAX_CONFIGURED_TIMEOUT
            || self.retry_base_delay.is_zero()
            || self.retry_max_delay < self.retry_base_delay
            || self.retry_max_delay > MAX_CONFIGURED_RETRY_DELAY
            || self.retry_after_max_delay.is_zero()
            || self.retry_after_max_delay > MAX_CONFIGURED_RETRY_DELAY
            || self.max_redirects > MAX_CONFIGURED_REDIRECTS
            || self.max_retries > MAX_CONFIGURED_RETRIES
        {
            return Err(RemoteHttpError::new(RemoteHttpErrorKind::InvalidOptions));
        }
        Ok(self)
    }
}

/// Whether a request may be repeated after a transient failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteRetryMode {
    /// Never repeat the request.
    Never,
    /// The caller guarantees that repeating the request is safe.
    Idempotent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RemoteHttpMethod {
    Get,
    Post,
}

impl RemoteHttpMethod {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }
}

/// A bounded remote-source request whose diagnostics omit its URL, body, and header values.
pub struct RemoteHttpRequest {
    method: RemoteHttpMethod,
    uri: Uri,
    headers: HeaderMap,
    body: Vec<u8>,
    retry_mode: RemoteRetryMode,
    max_response_bytes: Option<u64>,
}

impl RemoteHttpRequest {
    /// Creates an idempotent GET request.
    ///
    /// # Errors
    ///
    /// Returns [`RemoteHttpErrorKind::InvalidRequest`] for a malformed, non-HTTP(S), or
    /// user-information-bearing URL.
    pub fn get(url: impl AsRef<str>) -> Result<Self, RemoteHttpError> {
        Ok(Self {
            method: RemoteHttpMethod::Get,
            uri: parse_remote_uri(url.as_ref())?,
            headers: HeaderMap::new(),
            body: Vec::new(),
            retry_mode: RemoteRetryMode::Idempotent,
            max_response_bytes: None,
        })
    }

    /// Creates a POST request which is not retried unless explicitly marked idempotent.
    ///
    /// # Errors
    ///
    /// Returns [`RemoteHttpErrorKind::InvalidRequest`] for a malformed, non-HTTP(S), or
    /// user-information-bearing URL, and [`RemoteHttpErrorKind::RequestTooLarge`] when the body
    /// exceeds the hard request ceiling.
    pub fn post(url: impl AsRef<str>, body: impl AsRef<[u8]>) -> Result<Self, RemoteHttpError> {
        let body = body.as_ref();
        if u64::try_from(body.len()).unwrap_or(u64::MAX) > MAX_CONFIGURED_REQUEST_BYTES {
            return Err(RemoteHttpError::new(RemoteHttpErrorKind::RequestTooLarge));
        }
        Ok(Self {
            method: RemoteHttpMethod::Post,
            uri: parse_remote_uri(url.as_ref())?,
            headers: HeaderMap::new(),
            body: body.to_vec(),
            retry_mode: RemoteRetryMode::Never,
            max_response_bytes: None,
        })
    }

    /// Appends a validated header. Header values are never included in `Debug` output.
    ///
    /// # Errors
    ///
    /// Returns [`RemoteHttpErrorKind::InvalidRequest`] for invalid syntax or a transport-owned
    /// framing, host, connection, or content-encoding header.
    pub fn header(mut self, name: &str, value: &str) -> Result<Self, RemoteHttpError> {
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| RemoteHttpError::new(RemoteHttpErrorKind::InvalidRequest))?;
        if is_transport_owned_header(&name) {
            return Err(RemoteHttpError::new(RemoteHttpErrorKind::InvalidRequest));
        }
        let value = HeaderValue::from_str(value)
            .map_err(|_| RemoteHttpError::new(RemoteHttpErrorKind::InvalidRequest))?;
        self.headers.append(name, value);
        Ok(self)
    }

    /// Sets whether a transient failure may safely repeat this request.
    #[must_use]
    pub fn retry_mode(mut self, retry_mode: RemoteRetryMode) -> Self {
        self.retry_mode = retry_mode;
        self
    }

    /// Applies a response-body ceiling smaller than the client's global ceiling.
    ///
    /// # Errors
    ///
    /// Returns [`RemoteHttpErrorKind::InvalidRequest`] for zero or a value above the hard
    /// response ceiling. Execution also rejects a request ceiling above the client's policy.
    pub fn max_response_bytes(mut self, max_response_bytes: u64) -> Result<Self, RemoteHttpError> {
        if max_response_bytes == 0 || max_response_bytes > MAX_CONFIGURED_RESPONSE_BYTES {
            return Err(RemoteHttpError::new(RemoteHttpErrorKind::InvalidRequest));
        }
        self.max_response_bytes = Some(max_response_bytes);
        Ok(self)
    }
}

impl fmt::Debug for RemoteHttpRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RemoteHttpRequest")
            .field("method", &self.method.as_str())
            .field("header_count", &self.headers.len())
            .field("body_len", &self.body.len())
            .field("retry_mode", &self.retry_mode)
            .field("max_response_bytes", &self.max_response_bytes)
            .finish_non_exhaustive()
    }
}

/// A successful bounded response. Its diagnostics omit body and header values.
pub struct RemoteHttpResponse {
    status: u16,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl RemoteHttpResponse {
    #[must_use]
    pub const fn status(&self) -> u16 {
        self.status
    }

    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    #[must_use]
    pub fn into_body(self) -> Vec<u8> {
        self.body
    }

    /// Returns one ASCII response-header value, if present and unambiguous.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        let name = HeaderName::from_bytes(name.as_bytes()).ok()?;
        let mut values = self.headers.get_all(name).iter();
        let value = values.next()?;
        if values.next().is_some() {
            return None;
        }
        value.to_str().ok()
    }
}

impl fmt::Debug for RemoteHttpResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RemoteHttpResponse")
            .field("status", &self.status)
            .field("header_count", &self.headers.len())
            .field("body_len", &self.body.len())
            .finish()
    }
}

/// Stable source-facing classes for all remote HTTP failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteHttpErrorKind {
    InvalidOptions,
    InvalidRequest,
    RequestTooLarge,
    Cancelled,
    DestinationDenied,
    Timeout,
    Transport,
    ResponseTooLarge,
    InvalidResponse,
    Unauthorized,
    Forbidden,
    NotFound,
    RateLimited,
    ServerUnavailable,
    HttpStatus,
}

/// A credential-safe remote-source error with optional status and retry metadata.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RemoteHttpError {
    kind: RemoteHttpErrorKind,
    status: Option<u16>,
    retry_after: Option<Duration>,
}

impl RemoteHttpError {
    const fn new(kind: RemoteHttpErrorKind) -> Self {
        Self {
            kind,
            status: None,
            retry_after: None,
        }
    }

    const fn status(kind: RemoteHttpErrorKind, status: u16) -> Self {
        Self {
            kind,
            status: Some(status),
            retry_after: None,
        }
    }

    #[must_use]
    pub const fn kind(&self) -> RemoteHttpErrorKind {
        self.kind
    }

    #[must_use]
    pub const fn status_code(&self) -> Option<u16> {
        self.status
    }

    #[must_use]
    pub const fn retry_after(&self) -> Option<Duration> {
        self.retry_after
    }
}

impl fmt::Debug for RemoteHttpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RemoteHttpError")
            .field("kind", &self.kind)
            .field("status", &self.status)
            .field("retry_after", &self.retry_after)
            .finish()
    }
}

impl fmt::Display for RemoteHttpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            RemoteHttpErrorKind::InvalidOptions => "invalid remote HTTP policy",
            RemoteHttpErrorKind::InvalidRequest => "invalid remote HTTP request",
            RemoteHttpErrorKind::RequestTooLarge => "remote request body exceeds its limit",
            RemoteHttpErrorKind::Cancelled => "remote request cancelled",
            RemoteHttpErrorKind::DestinationDenied => {
                "remote destination rejected by network access policy"
            }
            RemoteHttpErrorKind::Timeout => "remote request timed out",
            RemoteHttpErrorKind::Transport => "remote request transport failed",
            RemoteHttpErrorKind::ResponseTooLarge => "remote response body exceeds its limit",
            RemoteHttpErrorKind::InvalidResponse => "remote response is invalid",
            RemoteHttpErrorKind::Unauthorized => "remote service rejected authentication",
            RemoteHttpErrorKind::Forbidden => "remote service denied the request",
            RemoteHttpErrorKind::NotFound => "remote resource was not found",
            RemoteHttpErrorKind::RateLimited => "remote service rate limit reached",
            RemoteHttpErrorKind::ServerUnavailable => "remote service is unavailable",
            RemoteHttpErrorKind::HttpStatus => "remote service returned an unsuccessful status",
        })
    }
}

impl std::error::Error for RemoteHttpError {}

/// Reusable, connection-pooled client for source control-plane requests.
pub struct RemoteHttpClient {
    agent: Agent,
    non_redirecting_agent: Agent,
    options: RemoteHttpOptions,
    route_policy: Option<Arc<dyn OutboundRoutePolicy>>,
}

impl RemoteHttpClient {
    /// Creates a client after validating all resource and retry bounds.
    ///
    /// # Errors
    ///
    /// Returns [`RemoteHttpErrorKind::InvalidOptions`] when any configured resource, timeout,
    /// redirect, retry, or backoff bound is invalid.
    pub fn new(options: RemoteHttpOptions) -> Result<Self, RemoteHttpError> {
        Self::new_inner(options, None)
    }

    /// Creates a client whose new connections select and bind an outbound route.
    ///
    /// Routed clients disable idle reuse because ureq's pool key cannot include the opaque route
    /// identity. This prevents a later selection from reusing a socket bound to an earlier IP.
    ///
    /// # Errors
    ///
    /// Returns [`RemoteHttpErrorKind::InvalidOptions`] for invalid bounded HTTP policy.
    pub fn with_route_policy(
        options: RemoteHttpOptions,
        route_policy: Arc<dyn OutboundRoutePolicy>,
    ) -> Result<Self, RemoteHttpError> {
        Self::new_inner(options, Some(route_policy))
    }

    fn new_inner(
        options: RemoteHttpOptions,
        route_policy: Option<Arc<dyn OutboundRoutePolicy>>,
    ) -> Result<Self, RemoteHttpError> {
        let options = options.validate()?;
        let agent = create_agent_with_route_policy(
            options.max_response_header_bytes,
            options.socket_buffer_bytes,
            options.connect_timeout,
            options.request_timeout,
            options.max_redirects,
            options.network_access,
            route_policy.clone(),
        );
        let non_redirecting_agent = create_agent_with_route_policy(
            options.max_response_header_bytes,
            options.socket_buffer_bytes,
            options.connect_timeout,
            options.request_timeout,
            0,
            options.network_access,
            route_policy.clone(),
        );
        Ok(Self {
            agent,
            non_redirecting_agent,
            options,
            route_policy,
        })
    }

    /// Executes with a fresh cancellation token.
    ///
    /// # Errors
    ///
    /// Returns a classified [`RemoteHttpError`] for policy, transport, status, or bounded-body
    /// failures.
    pub fn execute(
        &self,
        request: &RemoteHttpRequest,
    ) -> Result<RemoteHttpResponse, RemoteHttpError> {
        self.execute_with_cancellation(request, &MediaCancellation::new())
    }

    /// Executes a request with bounded retries and cooperative cancellation.
    ///
    /// # Errors
    ///
    /// Returns [`RemoteHttpErrorKind::Cancelled`] when cancellation is observed, or another
    /// classified [`RemoteHttpError`] for policy, transport, status, or bounded-body failures.
    pub fn execute_with_cancellation(
        &self,
        request: &RemoteHttpRequest,
        cancellation: &MediaCancellation,
    ) -> Result<RemoteHttpResponse, RemoteHttpError> {
        if u64::try_from(request.body.len()).unwrap_or(u64::MAX) > self.options.max_request_bytes {
            return Err(RemoteHttpError::new(RemoteHttpErrorKind::RequestTooLarge));
        }
        let max_response_bytes = request
            .max_response_bytes
            .unwrap_or(self.options.max_response_bytes);
        if max_response_bytes > self.options.max_response_bytes {
            return Err(RemoteHttpError::new(RemoteHttpErrorKind::InvalidRequest));
        }
        let mut retries = 0;
        loop {
            check_cancellation(cancellation)?;
            match self.call(request) {
                Ok(response) => {
                    let status = response.status().as_u16();
                    let retry_after = parse_retry_after(response.headers(), self.options);
                    if is_retriable_status(status)
                        && request.retry_mode == RemoteRetryMode::Idempotent
                        && retries < self.options.max_retries
                    {
                        drop(response);
                        let delay = self.retry_delay(retries, retry_after);
                        retries += 1;
                        wait_for_retry(delay, cancellation)?;
                        continue;
                    }
                    if !(200..300).contains(&status) {
                        return Err(classify_status(status, retry_after));
                    }
                    return read_response(response, max_response_bytes, cancellation);
                }
                Err(error) => {
                    if is_retriable_transport_error(&error)
                        && request.retry_mode == RemoteRetryMode::Idempotent
                        && retries < self.options.max_retries
                    {
                        let delay = self.retry_delay(retries, None);
                        retries += 1;
                        wait_for_retry(delay, cancellation)?;
                        continue;
                    }
                    return Err(classify_transport_error(&error));
                }
            }
        }
    }

    fn call(
        &self,
        request: &RemoteHttpRequest,
    ) -> Result<ureq::http::Response<ureq::Body>, UreqError> {
        // Caller header values can include service-specific credentials whose names are not known
        // centrally. Never let an automatic redirect copy them to another request.
        let agent = if request.headers.is_empty() {
            &self.agent
        } else {
            &self.non_redirecting_agent
        };
        match request.method {
            RemoteHttpMethod::Get => {
                let builder = apply_headers(
                    agent
                        .get(request.uri.clone())
                        .header(ACCEPT_ENCODING, "identity"),
                    &request.headers,
                );
                builder.call()
            }
            RemoteHttpMethod::Post => {
                let builder = apply_headers(
                    agent
                        .post(request.uri.clone())
                        .header(ACCEPT_ENCODING, "identity"),
                    &request.headers,
                );
                builder.send(request.body.as_slice())
            }
        }
    }

    fn retry_delay(&self, retry: u32, retry_after: Option<Duration>) -> Duration {
        let factor = 1_u32.checked_shl(retry.min(31)).unwrap_or(u32::MAX);
        let exponential = self
            .options
            .retry_base_delay
            .checked_mul(factor)
            .unwrap_or(self.options.retry_max_delay)
            .min(self.options.retry_max_delay);
        retry_after.map_or(exponential, |delay| delay.max(exponential))
    }
}

impl fmt::Debug for RemoteHttpClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RemoteHttpClient")
            .field("options", &self.options)
            .field("routed", &self.route_policy.is_some())
            .finish_non_exhaustive()
    }
}

fn apply_headers<B>(
    mut builder: ureq::RequestBuilder<B>,
    headers: &HeaderMap,
) -> ureq::RequestBuilder<B> {
    for (name, value) in headers {
        builder = builder.header(name.clone(), value.clone());
    }
    builder
}

fn parse_remote_uri(url: &str) -> Result<Uri, RemoteHttpError> {
    let uri = Uri::from_str(url)
        .map_err(|_| RemoteHttpError::new(RemoteHttpErrorKind::InvalidRequest))?;
    if !matches!(uri.scheme_str(), Some("http" | "https"))
        || uri.authority().is_none()
        || uri
            .authority()
            .is_some_and(|authority| authority.as_str().contains('@'))
    {
        return Err(RemoteHttpError::new(RemoteHttpErrorKind::InvalidRequest));
    }
    Ok(uri)
}

fn is_transport_owned_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "host" | "content-length" | "transfer-encoding" | "connection" | "accept-encoding"
    )
}

fn parse_retry_after(headers: &HeaderMap, options: RemoteHttpOptions) -> Option<Duration> {
    let mut values = headers.get_all(RETRY_AFTER).iter();
    let value = values.next()?;
    if values.next().is_some() {
        return None;
    }
    let seconds = value.to_str().ok()?.parse::<u64>().ok()?;
    Some(Duration::from_secs(seconds).min(options.retry_after_max_delay))
}

fn is_retriable_status(status: u16) -> bool {
    matches!(status, 408 | 425 | 429 | 500 | 502 | 503 | 504)
}

fn classify_status(status: u16, retry_after: Option<Duration>) -> RemoteHttpError {
    let kind = match status {
        401 => RemoteHttpErrorKind::Unauthorized,
        403 => RemoteHttpErrorKind::Forbidden,
        404 => RemoteHttpErrorKind::NotFound,
        429 => RemoteHttpErrorKind::RateLimited,
        500..=599 => RemoteHttpErrorKind::ServerUnavailable,
        _ => RemoteHttpErrorKind::HttpStatus,
    };
    let mut error = RemoteHttpError::status(kind, status);
    if kind == RemoteHttpErrorKind::RateLimited {
        error.retry_after = retry_after;
    }
    error
}

fn is_retriable_transport_error(error: &UreqError) -> bool {
    match error {
        UreqError::Timeout(_) => true,
        UreqError::Io(source) => matches!(
            source.kind(),
            io::ErrorKind::ConnectionAborted
                | io::ErrorKind::ConnectionRefused
                | io::ErrorKind::ConnectionReset
                | io::ErrorKind::TimedOut
                | io::ErrorKind::UnexpectedEof
        ),
        _ => false,
    }
}

fn classify_transport_error(error: &UreqError) -> RemoteHttpError {
    let kind = if is_blocked_destination_error(error) {
        RemoteHttpErrorKind::DestinationDenied
    } else {
        match error {
            UreqError::Timeout(_) => RemoteHttpErrorKind::Timeout,
            UreqError::BadUri(_) | UreqError::Http(_) => RemoteHttpErrorKind::InvalidRequest,
            UreqError::BodyExceedsLimit(_) => RemoteHttpErrorKind::RequestTooLarge,
            _ => RemoteHttpErrorKind::Transport,
        }
    };
    RemoteHttpError::new(kind)
}

fn read_response(
    response: ureq::http::Response<ureq::Body>,
    max_response_bytes: u64,
    cancellation: &MediaCancellation,
) -> Result<RemoteHttpResponse, RemoteHttpError> {
    if response
        .headers()
        .get_all(CONTENT_ENCODING)
        .iter()
        .any(|value| {
            !value
                .to_str()
                .is_ok_and(|value| value.eq_ignore_ascii_case("identity"))
        })
    {
        return Err(RemoteHttpError::new(RemoteHttpErrorKind::InvalidResponse));
    }
    if response
        .body()
        .content_length()
        .is_some_and(|length| length > max_response_bytes)
    {
        return Err(RemoteHttpError::new(RemoteHttpErrorKind::ResponseTooLarge));
    }
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let initial_capacity = response
        .body()
        .content_length()
        .and_then(|length| usize::try_from(length).ok())
        .unwrap_or(0);
    let mut reader = response.into_body().into_reader();
    let mut body = Vec::with_capacity(initial_capacity);
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        check_cancellation(cancellation)?;
        let remaining = max_response_bytes.saturating_sub(body.len() as u64);
        let allowed = usize::try_from(remaining)
            .unwrap_or(usize::MAX)
            .min(buffer.len());
        if allowed == 0 {
            let count = reader
                .read(&mut buffer[..1])
                .map_err(|error| classify_body_error(&error))?;
            check_cancellation(cancellation)?;
            if count == 0 {
                break;
            }
            return Err(RemoteHttpError::new(RemoteHttpErrorKind::ResponseTooLarge));
        }
        let count = reader
            .read(&mut buffer[..allowed])
            .map_err(|error| classify_body_error(&error))?;
        check_cancellation(cancellation)?;
        if count == 0 {
            break;
        }
        body.extend_from_slice(&buffer[..count]);
    }
    Ok(RemoteHttpResponse {
        status,
        headers,
        body,
    })
}

fn classify_body_error(error: &io::Error) -> RemoteHttpError {
    if error.kind() == io::ErrorKind::TimedOut {
        RemoteHttpError::new(RemoteHttpErrorKind::Timeout)
    } else {
        RemoteHttpError::new(RemoteHttpErrorKind::InvalidResponse)
    }
}

fn check_cancellation(cancellation: &MediaCancellation) -> Result<(), RemoteHttpError> {
    if cancellation.is_cancelled() {
        Err(RemoteHttpError::new(RemoteHttpErrorKind::Cancelled))
    } else {
        Ok(())
    }
}

fn wait_for_retry(
    delay: Duration,
    cancellation: &MediaCancellation,
) -> Result<(), RemoteHttpError> {
    let deadline = Instant::now()
        .checked_add(delay)
        .ok_or_else(|| RemoteHttpError::new(RemoteHttpErrorKind::InvalidOptions))?;
    loop {
        check_cancellation(cancellation)?;
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(());
        }
        thread::sleep(remaining.min(RETRY_WAIT_POLL_INTERVAL));
    }
}
