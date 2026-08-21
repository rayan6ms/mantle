use std::fmt;
use std::io::{self, Read, Seek, SeekFrom};
use std::net::IpAddr;
use std::time::Duration;

use ureq::http::header::{
    ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_RANGE, ETAG, IF_RANGE,
    LAST_MODIFIED, RANGE,
};
use ureq::http::{HeaderMap, HeaderName, Uri};
use ureq::unversioned::resolver::{DefaultResolver, ResolvedSocketAddrs, Resolver};
use ureq::unversioned::transport::{DefaultConnector, NextTimeout};
use ureq::{Agent, BodyReader, Error as UreqError, ResponseExt};

use crate::{MediaCancellation, MediaError, MediaInput};

pub(crate) const MAX_CONFIGURED_REDIRECTS: u32 = 16;
pub(crate) const MAX_CONFIGURED_RETRIES: u32 = 8;

/// Controls whether the HTTP resolver may return non-public destination addresses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HttpNetworkAccess {
    /// Permit only publicly routable Internet addresses.
    #[default]
    PublicInternetOnly,
    /// Permit private, loopback, link-local, and otherwise non-public addresses.
    ///
    /// This is intended for explicitly trusted deployments and deterministic loopback tests.
    AllowPrivateNetworks,
}

/// Resource and network policy for a seekable HTTP range input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HttpRangeOptions {
    pub range_window_bytes: usize,
    pub max_source_bytes: u64,
    pub max_response_header_bytes: usize,
    pub socket_buffer_bytes: usize,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub max_redirects: u32,
    pub max_retries: u32,
    pub network_access: HttpNetworkAccess,
}

impl Default for HttpRangeOptions {
    fn default() -> Self {
        Self {
            range_window_bytes: 256 * 1024,
            max_source_bytes: 64 * 1024 * 1024 * 1024,
            max_response_header_bytes: 32 * 1024,
            socket_buffer_bytes: 64 * 1024,
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
            max_redirects: 5,
            max_retries: 1,
            network_access: HttpNetworkAccess::PublicInternetOnly,
        }
    }
}

impl HttpRangeOptions {
    fn validate(self) -> Result<Self, MediaError> {
        if self.range_window_bytes == 0 {
            return Err(MediaError::InvalidHttpOptions(
                "range_window_bytes must be non-zero",
            ));
        }
        if self.max_source_bytes == 0 {
            return Err(MediaError::InvalidHttpOptions(
                "max_source_bytes must be non-zero",
            ));
        }
        if self.max_response_header_bytes < 1024 {
            return Err(MediaError::InvalidHttpOptions(
                "max_response_header_bytes must be at least 1 KiB",
            ));
        }
        if self.socket_buffer_bytes < 1024 {
            return Err(MediaError::InvalidHttpOptions(
                "socket_buffer_bytes must be at least 1 KiB",
            ));
        }
        if self.connect_timeout.is_zero() {
            return Err(MediaError::InvalidHttpOptions(
                "connect_timeout must be non-zero",
            ));
        }
        if self.request_timeout.is_zero() {
            return Err(MediaError::InvalidHttpOptions(
                "request_timeout must be non-zero",
            ));
        }
        validate_request_counts(self.max_redirects, self.max_retries)?;
        Ok(self)
    }
}

/// A finite, seekable HTTP(S) object read through validated fixed-size byte ranges.
///
/// The source URL is deliberately not exposed through `Debug` or error messages.
pub struct HttpRangeInput {
    agent: Agent,
    uri: Uri,
    options: HttpRangeOptions,
    position: u64,
    source_len: u64,
    active: Option<ActiveRange>,
    validator: Option<Validator>,
    cancellation: MediaCancellation,
}

impl HttpRangeInput {
    /// Opens an HTTP(S) object and validates its first byte-range response.
    ///
    /// # Errors
    ///
    /// Returns an error when options or the URL are invalid, destination policy rejects every
    /// resolved address, the request fails, or the server does not return a bounded and internally
    /// consistent `206 Partial Content` response.
    pub fn open(url: impl AsRef<str>, options: HttpRangeOptions) -> Result<Self, MediaError> {
        Self::open_with_cancellation(url, options, MediaCancellation::new())
    }

    /// Opens an HTTP(S) object with a caller-owned cancellation signal.
    ///
    /// # Errors
    ///
    /// Returns [`MediaError::Cancelled`] when cancellation is already requested, in addition to
    /// the errors from [`Self::open`]. Reads and range reopens observe the same signal.
    pub fn open_with_cancellation(
        url: impl AsRef<str>,
        options: HttpRangeOptions,
        cancellation: MediaCancellation,
    ) -> Result<Self, MediaError> {
        let options = options.validate()?;
        cancellation.check()?;
        let uri = parse_uri(url.as_ref())?;
        let agent = create_agent(
            options.max_response_header_bytes,
            options.socket_buffer_bytes,
            options.connect_timeout,
            options.request_timeout,
            options.max_redirects,
            options.network_access,
        );
        let mut input = Self {
            agent,
            uri,
            options,
            position: 0,
            source_len: 0,
            active: None,
            validator: None,
            cancellation,
        };
        input.open_range()?;
        Ok(input)
    }

    #[must_use]
    pub fn final_uri(&self) -> &Uri {
        &self.uri
    }

    fn open_range(&mut self) -> io::Result<()> {
        self.cancellation.check_io()?;
        if self.source_len != 0 && self.position >= self.source_len {
            self.active = None;
            return Ok(());
        }
        let window = u64::try_from(self.options.range_window_bytes).unwrap_or(u64::MAX);
        let requested_end = self
            .position
            .saturating_add(window.saturating_sub(1))
            .min(self.options.max_source_bytes.saturating_sub(1));
        let range_value = format!("bytes={}-{}", self.position, requested_end);
        let agent = self.agent.clone();
        let uri = self.uri.clone();
        let validator = self.validator.clone();
        let response = call_with_retries(
            || {
                let mut request = agent
                    .get(uri.clone())
                    .header(RANGE, range_value.as_str())
                    .header(ACCEPT_ENCODING, "identity");
                if let Some(validator) = &validator {
                    request = request.header(IF_RANGE, validator.value.as_str());
                }
                request.call()
            },
            self.options.max_retries,
            &self.cancellation,
        )?;
        self.uri = response.get_uri().clone();
        let status = response.status().as_u16();
        if status != 206 {
            return Err(invalid_response(format_args!(
                "HTTP range request returned status {status}, expected 206"
            )));
        }
        reject_content_encoding(response.headers())?;
        let parsed = parse_content_range(response.headers())?;
        if parsed.start != self.position {
            return Err(invalid_response(format_args!(
                "Content-Range begins at {}, expected {}",
                parsed.start, self.position
            )));
        }
        if parsed.total == 0 || parsed.total > self.options.max_source_bytes {
            return Err(invalid_response(format_args!(
                "HTTP source length {} is outside the configured limit {}",
                parsed.total, self.options.max_source_bytes
            )));
        }
        if self.source_len != 0 && parsed.total != self.source_len {
            return Err(invalid_response(format_args!(
                "HTTP source length changed from {} to {}",
                self.source_len, parsed.total
            )));
        }
        let expected_end = requested_end.min(parsed.total - 1);
        if parsed.end != expected_end {
            return Err(invalid_response(format_args!(
                "Content-Range ends at {}, expected {expected_end}",
                parsed.end
            )));
        }
        let expected_len = parsed
            .end
            .checked_sub(parsed.start)
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| invalid_response(format_args!("invalid Content-Range span")))?;
        let content_len = parse_single_u64_header(response.headers(), &CONTENT_LENGTH)?;
        if content_len != expected_len || response.body().content_length() != Some(expected_len) {
            return Err(invalid_response(format_args!(
                "Content-Length does not match the {expected_len}-byte range"
            )));
        }
        let response_validator = read_validator(response.headers())?;
        if self.validator.is_some() && self.validator != response_validator {
            return Err(invalid_response(format_args!(
                "HTTP source validator changed between ranges"
            )));
        }
        if self.validator.is_none() {
            self.validator = response_validator;
        }
        self.source_len = parsed.total;
        self.active = Some(ActiveRange {
            reader: response.into_body().into_reader(),
            remaining: expected_len,
        });
        Ok(())
    }
}

impl Read for HttpRangeInput {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.cancellation.check_io()?;
        if buffer.is_empty() || self.position >= self.source_len {
            return Ok(0);
        }
        if self.active.is_none() {
            self.open_range()?;
        }
        let Some(active) = self.active.as_mut() else {
            return Ok(0);
        };
        let allowed = usize::try_from(active.remaining)
            .unwrap_or(usize::MAX)
            .min(buffer.len());
        let count = active
            .reader
            .read(&mut buffer[..allowed])
            .map_err(|error| sanitize_body_error(&error))?;
        self.cancellation.check_io()?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "HTTP range body ended before its declared length",
            ));
        }
        self.position = self.position.saturating_add(count as u64);
        active.remaining = active.remaining.saturating_sub(count as u64);
        if active.remaining == 0 {
            self.active = None;
        }
        Ok(count)
    }
}

impl Seek for HttpRangeInput {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.cancellation.check_io()?;
        let target = match position {
            SeekFrom::Start(offset) => i128::from(offset),
            SeekFrom::Current(offset) => i128::from(self.position) + i128::from(offset),
            SeekFrom::End(offset) => i128::from(self.source_len) + i128::from(offset),
        };
        if !(0..=i128::from(self.source_len)).contains(&target) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "HTTP seek is outside the source",
            ));
        }
        let target = u64::try_from(target).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "HTTP seek position is invalid")
        })?;
        if target != self.position {
            self.position = target;
            self.active = None;
        }
        Ok(self.position)
    }
}

/// Resource and network policy for a finite or bounded unknown-length HTTP response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HttpStreamOptions {
    pub max_response_bytes: u64,
    pub max_response_header_bytes: usize,
    pub socket_buffer_bytes: usize,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub max_redirects: u32,
    pub max_retries: u32,
    pub network_access: HttpNetworkAccess,
}

impl Default for HttpStreamOptions {
    fn default() -> Self {
        Self {
            max_response_bytes: 8 * 1024 * 1024,
            max_response_header_bytes: 32 * 1024,
            socket_buffer_bytes: 64 * 1024,
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
            max_redirects: 5,
            max_retries: 1,
            network_access: HttpNetworkAccess::PublicInternetOnly,
        }
    }
}

impl HttpStreamOptions {
    pub(crate) fn validate(self) -> Result<Self, MediaError> {
        if self.max_response_bytes == 0 {
            return Err(MediaError::InvalidHttpOptions(
                "max_response_bytes must be non-zero",
            ));
        }
        if self.max_response_header_bytes < 1024 {
            return Err(MediaError::InvalidHttpOptions(
                "max_response_header_bytes must be at least 1 KiB",
            ));
        }
        if self.socket_buffer_bytes < 1024 {
            return Err(MediaError::InvalidHttpOptions(
                "socket_buffer_bytes must be at least 1 KiB",
            ));
        }
        if self.connect_timeout.is_zero() || self.request_timeout.is_zero() {
            return Err(MediaError::InvalidHttpOptions(
                "HTTP stream timeouts must be non-zero",
            ));
        }
        validate_request_counts(self.max_redirects, self.max_retries)?;
        Ok(self)
    }
}

/// A non-seekable HTTP(S) response with a hard total-byte ceiling.
pub struct HttpStreamInput {
    reader: BodyReader<'static>,
    final_uri: Uri,
    declared_len: Option<u64>,
    position: u64,
    max_response_bytes: u64,
    cancellation: MediaCancellation,
}

impl HttpStreamInput {
    /// Opens a finite or unknown-length HTTP response.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid policy, rejected destinations, request failures, non-success
    /// status, non-identity encoding, or a declared length above the configured ceiling.
    pub fn open(url: impl AsRef<str>, options: HttpStreamOptions) -> Result<Self, MediaError> {
        Self::open_with_cancellation(url, options, MediaCancellation::new())
    }

    /// Opens a finite or unknown-length HTTP response with cancellation.
    ///
    /// # Errors
    ///
    /// Returns [`MediaError::Cancelled`] when cancellation is already requested, in addition to
    /// the errors from [`Self::open`]. Body reads observe the same signal.
    pub fn open_with_cancellation(
        url: impl AsRef<str>,
        options: HttpStreamOptions,
        cancellation: MediaCancellation,
    ) -> Result<Self, MediaError> {
        let options = options.validate()?;
        cancellation.check()?;
        let uri = parse_uri(url.as_ref())?;
        let agent = create_agent(
            options.max_response_header_bytes,
            options.socket_buffer_bytes,
            options.connect_timeout,
            options.request_timeout,
            options.max_redirects,
            options.network_access,
        );
        let response = call_with_retries(
            || {
                agent
                    .get(uri.clone())
                    .header(ACCEPT_ENCODING, "identity")
                    .call()
            },
            options.max_retries,
            &cancellation,
        )?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(MediaError::Io(invalid_response(format_args!(
                "HTTP stream request returned status {status}"
            ))));
        }
        reject_content_encoding(response.headers())?;
        let declared_len = response.body().content_length();
        if declared_len.is_some_and(|length| length > options.max_response_bytes) {
            return Err(MediaError::Io(invalid_response(format_args!(
                "HTTP source length exceeds the configured {}-byte limit",
                options.max_response_bytes
            ))));
        }
        let final_uri = response.get_uri().clone();
        Ok(Self {
            reader: response.into_body().into_reader(),
            final_uri,
            declared_len,
            position: 0,
            max_response_bytes: options.max_response_bytes,
            cancellation,
        })
    }

    pub(crate) fn final_uri(&self) -> &Uri {
        &self.final_uri
    }
}

impl Read for HttpStreamInput {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.cancellation.check_io()?;
        if buffer.is_empty() {
            return Ok(0);
        }
        if self.position == self.max_response_bytes {
            let mut extra = [0_u8; 1];
            let count = self
                .reader
                .read(&mut extra)
                .map_err(|error| sanitize_body_error(&error))?;
            self.cancellation.check_io()?;
            if count == 0 {
                return Ok(0);
            }
            return Err(invalid_response(format_args!(
                "HTTP stream exceeded its {}-byte limit",
                self.max_response_bytes
            )));
        }
        let remaining = self.max_response_bytes - self.position;
        let allowed = usize::try_from(remaining)
            .unwrap_or(usize::MAX)
            .min(buffer.len());
        let count = self
            .reader
            .read(&mut buffer[..allowed])
            .map_err(|error| sanitize_body_error(&error))?;
        self.cancellation.check_io()?;
        if count == 0
            && self
                .declared_len
                .is_some_and(|length| self.position < length)
        {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "HTTP stream body ended before its declared length",
            ));
        }
        self.position = self.position.saturating_add(count as u64);
        Ok(count)
    }
}

impl Seek for HttpStreamInput {
    fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "HTTP stream input is not seekable",
        ))
    }
}

impl MediaInput for HttpStreamInput {
    fn is_seekable(&self) -> bool {
        false
    }

    fn byte_len(&self) -> Option<u64> {
        self.declared_len
    }
}

impl MediaInput for HttpRangeInput {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.source_len)
    }
}

struct ActiveRange {
    reader: BodyReader<'static>,
    remaining: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Validator {
    name: HeaderName,
    value: String,
}

#[derive(Clone, Copy, Debug)]
struct ParsedRange {
    start: u64,
    end: u64,
    total: u64,
}

fn parse_uri(url: &str) -> Result<Uri, MediaError> {
    let uri: Uri = url.parse().map_err(|_| {
        MediaError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid HTTP media URL",
        ))
    })?;
    if !matches!(uri.scheme_str(), Some("http" | "https")) || uri.authority().is_none() {
        return Err(MediaError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "HTTP media URL must use http or https and include an authority",
        )));
    }
    if uri
        .authority()
        .is_some_and(|authority| authority.as_str().contains('@'))
    {
        return Err(MediaError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "HTTP media URL must not contain user information",
        )));
    }
    Ok(uri)
}

fn parse_content_range(headers: &HeaderMap) -> io::Result<ParsedRange> {
    let value = single_header(headers, &CONTENT_RANGE)?;
    let value = value
        .to_str()
        .map_err(|_| invalid_response(format_args!("Content-Range is not valid ASCII")))?;
    let value = value
        .strip_prefix("bytes ")
        .ok_or_else(|| invalid_response(format_args!("invalid Content-Range unit")))?;
    let (span, total) = value
        .split_once('/')
        .ok_or_else(|| invalid_response(format_args!("invalid Content-Range syntax")))?;
    let (start, end) = span
        .split_once('-')
        .ok_or_else(|| invalid_response(format_args!("invalid Content-Range span")))?;
    let start = parse_header_number(start, "Content-Range start")?;
    let end = parse_header_number(end, "Content-Range end")?;
    let total = parse_header_number(total, "Content-Range total")?;
    if start > end || end >= total {
        return Err(invalid_response(format_args!(
            "Content-Range numbers are inconsistent"
        )));
    }
    Ok(ParsedRange { start, end, total })
}

fn parse_single_u64_header(headers: &HeaderMap, name: &HeaderName) -> io::Result<u64> {
    let value = single_header(headers, name)?;
    let value = value
        .to_str()
        .map_err(|_| invalid_response(format_args!("HTTP numeric header is not valid ASCII")))?;
    parse_header_number(value, "HTTP numeric header")
}

fn parse_header_number(value: &str, description: &'static str) -> io::Result<u64> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid_response(format_args!("invalid {description}")));
    }
    value
        .parse()
        .map_err(|_| invalid_response(format_args!("invalid {description}")))
}

fn single_header<'a>(
    headers: &'a HeaderMap,
    name: &HeaderName,
) -> io::Result<&'a ureq::http::HeaderValue> {
    let mut values = headers.get_all(name).iter();
    let value = values
        .next()
        .ok_or_else(|| invalid_response(format_args!("missing {name}")))?;
    if values.next().is_some() {
        return Err(invalid_response(format_args!("duplicate {name}")));
    }
    Ok(value)
}

fn reject_content_encoding(headers: &HeaderMap) -> io::Result<()> {
    let mut values = headers.get_all(CONTENT_ENCODING).iter();
    let Some(value) = values.next() else {
        return Ok(());
    };
    if values.next().is_some()
        || !value
            .to_str()
            .is_ok_and(|encoding| encoding.eq_ignore_ascii_case("identity"))
    {
        return Err(invalid_response(format_args!(
            "HTTP response uses a non-identity content encoding"
        )));
    }
    Ok(())
}

fn read_validator(headers: &HeaderMap) -> io::Result<Option<Validator>> {
    if let Some(value) = optional_single_header(headers, &ETAG)? {
        let value = value
            .to_str()
            .map_err(|_| invalid_response(format_args!("ETag is not valid ASCII")))?;
        if !value.starts_with("W/") {
            return Ok(Some(Validator {
                name: ETAG,
                value: value.to_owned(),
            }));
        }
    }
    let Some(value) = optional_single_header(headers, &LAST_MODIFIED)? else {
        return Ok(None);
    };
    let value = value
        .to_str()
        .map_err(|_| invalid_response(format_args!("Last-Modified is not valid ASCII")))?;
    Ok(Some(Validator {
        name: LAST_MODIFIED,
        value: value.to_owned(),
    }))
}

fn optional_single_header<'a>(
    headers: &'a HeaderMap,
    name: &HeaderName,
) -> io::Result<Option<&'a ureq::http::HeaderValue>> {
    let mut values = headers.get_all(name).iter();
    let value = values.next();
    if values.next().is_some() {
        return Err(invalid_response(format_args!("duplicate {name}")));
    }
    Ok(value)
}

fn invalid_response(arguments: fmt::Arguments<'_>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, arguments.to_string())
}

fn validate_request_counts(max_redirects: u32, max_retries: u32) -> Result<(), MediaError> {
    if max_redirects > MAX_CONFIGURED_REDIRECTS {
        return Err(MediaError::InvalidHttpOptions(
            "max_redirects must not exceed 16",
        ));
    }
    if max_retries > MAX_CONFIGURED_RETRIES {
        return Err(MediaError::InvalidHttpOptions(
            "max_retries must not exceed 8",
        ));
    }
    Ok(())
}

pub(crate) fn create_agent(
    max_response_header_bytes: usize,
    socket_buffer_bytes: usize,
    connect_timeout: Duration,
    request_timeout: Duration,
    max_redirects: u32,
    network_access: HttpNetworkAccess,
) -> Agent {
    let config = Agent::config_builder()
        .proxy(None)
        .max_redirects(max_redirects)
        .max_redirects_will_error(true)
        .http_status_as_error(false)
        .accept_encoding("")
        .max_response_header_size(max_response_header_bytes)
        .input_buffer_size(socket_buffer_bytes)
        .output_buffer_size(socket_buffer_bytes)
        .timeout_global(Some(request_timeout))
        .timeout_connect(Some(connect_timeout))
        .timeout_recv_response(Some(request_timeout))
        .timeout_recv_body(Some(request_timeout))
        .build();
    Agent::with_parts(
        config,
        DefaultConnector::default(),
        PolicyResolver {
            access: network_access,
        },
    )
}

fn call_with_retries(
    mut call: impl FnMut() -> Result<ureq::http::Response<ureq::Body>, UreqError>,
    max_retries: u32,
    cancellation: &MediaCancellation,
) -> io::Result<ureq::http::Response<ureq::Body>> {
    let mut retries = 0;
    loop {
        cancellation.check_io()?;
        match call() {
            Ok(response) if response.status().is_server_error() && retries < max_retries => {
                retries += 1;
            }
            Ok(response) => {
                cancellation.check_io()?;
                return Ok(response);
            }
            Err(error) if is_retriable_request_error(&error) && retries < max_retries => {
                retries += 1;
            }
            Err(error) => return Err(sanitize_ureq_error(&error)),
        }
    }
}

fn is_retriable_request_error(error: &UreqError) -> bool {
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

pub(crate) fn is_blocked_destination_error(error: &UreqError) -> bool {
    matches!(error, UreqError::Other(inner) if inner.is::<BlockedDestination>())
}

fn sanitize_ureq_error(error: &UreqError) -> io::Error {
    if is_blocked_destination_error(error) {
        return io::Error::new(
            io::ErrorKind::PermissionDenied,
            "HTTP destination rejected by network access policy",
        );
    }
    let kind = match error {
        UreqError::Timeout(_) => io::ErrorKind::TimedOut,
        UreqError::HostNotFound => io::ErrorKind::NotFound,
        UreqError::Io(source) => source.kind(),
        _ => io::ErrorKind::Other,
    };
    io::Error::new(kind, "HTTP range request failed")
}

fn sanitize_body_error(error: &io::Error) -> io::Error {
    io::Error::new(error.kind(), "HTTP range body read failed")
}

#[derive(Clone, Copy, Debug)]
struct PolicyResolver {
    access: HttpNetworkAccess,
}

impl Resolver for PolicyResolver {
    fn resolve(
        &self,
        uri: &Uri,
        config: &ureq::config::Config,
        timeout: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, UreqError> {
        let resolved = DefaultResolver::default().resolve(uri, config, timeout)?;
        if self.access == HttpNetworkAccess::AllowPrivateNetworks {
            return Ok(resolved);
        }
        let mut allowed = self.empty();
        for address in resolved.iter().copied() {
            if is_public_address(address.ip()) {
                allowed.push(address);
            }
        }
        if allowed.is_empty() {
            Err(UreqError::Other(Box::new(BlockedDestination)))
        } else {
            Ok(allowed)
        }
    }
}

#[derive(Debug)]
struct BlockedDestination;

impl fmt::Display for BlockedDestination {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("HTTP destination rejected by network access policy")
    }
}

impl std::error::Error for BlockedDestination {}

fn is_public_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            let [a, b, c, d] = address.octets();
            !(a == 0
                || a == 10
                || a == 127
                || (a == 100 && (64..=127).contains(&b))
                || (a == 169 && b == 254)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192 && b == 0 && c == 0)
                || (a == 192 && b == 0 && c == 2)
                || (a == 192 && b == 88 && c == 99)
                || (a == 192 && b == 168)
                || (a == 198 && (b == 18 || b == 19))
                || (a == 198 && b == 51 && c == 100)
                || (a == 203 && b == 0 && c == 113)
                || a >= 224
                || (a == 255 && b == 255 && c == 255 && d == 255))
        }
        IpAddr::V6(address) => {
            let octets = address.octets();
            let globally_routable_prefix = octets[0] & 0xe0 == 0x20;
            let documentation = octets[..4] == [0x20, 0x01, 0x0d, 0xb8];
            let benchmarking = octets[..6] == [0x20, 0x01, 0x00, 0x02, 0x00, 0x00];
            let orchid =
                octets[..3] == [0x20, 0x01, 0x00] && matches!(octets[3] & 0xf0, 0x10 | 0x20);
            globally_routable_prefix && !documentation && !benchmarking && !orchid
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use super::*;

    #[test]
    fn public_address_policy_denies_special_ranges() {
        for address in [
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1)),
            IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1)),
            IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1)),
            IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)),
            IpAddr::V4(Ipv4Addr::new(198, 51, 100, 1)),
            IpAddr::V4(Ipv4Addr::new(224, 0, 0, 1)),
            IpAddr::V6(Ipv6Addr::LOCALHOST),
            IpAddr::V6("fc00::1".parse().unwrap()),
            IpAddr::V6("fe80::1".parse().unwrap()),
            IpAddr::V6("2001:db8::1".parse().unwrap()),
            IpAddr::V6("2001:10::1".parse().unwrap()),
            IpAddr::V6("2001:20::1".parse().unwrap()),
        ] {
            assert!(!is_public_address(address), "{address}");
        }
        assert!(is_public_address(IpAddr::V4(Ipv4Addr::new(
            93, 184, 216, 34
        ))));
        assert!(is_public_address(IpAddr::V6(
            "2606:2800:220:1:248:1893:25c8:1946".parse().unwrap()
        )));
    }

    #[test]
    fn dependency_trace_logging_is_compile_time_disabled() {
        assert_eq!(log::STATIC_MAX_LEVEL, log::LevelFilter::Debug);
    }

    #[test]
    fn content_range_parser_rejects_duplicates_and_inconsistent_numbers() {
        let mut headers = HeaderMap::new();
        headers.append(CONTENT_RANGE, "bytes 0-9/10".parse().unwrap());
        assert_eq!(parse_content_range(&headers).unwrap().total, 10);

        headers.append(CONTENT_RANGE, "bytes 0-9/10".parse().unwrap());
        assert!(parse_content_range(&headers).is_err());

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_RANGE, "bytes 9-10/10".parse().unwrap());
        assert!(parse_content_range(&headers).is_err());
    }

    #[test]
    fn request_count_configuration_has_hard_ceilings() {
        assert!(
            HttpRangeOptions {
                max_redirects: 17,
                ..HttpRangeOptions::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            HttpStreamOptions {
                max_retries: 9,
                ..HttpStreamOptions::default()
            }
            .validate()
            .is_err()
        );
    }
}
