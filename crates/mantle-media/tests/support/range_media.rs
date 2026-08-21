use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct RangeMediaServer {
    address: SocketAddr,
    requests: Arc<Mutex<Vec<(u64, u64)>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl RangeMediaServer {
    pub fn start(bytes: Vec<u8>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind range server");
        listener
            .set_nonblocking(true)
            .expect("configure range server");
        let address = listener.local_addr().expect("range server address");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let thread_requests = Arc::clone(&requests);
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let bytes: Arc<[u8]> = bytes.into();
        let thread = thread::spawn(move || {
            while !thread_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => serve_media_range(stream, &bytes, &thread_requests),
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

    pub fn authority(&self) -> String {
        self.address.to_string()
    }

    pub fn requests(&self) -> Vec<(u64, u64)> {
        self.requests.lock().expect("range requests").clone()
    }
}

impl Drop for RangeMediaServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = TcpStream::connect(self.address);
        if let Some(thread) = self.thread.take() {
            thread.join().expect("join range server");
        }
    }
}

fn serve_media_range(mut stream: TcpStream, bytes: &[u8], requests: &Mutex<Vec<(u64, u64)>>) {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set range request timeout");
    let mut request = [0_u8; 16 * 1024];
    let mut used = 0_usize;
    while used < request.len() {
        let Ok(count) = stream.read(&mut request[used..]) else {
            return;
        };
        if count == 0 {
            return;
        }
        used += count;
        if request[..used]
            .windows(4)
            .any(|window| window == b"\r\n\r\n")
        {
            break;
        }
    }
    let request = String::from_utf8_lossy(&request[..used]);
    let Some((start, requested_end)) = request
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find_map(|(name, value)| name.eq_ignore_ascii_case("range").then_some(value.trim()))
        .and_then(|value| value.strip_prefix("bytes="))
        .and_then(|value| value.split_once('-'))
        .and_then(|(start, end)| Some((start.parse().ok()?, end.parse().ok()?)))
    else {
        return;
    };
    requests
        .lock()
        .expect("record range request")
        .push((start, requested_end));
    if start >= bytes.len() as u64 {
        return;
    }
    let end = requested_end.min(bytes.len() as u64 - 1);
    let (Ok(start_index), Ok(end_index)) = (usize::try_from(start), usize::try_from(end)) else {
        return;
    };
    let body = &bytes[start_index..=end_index];
    let _ = write!(
        stream,
        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {start}-{end}/{}\r\nConnection: close\r\n\r\n",
        body.len(),
        bytes.len()
    );
    let _ = stream.write_all(body);
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Both);
}
