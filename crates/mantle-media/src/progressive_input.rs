//! A size-bounded compressed cache with one owned, cancellable downloader.
//!
//! Readers have independent offsets and never read holes in the growing file.
//! Only the downloader writes. Release/acquire publication follows each write.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::FileExt;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::http_input::{HttpRangeInput, staging_file};
use crate::{MediaCancellation, MediaInput};

const MAX_DOWNLOADERS: usize = 8;
const COPY_BYTES: usize = 64 * 1024;
const CANCEL_POLL: Duration = Duration::from_millis(50);
static DOWNLOADERS: AtomicUsize = AtomicUsize::new(0);

struct Slot;
impl Slot {
    fn acquire() -> io::Result<Self> {
        DOWNLOADERS
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < MAX_DOWNLOADERS).then_some(n + 1)
            })
            .map(|_| Self)
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "progressive downloader capacity reached",
                )
            })
    }
}
impl Drop for Slot {
    fn drop(&mut self) {
        DOWNLOADERS.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Default)]
struct Completion {
    done: bool,
    error: Option<io::ErrorKind>,
}
struct Shared {
    file: File,
    len: u64,
    available: AtomicU64,
    completion: Mutex<Completion>,
    changed: Condvar,
}
struct Owner {
    shared: Arc<Shared>,
    stop: MediaCancellation,
    worker: Option<JoinHandle<()>>,
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.stop.cancel();
        self.shared.changed.notify_all();
        if let Some(worker) = self.worker.take() {
            // Never detach a downloader or leave temporary storage behind.
            // Body waits are interruptible; reconnects retain HTTP's bounded
            // connect/request deadline.
            let _ = worker.join();
        }
    }
}

pub(crate) struct ProgressiveInput {
    owner: Arc<Owner>,
    position: u64,
    cancellation: MediaCancellation,
}

impl ProgressiveInput {
    pub(crate) fn start(mut input: HttpRangeInput, prefix: u64) -> io::Result<Self> {
        let slot = Slot::acquire()?;
        let len = input.byte_len().expect("validated finite input");
        let shared = Arc::new(Shared {
            file: staging_file()?,
            len,
            available: AtomicU64::new(0),
            completion: Mutex::new(Completion::default()),
            changed: Condvar::new(),
        });
        let mut writer = shared.file.try_clone()?;
        let parent = input.cancellation.clone();
        let consumer_cancel = parent.clone();
        let stop = MediaCancellation::linked(move || parent.is_cancelled());
        input.cancellation = stop.clone();
        let worker_stop = stop.clone();
        let work = shared.clone();
        let deadline = *input
            .progressive_deadline
            .get_or_insert_with(|| Instant::now() + input.options.progressive_download_timeout);
        let idle_timeout = input.options.request_timeout;
        let worker = std::thread::Builder::new()
            .name("mantle-http-cache".into()).stack_size(256 * 1024)
            .spawn(move || {
                let _slot = slot;
                let started = Instant::now();
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    crate::http_input::with_body_cancellation(worker_stop.clone(), idle_timeout, || {
                        let mut buffer = vec![0; COPY_BYTES];
                        loop {
                            worker_stop.check_io()?;
                            if Instant::now() >= deadline {
                                return Err(io::Error::new(io::ErrorKind::TimedOut, "progressive download deadline exceeded"));
                            }
                            let count = input.read(&mut buffer)?;
                            if Instant::now() >= deadline {
                                return Err(io::Error::new(io::ErrorKind::TimedOut, "progressive download deadline exceeded"));
                            }
                            if count == 0 { break; }
                            writer.write_all(&buffer[..count])?;
                            // Synchronize the condition and notification to avoid
                            // a lost wake between a reader's recheck and wait.
                            let _completion = work.completion.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                            work.available.fetch_add(count as u64, Ordering::Release);
                            work.changed.notify_all();
                        }
                        if work.available.load(Ordering::Acquire) != len {
                            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "progressive source incomplete"));
                        }
                        Ok::<_, io::Error>(())
                    })
                }));
                let error = match outcome {
                    Ok(Ok(())) => None,
                    Ok(Err(error)) => Some(error.kind()),
                    Err(_) => Some(io::ErrorKind::Other),
                };
                let mut completion = work.completion.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                completion.done = true;
                completion.error = error;
                work.changed.notify_all();
                log::info!(target: "mantle_media::startup",
                    "progressive cache finished: source_bytes={} available_bytes={} elapsed_ms={:.3} failure={:?}",
                    len, work.available.load(Ordering::Acquire), started.elapsed().as_secs_f64()*1000.0, error);
            })?;
        let result = Self {
            owner: Arc::new(Owner {
                shared,
                stop,
                worker: Some(worker),
            }),
            position: 0,
            cancellation: consumer_cancel,
        };
        result.wait_for(prefix.min(len))?;
        Ok(result)
    }

    pub(crate) fn try_clone(&self) -> Self {
        Self {
            owner: self.owner.clone(),
            position: self.position,
            cancellation: self.cancellation.clone(),
        }
    }

    pub(crate) fn set_cancellation(&mut self, cancellation: MediaCancellation) {
        self.cancellation = cancellation;
    }

    fn wait_for(&self, needed: u64) -> io::Result<()> {
        let shared = &self.owner.shared;
        self.cancellation.check_io()?;
        if shared.available.load(Ordering::Acquire) >= needed {
            return Ok(());
        }
        let started = Instant::now();
        let mut completion = shared
            .completion
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            self.cancellation.check_io()?;
            if shared.available.load(Ordering::Acquire) >= needed {
                break;
            }
            if let Some(kind) = completion.error {
                return Err(io::Error::new(kind, "progressive source download failed"));
            }
            if completion.done {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "progressive source incomplete",
                ));
            }
            completion = shared
                .changed
                .wait_timeout(completion, CANCEL_POLL)
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .0;
        }
        if started.elapsed() >= Duration::from_millis(20) {
            log::info!(target: "mantle_media::startup",
                "progressive cache wait: position_bytes={} needed_bytes={} wait_ms={:.3}",
                self.position, needed, started.elapsed().as_secs_f64()*1000.0);
        }
        Ok(())
    }
}

impl Read for ProgressiveInput {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.cancellation.check_io()?;
        if buffer.is_empty() || self.position == self.owner.shared.len {
            return Ok(0);
        }
        self.wait_for(self.position + 1)?;
        let available = self.owner.shared.available.load(Ordering::Acquire);
        let count = usize::try_from(available - self.position)
            .unwrap_or(usize::MAX)
            .min(buffer.len());
        let count = self
            .owner
            .shared
            .file
            .read_at(&mut buffer[..count], self.position)?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "progressive cache ended before published bytes",
            ));
        }
        self.position += count as u64;
        Ok(count)
    }
}
impl Seek for ProgressiveInput {
    fn seek(&mut self, target: SeekFrom) -> io::Result<u64> {
        self.cancellation.check_io()?;
        let target = match target {
            SeekFrom::Start(n) => i128::from(n),
            SeekFrom::Current(n) => i128::from(self.position) + i128::from(n),
            SeekFrom::End(n) => i128::from(self.owner.shared.len) + i128::from(n),
        };
        if !(0..=i128::from(self.owner.shared.len)).contains(&target) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "progressive seek outside source",
            ));
        }
        self.position = u64::try_from(target).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "progressive seek outside source",
            )
        })?;
        // A seek never publishes holes or blocks; the next read waits for its
        // requested byte. Cached seeks and replay never reopen the origin.
        Ok(self.position)
    }
}
impl MediaInput for ProgressiveInput {
    fn is_seekable(&self) -> bool {
        true
    }
    fn byte_len(&self) -> Option<u64> {
        Some(self.owner.shared.len)
    }
    fn buffered_prefix_bytes(&self) -> Option<u64> {
        Some(self.owner.shared.available.load(Ordering::Acquire))
    }
    fn clone_buffered_input(&self) -> Option<Box<dyn MediaInput>> {
        Some(Box::new(self.try_clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HttpNetworkAccess, HttpRangeOptions};
    use std::net::TcpListener;
    use std::sync::mpsc;

    const PREFIX: usize = 16 * 1024;
    const LENGTH: usize = 128 * 1024;

    struct Origin {
        url: String,
        release: Option<mpsc::Sender<bool>>,
        worker: Option<JoinHandle<()>>,
    }
    impl Origin {
        fn new() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!("http://{}/source", listener.local_addr().unwrap());
            let (release, gate) = mpsc::channel();
            let worker = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                socket
                    .set_write_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") && request.len() < 16384 {
                    let mut byte = [0];
                    if socket.read(&mut byte).unwrap_or(0) != 1 {
                        return;
                    }
                    request.push(byte[0]);
                }
                let end = String::from_utf8(request)
                    .unwrap()
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix("range: bytes=0-")
                            .or_else(|| line.strip_prefix("Range: bytes=0-"))
                    })
                    .unwrap()
                    .parse::<usize>()
                    .unwrap()
                    .min(LENGTH - 1);
                write!(socket, "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes 0-{end}/{LENGTH}\r\nConnection: close\r\n\r\n", end+1).unwrap();
                socket.write_all(&bytes(0, PREFIX)).unwrap();
                if gate.recv_timeout(Duration::from_secs(3)).unwrap_or(false) {
                    let _ = socket.write_all(&bytes(PREFIX, end + 1 - PREFIX));
                }
            });
            Self {
                url,
                release: Some(release),
                worker: Some(worker),
            }
        }
        fn release(&mut self, complete: bool) {
            let _ = self.release.take().unwrap().send(complete);
        }
        fn open(&self, cancellation: MediaCancellation) -> HttpRangeInput {
            HttpRangeInput::open_with_cancellation(
                &self.url,
                HttpRangeOptions {
                    staging_max_bytes: LENGTH as u64,
                    progressive_buffer_bytes: PREFIX as u64,
                    expected_source_bytes: Some(LENGTH as u64),
                    request_timeout: Duration::from_secs(2),
                    network_access: HttpNetworkAccess::AllowPrivateNetworks,
                    max_retries: 0,
                    ..HttpRangeOptions::default()
                },
                cancellation,
            )
            .unwrap()
        }
    }
    impl Drop for Origin {
        fn drop(&mut self) {
            if let Some(gate) = self.release.take() {
                let _ = gate.send(false);
            }
            if let Some(worker) = self.worker.take() {
                worker.join().unwrap();
            }
        }
    }
    fn bytes(start: usize, count: usize) -> Vec<u8> {
        (start..start + count)
            .map(|n| u8::try_from(n % 251).unwrap())
            .collect()
    }

    #[test]
    fn opens_at_prefix_and_reads_cached_bytes_during_a_stall() {
        let mut origin = Origin::new();
        let mut input = origin.open(MediaCancellation::new());
        let mut prefix = vec![0; PREFIX];
        input.read_exact(&mut prefix).unwrap();
        assert_eq!(prefix, bytes(0, PREFIX));
        input.seek(SeekFrom::Start(10)).unwrap();
        input.read_exact(&mut prefix[..100]).unwrap();
        assert_eq!(&prefix[..100], bytes(10, 100));
        // Three cancelled transport polls must not turn an ordinary stall into
        // a truncated response or an extra range request.
        std::thread::sleep(Duration::from_millis(350));
        origin.release(true);
        input.seek(SeekFrom::Start(0)).unwrap();
        let mut whole = Vec::new();
        input.read_to_end(&mut whole).unwrap();
        assert_eq!(whole, bytes(0, LENGTH));
    }

    #[test]
    fn forward_seek_waits_for_real_bytes_and_replay_keeps_independent_offsets() {
        let mut origin = Origin::new();
        let mut input = origin.open(MediaCancellation::new());
        let mut replay = input.clone_cached_input().unwrap().unwrap();
        let (done, result) = mpsc::channel();
        input.seek(SeekFrom::Start(100_000)).unwrap();
        let reader = std::thread::spawn(move || {
            let mut data = vec![0; 100];
            input.read_exact(&mut data).unwrap();
            done.send(data).unwrap();
            input
        });
        assert!(
            result.recv_timeout(Duration::from_millis(150)).is_err(),
            "must not read sparse holes"
        );
        origin.release(true);
        assert_eq!(
            result.recv_timeout(Duration::from_secs(2)).unwrap(),
            bytes(100_000, 100)
        );
        let mut input = reader.join().unwrap();
        input.seek(SeekFrom::Start(0)).unwrap();
        let mut original = Vec::new();
        input.read_to_end(&mut original).unwrap();
        drop(input);
        drop(origin);
        replay.seek(SeekFrom::Start(0)).unwrap();
        let mut repeated = Vec::new();
        replay.read_to_end(&mut repeated).unwrap();
        assert_eq!(original, repeated);
        assert_eq!(repeated, bytes(0, LENGTH));
    }

    #[test]
    fn cancellation_and_drop_join_a_downloader_while_origin_is_stalled() {
        let origin = Origin::new();
        let signal = MediaCancellation::new();
        let input = origin.open(signal.clone());
        let started = Instant::now();
        signal.cancel();
        drop(input);
        assert!(
            started.elapsed() < Duration::from_millis(750),
            "body cancellation must not await the two-second request deadline"
        );
        drop(origin);
    }

    #[test]
    fn dropping_the_last_reader_also_cancels_without_external_signal() {
        let origin = Origin::new();
        let input = origin.open(MediaCancellation::new());
        let started = Instant::now();
        drop(input);
        assert!(started.elapsed() < Duration::from_millis(750));
        drop(origin);
    }

    #[test]
    fn truncation_is_an_error_not_clean_eof_or_zero_filled_audio() {
        let mut origin = Origin::new();
        let mut input = origin.open(MediaCancellation::new());
        origin.release(false);
        let mut data = Vec::new();
        let error = input.read_to_end(&mut data).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
        assert_eq!(data, bytes(0, PREFIX));
    }

    #[test]
    fn stalled_body_preserves_inactivity_deadline_and_empty_reads() {
        let origin = Origin::new();
        let started = Instant::now();
        let mut input = origin.open(MediaCancellation::new());
        assert_eq!(input.read(&mut []).unwrap(), 0);
        input.seek(SeekFrom::Start(PREFIX as u64)).unwrap();
        let error = input.read(&mut [0]).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_millis(2750));
        drop(origin);
    }

    // A real socket origin is needed here: phase timeouts and transport polling
    // were both involved in the production failure. The optional first-body
    // truncation forces exact-offset recovery under the same total budget.
    fn paced_origin(recover: bool, header_delay: Duration) -> (String, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/paced", listener.local_addr().unwrap());
        let worker = std::thread::spawn(move || {
            let started = Instant::now();
            for response in 0..=usize::from(recover) {
                let mut socket = loop {
                    match listener.accept() {
                        Ok((socket, _)) => break socket,
                        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                            if started.elapsed() > Duration::from_secs(3) {
                                return;
                            }
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        Err(error) => panic!("origin accept failed: {error}"),
                    }
                };
                socket
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                socket
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    if socket.read(&mut byte).unwrap_or(0) != 1 {
                        return;
                    }
                    request.push(byte[0]);
                    assert!(request.len() <= 16384);
                }
                let request = String::from_utf8(request).unwrap().to_ascii_lowercase();
                let range = request
                    .lines()
                    .find_map(|line| line.strip_prefix("range: bytes="))
                    .unwrap();
                let (start, end) = range.split_once('-').unwrap();
                let start = start.parse::<usize>().unwrap();
                let end = end.parse::<usize>().unwrap().min(LENGTH - 1);
                assert_eq!(start, if response == 0 { 0 } else { 2 * PREFIX });
                std::thread::sleep(header_delay);
                if write!(socket, "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {start}-{end}/{LENGTH}\r\nETag: \"stable\"\r\nConnection: close\r\n\r\n", end+1-start).is_err() { return; }
                for offset in (start..=end).step_by(PREFIX) {
                    if offset != start {
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    if recover && response == 0 && offset == 2 * PREFIX {
                        break;
                    }
                    if socket
                        .write_all(&bytes(offset, PREFIX.min(end + 1 - offset)))
                        .is_err()
                    {
                        return;
                    }
                }
            }
        });
        (url, worker)
    }

    fn paced_options() -> HttpRangeOptions {
        HttpRangeOptions {
            staging_max_bytes: LENGTH as u64,
            progressive_buffer_bytes: PREFIX as u64,
            expected_source_bytes: Some(LENGTH as u64),
            request_timeout: Duration::from_millis(250),
            max_retries: 0,
            network_access: HttpNetworkAccess::AllowPrivateNetworks,
            ..HttpRangeOptions::default()
        }
    }

    #[test]
    fn healthy_progressive_transfer_can_outlive_one_request_timeout() {
        let (url, worker) = paced_origin(false, Duration::ZERO);
        let opened = Instant::now();
        let mut input = HttpRangeInput::open(&url, paced_options()).unwrap();
        assert!(
            opened.elapsed() < Duration::from_millis(250),
            "startup must not await the whole source"
        );
        let mut whole = Vec::new();
        let outcome = input.read_to_end(&mut whole);
        worker.join().unwrap();
        assert!(
            outcome.is_ok(),
            "healthy source was terminated: {outcome:?}, bytes={}",
            whole.len()
        );
        assert_eq!(whole, bytes(0, LENGTH));
        assert!(opened.elapsed() > Duration::from_millis(500));
    }

    #[test]
    fn continuous_progress_does_not_reset_the_total_download_deadline() {
        let (url, worker) = paced_origin(false, Duration::from_millis(100));
        let opened = Instant::now();
        let mut options = paced_options();
        options.progressive_download_timeout = Duration::from_millis(450);
        let mut input = HttpRangeInput::open(&url, options).unwrap();
        let mut whole = Vec::new();
        assert_eq!(
            input.read_to_end(&mut whole).unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
        assert!(opened.elapsed() < Duration::from_millis(700));
        assert!(!whole.is_empty() && whole.len() < LENGTH);
        assert_eq!(whole, bytes(0, whole.len()));
        drop(input);
        worker.join().unwrap();
    }

    #[test]
    fn body_recovery_cannot_reset_the_total_download_deadline() {
        let (url, worker) = paced_origin(true, Duration::from_millis(100));
        let opened = Instant::now();
        let mut options = paced_options();
        options.max_retries = 1;
        options.progressive_download_timeout = Duration::from_millis(550);
        let mut input = HttpRangeInput::open(&url, options).unwrap();
        let mut whole = Vec::new();
        assert_eq!(
            input.read_to_end(&mut whole).unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
        assert!(opened.elapsed() < Duration::from_millis(750));
        assert!(whole.len() > 2 * PREFIX && whole.len() < LENGTH);
        assert_eq!(whole, bytes(0, whole.len()));
        drop(input);
        worker.join().unwrap();
    }

    #[test]
    fn complete_staging_keeps_the_short_request_deadline() {
        let (url, worker) = paced_origin(false, Duration::ZERO);
        let mut options = paced_options();
        options.progressive_buffer_bytes = 0;
        let opened = Instant::now();
        let result = HttpRangeInput::open(&url, options);
        assert!(
            matches!(result, Err(crate::MediaError::Io(ref error)) if error.kind() == io::ErrorKind::TimedOut)
        );
        assert!(opened.elapsed() < Duration::from_millis(750));
        worker.join().unwrap();
    }

    #[test]
    fn rejects_invalid_progressive_bounds_before_network_access() {
        for timeout in [Duration::ZERO, Duration::from_secs(3601)] {
            assert!(matches!(
                HttpRangeInput::open(
                    "http://127.0.0.1:1/",
                    HttpRangeOptions {
                        progressive_download_timeout: timeout,
                        ..HttpRangeOptions::default()
                    }
                ),
                Err(crate::MediaError::InvalidHttpOptions(_))
            ));
        }
        for (limit, prefix) in [
            (0, PREFIX as u64),
            (LENGTH as u64, 1),
            (LENGTH as u64, LENGTH as u64 + 1),
            (2 * 1024 * 1024, 1024 * 1024 + 1),
        ] {
            assert!(
                HttpRangeInput::open(
                    "http://127.0.0.1:1/source",
                    HttpRangeOptions {
                        staging_max_bytes: limit,
                        progressive_buffer_bytes: prefix,
                        ..HttpRangeOptions::default()
                    }
                )
                .is_err()
            );
        }
    }

    #[test]
    fn progressive_startup_avoids_waiting_for_a_delayed_remainder() {
        let mut measured = Vec::new();
        for prefix in [0, PREFIX as u64] {
            let mut origin = Origin::new();
            let gate = origin.release.take().unwrap();
            let release = std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(400));
                let _ = gate.send(true);
            });
            let started = Instant::now();
            let mut input = HttpRangeInput::open(
                &origin.url,
                HttpRangeOptions {
                    staging_max_bytes: LENGTH as u64,
                    progressive_buffer_bytes: prefix,
                    expected_source_bytes: Some(LENGTH as u64),
                    network_access: HttpNetworkAccess::AllowPrivateNetworks,
                    request_timeout: Duration::from_secs(2),
                    ..HttpRangeOptions::default()
                },
            )
            .unwrap();
            let opened = started.elapsed();
            let mut actual = Vec::new();
            input.read_to_end(&mut actual).unwrap();
            assert_eq!(actual, bytes(0, LENGTH));
            release.join().unwrap();
            measured.push(opened);
        }
        eprintln!(
            "128 KiB source with gated 400 ms remainder: full={:.3} ms progressive={:.3} ms; identical bytes",
            measured[0].as_secs_f64() * 1000.0,
            measured[1].as_secs_f64() * 1000.0
        );
        assert!(measured[0] > measured[1] + Duration::from_millis(200));
    }
}
