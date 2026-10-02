use std::fmt;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::sync::Arc;
use std::time::{Duration, Instant};

use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use ureq::http::header::{
    ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_RANGE, ETAG, IF_RANGE,
    LAST_MODIFIED, RANGE,
};
use ureq::http::{HeaderMap, HeaderName, Uri};
use ureq::unversioned::resolver::{DefaultResolver, ResolvedSocketAddrs, Resolver};
use ureq::unversioned::transport::{
    Buffers, Connector, DefaultConnector, Either, LazyBuffers, NextTimeout, RustlsConnector,
    Transport,
};
use ureq::{Agent, BodyReader, Error as UreqError, Proxy, ResponseExt};

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

/// One selected outbound source address and opaque connection-pool identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutboundRoute {
    pub local_ip: IpAddr,
    pub identity: u64,
}

/// Credential-safe destination context passed to an outbound route policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutboundRouteContext<'a> {
    pub scheme: &'a str,
    pub authority: &'a str,
}

/// Stable outcome classes reported after a routed request or connection attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutboundRouteOutcome {
    ConnectionEstablished,
    DestinationDenied,
    Timeout,
    TransportFailure,
}

/// Injectable outbound address selection used by RoutePlanner-style integrations.
pub trait OutboundRoutePolicy: fmt::Debug + Send + Sync {
    fn select_route(&self, context: OutboundRouteContext<'_>) -> Option<OutboundRoute>;

    fn report_outcome(&self, route: OutboundRoute, outcome: OutboundRouteOutcome);
}

/// Resource and network policy for a seekable HTTP range input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HttpRangeOptions {
    /// Stage finite objects no larger than this many bytes in an anonymous
    /// temporary file before returning from open. Zero disables staging;
    /// larger objects retain ordinary range streaming. Maximum 64 MiB.
    /// Available on Unix; nonzero values are rejected on other platforms.
    pub staging_max_bytes: u64,
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
            staging_max_bytes: 0,
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
        if self.staging_max_bytes != 0 && !cfg!(unix) {
            return Err(MediaError::InvalidHttpOptions(
                "source staging requires Unix",
            ));
        }
        if self.staging_max_bytes > 64 * 1024 * 1024 {
            return Err(MediaError::InvalidHttpOptions(
                "staging_max_bytes must not exceed 64 MiB",
            ));
        }
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
    body_retries: u32,
    recovery: Option<(Instant, u64)>,
    staged: Option<File>,
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
        Self::open_inner(url, options, cancellation, None)
    }

    /// Opens a ranged object with a selected local-address policy on every new connection.
    ///
    /// # Errors
    ///
    /// Returns the bounded option, cancellation, URL, destination, transport, or range-response
    /// errors documented by [`Self::open_with_cancellation`].
    pub fn open_routed_with_cancellation(
        url: impl AsRef<str>,
        options: HttpRangeOptions,
        cancellation: MediaCancellation,
        route_policy: Arc<dyn OutboundRoutePolicy>,
    ) -> Result<Self, MediaError> {
        Self::open_inner(url, options, cancellation, Some(route_policy))
    }

    fn open_inner(
        url: impl AsRef<str>,
        options: HttpRangeOptions,
        cancellation: MediaCancellation,
        route_policy: Option<Arc<dyn OutboundRoutePolicy>>,
    ) -> Result<Self, MediaError> {
        let options = options.validate()?;
        cancellation.check()?;
        let uri = parse_uri(url.as_ref())?;
        let agent = create_agent_with_route_policy(
            options.max_response_header_bytes,
            options.socket_buffer_bytes,
            options.connect_timeout,
            options.request_timeout,
            options.max_redirects,
            options.network_access,
            route_policy,
        );
        let mut input = Self {
            agent,
            uri,
            options,
            position: 0,
            source_len: 0,
            active: None,
            body_retries: 0,
            recovery: None,
            staged: None,
            validator: None,
            cancellation,
        };
        input.open_range()?;
        if input.source_len <= input.options.staging_max_bytes {
            input.stage()?;
        }
        Ok(input)
    }

    // Duplicate only the handle, not the compressed bytes. The consumer must
    // drop the active reader before seeking the shared file cursor for replay.
    pub(crate) fn clone_staged_file(&self) -> Result<Option<File>, MediaError> {
        self.staged
            .as_ref()
            .map(File::try_clone)
            .transpose()
            .map_err(MediaError::Io)
    }

    fn stage(&mut self) -> io::Result<()> {
        // The file is anonymous and closes on every error/cancellation path.
        // Keep compressed bytes off the heap and reuse one bounded copy buffer.
        let mut file = staging_file()?;
        let mut buffer = vec![0_u8; 64 * 1024];
        let started = Instant::now();
        loop {
            if started.elapsed() >= self.options.request_timeout {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "HTTP staging deadline exceeded",
                ));
            }
            let count = self.read(&mut buffer)?;
            if started.elapsed() >= self.options.request_timeout {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "HTTP staging deadline exceeded",
                ));
            }
            if count == 0 {
                break;
            }
            file.write_all(&buffer[..count])?;
        }
        self.cancellation.check_io()?;
        file.seek(SeekFrom::Start(0))?;
        self.position = 0;
        self.staged = Some(file);
        Ok(())
    }

    #[must_use]
    pub fn final_uri(&self) -> &Uri {
        &self.uri
    }

    #[allow(clippy::too_many_lines)]
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
            .min(self.options.max_source_bytes.saturating_sub(1))
            .min(
                self.recovery
                    .map_or(u64::MAX, |(_, end)| end.saturating_sub(1)),
            );
        let range_value = format!("bytes={}-{}", self.position, requested_end);
        let agent = self.agent.clone();
        let uri = self.uri.clone();
        let validator = self.validator.clone();
        let request_timeout = self
            .recovery
            .map_or(self.options.request_timeout, |(started, _)| {
                self.options
                    .request_timeout
                    .saturating_sub(started.elapsed())
            });
        if request_timeout.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "HTTP body recovery deadline exceeded",
            ));
        }
        let response = call_with_retries(
            || {
                let mut request = agent
                    .get(uri.clone())
                    .header(RANGE, range_value.as_str())
                    .header(ACCEPT_ENCODING, "identity");
                if let Some(validator) = &validator {
                    request = request.header(IF_RANGE, validator.value.as_str());
                }
                request
                    .config()
                    .timeout_global(Some(request_timeout))
                    .build()
                    .call()
            },
            if self.recovery.is_some() {
                0
            } else {
                self.options.max_retries
            },
            &self.cancellation,
        )?;
        self.uri = response.get_uri().clone();
        let status = response.status().as_u16();
        if status != 206 {
            log::debug!("HTTP media range rejected with status {status}");
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

#[cfg(unix)]
fn staging_file() -> io::Result<File> {
    use std::fs::OpenOptions;
    use std::os::unix::fs::OpenOptionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::SystemTime;
    static NEXT_FILE: AtomicU64 = AtomicU64::new(0);
    let stamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    for _ in 0..32 {
        let serial = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "mantle-{}-{stamp:x}-{serial:x}.tmp",
            std::process::id()
        ));
        // create_new atomically rejects every existing path, including symlinks.
        // The name is not a security boundary: no existing file can be opened,
        // contents start private, and unlink occurs before source bytes arrive.
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
        {
            Ok(file) => {
                std::fs::remove_file(path)?;
                return Ok(file);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "temporary staging namespace exhausted",
    ))
}

#[cfg(not(unix))]
fn staging_file() -> io::Result<File> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "source staging requires Unix",
    ))
}

impl Read for HttpRangeInput {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.cancellation.check_io()?;
        if buffer.is_empty() || self.position >= self.source_len {
            return Ok(0);
        }
        if let Some(file) = &mut self.staged {
            let count = file.read(buffer)?;
            self.position += count as u64;
            return Ok(count);
        }
        loop {
            self.cancellation.check_io()?;
            if self.active.is_none() {
                self.open_range()?;
            }
            let Some(active) = self.active.as_mut() else {
                return Ok(0);
            };
            let end = self.position.saturating_add(active.remaining);
            let allowed = usize::try_from(active.remaining)
                .unwrap_or(usize::MAX)
                .min(buffer.len());
            let read = active.reader.read(&mut buffer[..allowed]);
            self.cancellation.check_io()?;
            let error = match read {
                Ok(count) if count != 0 => {
                    self.position = self.position.saturating_add(count as u64);
                    active.remaining = active.remaining.saturating_sub(count as u64);
                    if active.remaining == 0 {
                        self.active = None;
                    }
                    if self.recovery.is_some_and(|(_, end)| self.position >= end) {
                        self.recovery = None;
                    }
                    return Ok(count);
                }
                Ok(_) => io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "HTTP range body ended before its declared length",
                ),
                Err(error) => sanitize_body_error(&error),
            };
            let retriable = matches!(
                error.kind(),
                io::ErrorKind::ConnectionAborted
                    | io::ErrorKind::ConnectionReset
                    | io::ErrorKind::TimedOut
                    | io::ErrorKind::UnexpectedEof
                    | io::ErrorKind::Interrupted
            );
            // A strong entity tag is required before splicing a reopened response
            // into a decoder. Last-Modified alone can miss same-second changes.
            if !retriable
                || self.body_retries >= self.options.max_retries
                || !self.validator.as_ref().is_some_and(|v| v.name == ETAG)
            {
                return Err(error);
            }
            self.body_retries += 1;
            self.recovery.get_or_insert((Instant::now(), end));
            self.active = None;
            // open_range validates identity, total length and the exact consumed offset.
        }
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
            if let Some(file) = &mut self.staged {
                file.seek(SeekFrom::Start(target))?;
            }
            self.position = target;
            self.active = None;
            self.recovery = None;
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
        Self::open_inner(url, options, cancellation, None)
    }

    /// Opens a bounded stream whose new connection selects and binds an outbound route.
    ///
    /// # Errors
    ///
    /// Returns the bounded option, cancellation, URL, destination, transport, status, or body
    /// errors documented by [`Self::open_with_cancellation`].
    pub fn open_routed_with_cancellation(
        url: impl AsRef<str>,
        options: HttpStreamOptions,
        cancellation: MediaCancellation,
        route_policy: Arc<dyn OutboundRoutePolicy>,
    ) -> Result<Self, MediaError> {
        Self::open_inner(url, options, cancellation, Some(route_policy))
    }

    fn open_inner(
        url: impl AsRef<str>,
        options: HttpStreamOptions,
        cancellation: MediaCancellation,
        route_policy: Option<Arc<dyn OutboundRoutePolicy>>,
    ) -> Result<Self, MediaError> {
        let options = options.validate()?;
        cancellation.check()?;
        let uri = parse_uri(url.as_ref())?;
        let agent = create_agent_with_route_policy(
            options.max_response_header_bytes,
            options.socket_buffer_bytes,
            options.connect_timeout,
            options.request_timeout,
            options.max_redirects,
            options.network_access,
            route_policy,
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

pub(crate) fn create_agent_with_route_policy(
    max_response_header_bytes: usize,
    socket_buffer_bytes: usize,
    connect_timeout: Duration,
    request_timeout: Duration,
    max_redirects: u32,
    network_access: HttpNetworkAccess,
    route_policy: Option<Arc<dyn OutboundRoutePolicy>>,
) -> Agent {
    create_agent_with_route_policy_and_proxy(
        max_response_header_bytes,
        socket_buffer_bytes,
        connect_timeout,
        request_timeout,
        max_redirects,
        network_access,
        route_policy,
        configured_proxy(),
    )
}

/// Creates a source HTTP agent without the process-wide YouTube proxy.
///
/// Companion is an explicitly trusted loopback sidecar. It must be reached directly while
/// ordinary YouTube control and media requests continue to use `RAYDIO_YOUTUBE_PROXY`.
pub(crate) fn create_direct_agent_with_route_policy(
    max_response_header_bytes: usize,
    socket_buffer_bytes: usize,
    connect_timeout: Duration,
    request_timeout: Duration,
    max_redirects: u32,
    network_access: HttpNetworkAccess,
    route_policy: Option<Arc<dyn OutboundRoutePolicy>>,
) -> Agent {
    create_agent_with_route_policy_and_proxy(
        max_response_header_bytes,
        socket_buffer_bytes,
        connect_timeout,
        request_timeout,
        max_redirects,
        network_access,
        route_policy,
        None,
    )
}

fn create_agent_with_route_policy_and_proxy(
    max_response_header_bytes: usize,
    socket_buffer_bytes: usize,
    connect_timeout: Duration,
    request_timeout: Duration,
    max_redirects: u32,
    network_access: HttpNetworkAccess,
    route_policy: Option<Arc<dyn OutboundRoutePolicy>>,
    proxy: Option<Proxy>,
) -> Agent {
    // A configured source proxy is an explicitly trusted local sidecar in deployments such as
    // the home-egress tunnel. Resolve and connect to that proxy even when the normal source
    // policy rejects loopback/private destinations; the policy still filters any destination
    // URI that is resolved directly by the agent.
    let proxy_authority = proxy.as_ref().and_then(|proxy| {
        proxy
            .uri()
            .authority()
            .map(|authority| authority.as_str().to_owned())
    });
    let config = Agent::config_builder()
        .proxy(proxy)
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
        .max_idle_connections(if route_policy.is_some() { 0 } else { 10 })
        .build();
    let resolver = PolicyResolver {
        access: network_access,
        proxy_authority,
    };
    if let Some(policy) = route_policy {
        let connector = ().chain(RoutedTcpConnector { policy }).chain(RustlsConnector::default());
        Agent::with_parts(config, connector, resolver)
    } else {
        Agent::with_parts(config, DefaultConnector::default(), resolver)
    }
}

/// Returns the optional YouTube/source proxy without ever logging its URL or credentials.
///
/// The process sets this only for source traffic (`RAYDIO_YOUTUBE_PROXY`). Discord gateway and
/// voice traffic use separate clients and are unaffected. HTTP CONNECT, HTTPS CONNECT, and
/// SOCKS4/4A/5 URLs are supported by ureq; SOCKS support is compiled in explicitly above.
fn configured_proxy() -> Option<Proxy> {
    std::env::var("RAYDIO_YOUTUBE_PROXY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .and_then(|value| Proxy::new(value.trim()).ok())
}

#[derive(Debug)]
struct RoutedTcpConnector {
    policy: Arc<dyn OutboundRoutePolicy>,
}

impl<In: Transport> Connector<In> for RoutedTcpConnector {
    type Out = Either<In, RoutedTcpTransport>;

    fn connect(
        &self,
        details: &ureq::unversioned::transport::ConnectionDetails<'_>,
        chained: Option<In>,
    ) -> Result<Option<Self::Out>, UreqError> {
        if chained.is_some() {
            return Ok(chained.map(Either::A));
        }
        let scheme = details.uri.scheme_str().unwrap_or_default();
        let authority = details
            .uri
            .authority()
            .map_or("", ureq::http::uri::Authority::as_str);
        let route = self
            .policy
            .select_route(OutboundRouteContext { scheme, authority });
        let stream = connect_routed(
            &details.addrs,
            route,
            details.timeout,
            details.config.no_delay(),
        )
        .map_err(|error| {
            if let Some(route) = route {
                self.policy.report_outcome(
                    route,
                    if error.kind() == io::ErrorKind::TimedOut {
                        OutboundRouteOutcome::Timeout
                    } else {
                        OutboundRouteOutcome::TransportFailure
                    },
                );
            }
            UreqError::Io(error)
        })?;
        if let Some(route) = route {
            self.policy
                .report_outcome(route, OutboundRouteOutcome::ConnectionEstablished);
        }
        let buffers = LazyBuffers::new(
            details.config.input_buffer_size(),
            details.config.output_buffer_size(),
        );
        Ok(Some(Either::B(RoutedTcpTransport {
            stream,
            buffers,
            read_timeout: None,
            write_timeout: None,
        })))
    }
}

struct RoutedTcpTransport {
    stream: TcpStream,
    buffers: LazyBuffers,
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
}

impl fmt::Debug for RoutedTcpTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RoutedTcpTransport")
            .field("peer", &self.stream.peer_addr().ok())
            .finish_non_exhaustive()
    }
}

impl Transport for RoutedTcpTransport {
    fn buffers(&mut self) -> &mut dyn Buffers {
        &mut self.buffers
    }

    fn transmit_output(&mut self, amount: usize, timeout: NextTimeout) -> Result<(), UreqError> {
        let next = timeout.not_zero().map(|duration| *duration);
        if next != self.write_timeout {
            self.stream.set_write_timeout(next)?;
            self.write_timeout = next;
        }
        let output = &self.buffers.output()[..amount];
        self.stream.write_all(output).map_err(|error| {
            if error.kind() == io::ErrorKind::TimedOut {
                UreqError::Timeout(timeout.reason)
            } else {
                UreqError::Io(error)
            }
        })
    }

    fn await_input(&mut self, timeout: NextTimeout) -> Result<bool, UreqError> {
        let next = timeout.not_zero().map(|duration| *duration);
        if next != self.read_timeout {
            self.stream.set_read_timeout(next)?;
            self.read_timeout = next;
        }
        let input = self.buffers.input_append_buf();
        let amount = self.stream.read(input).map_err(|error| {
            if error.kind() == io::ErrorKind::TimedOut {
                UreqError::Timeout(timeout.reason)
            } else {
                UreqError::Io(error)
            }
        })?;
        self.buffers.input_appended(amount);
        Ok(amount > 0)
    }

    fn is_open(&mut self) -> bool {
        false
    }
}

fn connect_routed(
    destinations: &ResolvedSocketAddrs,
    route: Option<OutboundRoute>,
    timeout: NextTimeout,
    no_delay: bool,
) -> io::Result<TcpStream> {
    let mut last_error = None;
    let started = std::time::Instant::now();
    let total_timeout = timeout.not_zero().map(|duration| *duration);
    for destination in destinations {
        let domain = Domain::for_address(*destination);
        let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;
        if let Some(route) = route {
            if route.local_ip.is_ipv4() != destination.is_ipv4() {
                last_error = Some(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "outbound route address family does not match destination",
                ));
                continue;
            }
            socket.bind(&SockAddr::from(SocketAddr::new(route.local_ip, 0)))?;
        }
        let address = SockAddr::from(*destination);
        let result = if let Some(total_timeout) = total_timeout {
            let remaining = total_timeout.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "routed connection timed out",
                ));
            }
            socket.connect_timeout(&address, remaining)
        } else {
            socket.connect(&address)
        };
        match result {
            Ok(()) => {
                let stream: TcpStream = socket.into();
                stream.set_nodelay(no_delay)?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            "no routed destination address",
        )
    }))
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

#[derive(Clone, Debug)]
struct PolicyResolver {
    access: HttpNetworkAccess,
    proxy_authority: Option<String>,
}

impl Resolver for PolicyResolver {
    fn resolve(
        &self,
        uri: &Uri,
        config: &ureq::config::Config,
        timeout: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, UreqError> {
        let resolved = DefaultResolver::default().resolve(uri, config, timeout)?;
        let is_configured_proxy = self.is_configured_proxy(uri);
        if self.access == HttpNetworkAccess::AllowPrivateNetworks || is_configured_proxy {
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

impl PolicyResolver {
    fn is_configured_proxy(&self, uri: &Uri) -> bool {
        self.proxy_authority.as_deref().is_some_and(|authority| {
            uri.authority()
                .is_some_and(|candidate| candidate.as_str() == authority)
        })
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
    fn source_proxy_authority_is_the_only_private_exception() {
        let resolver = PolicyResolver {
            access: HttpNetworkAccess::PublicInternetOnly,
            proxy_authority: Some("127.0.0.1:18080".to_owned()),
        };
        assert!(resolver.is_configured_proxy(&Uri::from_static(
            "socks5://127.0.0.1:18080",
        )));
        assert!(!resolver.is_configured_proxy(&Uri::from_static(
            "https://127.0.0.1:18081",
        )));
        assert!(!resolver.is_configured_proxy(&Uri::from_static(
            "https://youtube.com",
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
    #[cfg(unix)]
    fn staging_storage_is_private_and_anonymous() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let file = super::staging_file().unwrap();
        let metadata = file.metadata().unwrap();
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        assert_eq!(
            metadata.nlink(),
            0,
            "staging must have no persistent pathname"
        );
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
