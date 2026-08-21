use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct ReplayRequest {
    pub target: String,
    pub headers: Vec<(String, String)>,
    #[allow(dead_code)]
    pub body: Vec<u8>,
}

impl ReplayRequest {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find_map(|(candidate, value)| {
            candidate
                .eq_ignore_ascii_case(name)
                .then_some(value.as_str())
        })
    }
}

pub struct ReplayResponse {
    status: u16,
    body: Vec<u8>,
}

impl ReplayResponse {
    pub fn json(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: 200,
            body: body.into(),
        }
    }

    pub fn status(status: u16) -> Self {
        Self {
            status,
            body: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn json_status(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            body: body.into(),
        }
    }
}

pub struct ReplayServer {
    address: SocketAddr,
    requests: Arc<Mutex<Vec<ReplayRequest>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl ReplayServer {
    pub fn start(
        responder: impl Fn(ReplayRequest, usize) -> ReplayResponse + Send + Sync + 'static,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind replay server");
        listener
            .set_nonblocking(true)
            .expect("configure replay server");
        let address = listener.local_addr().expect("replay address");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let thread_requests = Arc::clone(&requests);
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let responder = Arc::new(responder);
        let thread = thread::spawn(move || {
            while !thread_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => serve(stream, &thread_requests, responder.as_ref()),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(1));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            address,
            requests,
            stop,
            thread: Some(thread),
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://{}/{path}", self.address)
    }

    pub fn requests(&self) -> Vec<ReplayRequest> {
        self.requests.lock().expect("replay requests").clone()
    }
}

impl Drop for ReplayServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = TcpStream::connect(self.address);
        if let Some(thread) = self.thread.take() {
            thread.join().expect("join replay server");
        }
    }
}

fn serve(
    mut stream: TcpStream,
    requests: &Mutex<Vec<ReplayRequest>>,
    responder: &(dyn Fn(ReplayRequest, usize) -> ReplayResponse + Send + Sync),
) {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set replay timeout");
    let mut raw = Vec::with_capacity(4096);
    let mut buffer = [0_u8; 1024];
    let header_end = loop {
        let Ok(count) = stream.read(&mut buffer) else {
            return;
        };
        if count == 0 {
            return;
        }
        raw.extend_from_slice(&buffer[..count]);
        if let Some(position) = raw.windows(4).position(|window| window == b"\r\n\r\n") {
            break position + 4;
        }
        if raw.len() > 16 * 1024 {
            return;
        }
    };
    let header_text = String::from_utf8_lossy(&raw[..header_end]);
    let target = header_text
        .lines()
        .next()
        .and_then(|line| line.split_ascii_whitespace().nth(1))
        .unwrap_or_default()
        .to_owned();
    let headers: Vec<_> = header_text
        .lines()
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    let content_length = headers
        .iter()
        .find_map(|(name, value)| {
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    while raw.len().saturating_sub(header_end) < content_length {
        let Ok(count) = stream.read(&mut buffer) else {
            return;
        };
        if count == 0 {
            return;
        }
        raw.extend_from_slice(&buffer[..count]);
    }
    let request = ReplayRequest {
        target,
        headers,
        body: raw[header_end..header_end + content_length].to_vec(),
    };
    let count = requests.lock().expect("replay request count").len();
    requests
        .lock()
        .expect("record replay request")
        .push(request.clone());
    let response = responder(request, count);
    let reason = if matches!(response.status, 200 | 201) {
        "OK"
    } else {
        "Error"
    };
    let _ = write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.status,
        reason,
        response.body.len()
    );
    let _ = stream.write_all(&response.body);
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Both);
}
