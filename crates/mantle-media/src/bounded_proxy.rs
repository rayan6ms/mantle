//! Deadline-bounded SOCKS setup on ureq's ordinary pooled TCP transport.
//! ureq 3.4.x's scoped SOCKS helper waits for unbounded socket I/O after timeout.
//! No helper thread, replacement TLS stack, or audio-path work is introduced.
use std::io;
use std::time::{Duration, Instant};
use ureq::unversioned::transport::{
    ConnectionDetails, Connector, DefaultConnector, NextTimeout, TcpConnector, Transport,
};
use ureq::{Error, Proxy, ProxyProtocol};

#[derive(Debug)]
pub(crate) struct BoundedSocksConnector;

impl Connector<()> for BoundedSocksConnector {
    type Out = Box<dyn Transport>;
    fn connect(
        &self,
        details: &ConnectionDetails<'_>,
        _: Option<()>,
    ) -> Result<Option<Self::Out>, Error> {
        let Some(proxy) = details.config.proxy().filter(|p| {
            matches!(
                p.protocol(),
                ProxyProtocol::Socks4
                    | ProxyProtocol::Socks4A
                    | ProxyProtocol::Socks5
                    | ProxyProtocol::Socks5h
            )
        }) else {
            return DefaultConnector::default().connect(details, None);
        };
        if proxy.is_no_proxy(details.uri) {
            return TcpConnector::default()
                .connect(details, None::<()>)
                .map(|transport| transport.map(Transport::boxed));
        }
        let deadline = Deadline {
            started: Instant::now(),
            timeout: details.timeout,
        };
        let addresses = details
            .resolver
            .resolve(proxy.uri(), details.config, deadline.next()?)?;
        let proxy_details = ConnectionDetails {
            uri: proxy.uri(),
            addrs: addresses,
            config: details.config,
            request_level: details.request_level,
            resolver: details.resolver,
            now: details.now,
            timeout: deadline.next()?,
            current_time: details.current_time.clone(),
            run_connector: details.run_connector.clone(),
        };
        let mut transport = TcpConnector::default()
            .connect(&proxy_details, None::<()>)?
            .ok_or(Error::ConnectionFailed)?;
        if matches!(
            proxy.protocol(),
            ProxyProtocol::Socks4 | ProxyProtocol::Socks4A
        ) {
            handshake_v4(&mut transport, proxy, details, &deadline)?;
        } else {
            handshake(&mut transport, proxy, details, &deadline)?;
        }
        Ok(Some(transport.boxed()))
    }
}

fn handshake_v4(
    transport: &mut dyn Transport,
    proxy: &Proxy,
    details: &ConnectionDetails<'_>,
    deadline: &Deadline,
) -> Result<(), Error> {
    let port = details
        .uri
        .port_u16()
        .unwrap_or(if details.needs_tls() { 443 } else { 80 });
    let mut request = vec![4, 1];
    request.extend_from_slice(&port.to_be_bytes());
    let host = if proxy.resolve_target() {
        let target = details
            .addrs
            .iter()
            .find(|address| address.is_ipv4())
            .ok_or_else(invalid)?;
        let std::net::IpAddr::V4(ip) = target.ip() else {
            return Err(invalid());
        };
        request.extend_from_slice(&ip.octets());
        None
    } else {
        let host = details.uri.host().ok_or_else(invalid)?;
        if host.is_empty() || host.len() > 255 {
            return Err(invalid());
        }
        request.extend_from_slice(&[0, 0, 0, 1]);
        Some(host)
    };
    request.push(0); // SOCKS4 has no password authentication.
    if let Some(host) = host {
        request.extend_from_slice(host.as_bytes());
        request.push(0);
    }
    deadline.write(transport, &request)?;
    let mut reply = [0; 8];
    deadline.read(transport, &mut reply)?;
    if reply[..2] != [0, 90] {
        return Err(invalid());
    }
    Ok(())
}

struct Deadline {
    started: Instant,
    timeout: NextTimeout,
}
impl Deadline {
    fn next(&self) -> Result<NextTimeout, Error> {
        crate::http_input::check_body_cancellation()?;
        let budget = self
            .timeout
            .not_zero()
            .map_or(Duration::from_secs(10), |d| *d);
        let left = budget.saturating_sub(self.started.elapsed());
        if left.is_zero() {
            return Err(Error::Timeout(self.timeout.reason));
        }
        Ok(NextTimeout {
            after: left.into(),
            reason: self.timeout.reason,
        })
    }
    fn write(&self, transport: &mut dyn Transport, bytes: &[u8]) -> Result<(), Error> {
        let timeout = self.next()?;
        let output = transport.buffers().output();
        if output.len() < bytes.len() {
            return Err(invalid());
        }
        output[..bytes.len()].copy_from_slice(bytes);
        transport.transmit_output(bytes.len(), timeout)
    }
    fn read(&self, transport: &mut dyn Transport, mut target: &mut [u8]) -> Result<(), Error> {
        while !target.is_empty() {
            let timeout = self.next()?;
            let available = transport.buffers().input();
            if !available.is_empty() {
                let count = target.len().min(available.len());
                target[..count].copy_from_slice(&available[..count]);
                transport.buffers().input_consume(count);
                target = &mut target[count..];
                continue;
            }
            let poll = NextTimeout {
                after: timeout
                    .not_zero()
                    .map_or(Duration::from_millis(100), |d| {
                        (*d).min(Duration::from_millis(100))
                    })
                    .into(),
                reason: timeout.reason,
            };
            match transport.await_input(poll) {
                Err(Error::Timeout(_)) => {}
                Err(Error::Io(e))
                    if matches!(
                        e.kind(),
                        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                    ) => {}
                Ok(false) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "source proxy closed during handshake",
                    )
                    .into());
                }
                result => {
                    result?;
                }
            }
        }
        Ok(())
    }
}

fn invalid() -> Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid source proxy handshake").into()
}

fn handshake(
    transport: &mut dyn Transport,
    proxy: &Proxy,
    details: &ConnectionDetails<'_>,
    deadline: &Deadline,
) -> Result<(), Error> {
    let credentials = proxy.username().zip(proxy.password());
    let method = u8::from(credentials.is_some()) * 2;
    deadline.write(transport, &[5, 1, method])?;
    let mut response = [0; 2];
    deadline.read(transport, &mut response)?;
    if response != [5, method] {
        return Err(invalid());
    }
    if let Some((username, password)) = credentials {
        let user_len = u8::try_from(username.len()).map_err(|_| invalid())?;
        let password_len = u8::try_from(password.len()).map_err(|_| invalid())?;
        if user_len == 0 || password_len == 0 {
            return Err(invalid());
        }
        let mut auth = [0; 515];
        auth[0] = 1;
        auth[1] = user_len;
        auth[2..2 + username.len()].copy_from_slice(username.as_bytes());
        auth[2 + username.len()] = password_len;
        let length = 3 + username.len() + password.len();
        auth[3 + username.len()..length].copy_from_slice(password.as_bytes());
        deadline.write(transport, &auth[..length])?;
        deadline.read(transport, &mut response)?;
        if response != [1, 0] {
            return Err(invalid());
        }
    }
    let port = details
        .uri
        .port_u16()
        .unwrap_or(if details.needs_tls() { 443 } else { 80 });
    let mut request = vec![5, 1, 0];
    if proxy.resolve_target() {
        let target = details.addrs.first().ok_or(Error::ConnectionFailed)?;
        match target.ip() {
            std::net::IpAddr::V4(ip) => {
                request.push(1);
                request.extend_from_slice(&ip.octets());
            }
            std::net::IpAddr::V6(ip) => {
                request.push(4);
                request.extend_from_slice(&ip.octets());
            }
        }
    } else {
        let host = details.uri.host().ok_or_else(invalid)?;
        let length = u8::try_from(host.len()).map_err(|_| invalid())?;
        if length == 0 {
            return Err(invalid());
        }
        request.extend_from_slice(&[3, length]);
        request.extend_from_slice(host.as_bytes());
    }
    request.extend_from_slice(&port.to_be_bytes());
    deadline.write(transport, &request)?;
    let mut reply = [0; 4];
    deadline.read(transport, &mut reply)?;
    if reply[..3] != [5, 0, 0] {
        return Err(invalid());
    }
    let address_bytes = match reply[3] {
        1 => 4,
        4 => 16,
        3 => {
            let mut length = [0];
            deadline.read(transport, &mut length)?;
            usize::from(length[0])
        }
        _ => return Err(invalid()),
    };
    let mut bound_address = [0; 257];
    deadline.read(transport, &mut bound_address[..address_bytes + 2])
}
