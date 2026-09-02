use std::env;
use std::error::Error;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::process::{Command, ExitCode};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use mantle_media::{
    HttpNetworkAccess, HttpRangeInput, HttpRangeOptions, HttpStreamInput, HttpStreamOptions,
    RemoteHttpClient, RemoteHttpErrorKind, RemoteHttpOptions, RemoteHttpRequest,
};
use serde::Serialize;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const FULL_DURATION_SECONDS: u64 = 72 * 60 * 60;
const MAX_CHECKPOINTS: u64 = 4_322;
const MAX_CYCLE_DELAY_MS: u64 = 10_000;
const MEMORY_WINDOW: usize = 8;
const SCENARIOS_PER_CYCLE: u64 = 9;
const EXPECTED_FAULTS_PER_CYCLE: u64 = 4;
const REQUESTS_PER_CYCLE: u64 = 24;
const PAYLOAD_BYTES: usize = 4_096;
const RANGE_WINDOW_BYTES: usize = 1_024;

#[derive(Debug)]
struct Config {
    duration_seconds: u64,
    checkpoint_seconds: u64,
    cycle_delay_ms: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
struct ServerCounters {
    range_replay: u64,
    range_redirect: u64,
    range_final: u64,
    range_retry: u64,
    range_truncated: u64,
    range_wrong: u64,
    stream_chunked: u64,
    stream_truncated: u64,
    remote_retry: u64,
    remote_oversized: u64,
}

impl ServerCounters {
    const fn total(self) -> u64 {
        self.range_replay
            .saturating_add(self.range_redirect)
            .saturating_add(self.range_final)
            .saturating_add(self.range_retry)
            .saturating_add(self.range_truncated)
            .saturating_add(self.range_wrong)
            .saturating_add(self.stream_chunked)
            .saturating_add(self.stream_truncated)
            .saturating_add(self.remote_retry)
            .saturating_add(self.remote_oversized)
    }

    fn matches_cycles(self, cycles: u64) -> bool {
        self.range_replay == cycles.saturating_mul(4)
            && self.range_redirect == cycles
            && self.range_final == cycles.saturating_mul(4)
            && self.range_retry == cycles.saturating_mul(8)
            && self.range_truncated == cycles
            && self.range_wrong == cycles
            && self.stream_chunked == cycles
            && self.stream_truncated == cycles
            && self.remote_retry == cycles.saturating_mul(2)
            && self.remote_oversized == cycles
            && self.total() == cycles.saturating_mul(REQUESTS_PER_CYCLE)
    }
}

struct ServerShared {
    payload: Arc<[u8]>,
    counters: Mutex<ServerCounters>,
}

struct ReplayServer {
    address: SocketAddr,
    shared: Arc<ServerShared>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl ReplayServer {
    fn start(payload: Arc<[u8]>) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let shared = Arc::new(ServerShared {
            payload,
            counters: Mutex::new(ServerCounters::default()),
        });
        let thread_shared = Arc::clone(&shared);
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            while !thread_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => serve_request(stream, &thread_shared),
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(1));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            address,
            shared,
            stop,
            thread: Some(thread),
        })
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}/{path}", self.address)
    }

    fn counters(&self) -> Result<ServerCounters> {
        Ok(*self
            .shared
            .counters
            .lock()
            .map_err(|_| "replay counter lock poisoned")?)
    }
}

impl Drop for ReplayServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = TcpStream::connect(self.address);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[derive(Debug)]
struct ReplayRequest {
    path: String,
    range: Option<(u64, u64)>,
}

struct ReplayResponse {
    status: &'static str,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    chunks: Option<Vec<Vec<u8>>>,
    declared_length: Option<usize>,
}

impl ReplayResponse {
    fn status(status: &'static str) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Vec::new(),
            chunks: None,
            declared_length: None,
        }
    }

    fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self {
            body: body.into(),
            ..Self::status("200 OK")
        }
    }

    fn redirect(location: &str) -> Self {
        Self {
            headers: vec![("Location".to_owned(), location.to_owned())],
            ..Self::status("302 Found")
        }
    }

    fn truncated(status: &'static str, body: Vec<u8>, declared_length: usize) -> Self {
        Self {
            status,
            body,
            declared_length: Some(declared_length),
            headers: Vec::new(),
            chunks: None,
        }
    }

    fn chunked(chunks: Vec<Vec<u8>>) -> Self {
        Self {
            chunks: Some(chunks),
            ..Self::status("200 OK")
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ProcCounters {
    cpu_ticks: u64,
    voluntary_context_switches: u64,
    involuntary_context_switches: u64,
}

#[derive(Clone, Copy, Debug)]
struct ProcMemory {
    peak_rss_kib: u64,
    current_rss_kib: u64,
    current_pss_kib: u64,
    threads: u64,
}

#[derive(Debug, Serialize)]
struct MemorySummary {
    first_window_samples: usize,
    last_window_samples: usize,
    first_rss_median_kib: u64,
    last_rss_median_kib: u64,
    rss_growth_kib: i64,
    first_pss_median_kib: u64,
    last_pss_median_kib: u64,
    pss_growth_kib: i64,
    peak_rss_kib: u64,
    max_threads: u64,
}

#[derive(Debug, Serialize)]
struct Checkpoint {
    schema_version: u32,
    kind: &'static str,
    sequence: usize,
    elapsed_seconds: f64,
    cycles: u64,
    scenario_executions: u64,
    expected_faults: u64,
    requests: u64,
    current_rss_kib: u64,
    current_pss_kib: u64,
    peak_rss_kib: u64,
    threads: u64,
}

#[derive(Debug, Serialize)]
struct SoakResult {
    schema_version: u32,
    kind: &'static str,
    status: &'static str,
    configured_duration_seconds: u64,
    elapsed_seconds: f64,
    checkpoint_seconds: u64,
    cycle_delay_ms: u64,
    scenarios_per_cycle: u64,
    expected_faults_per_cycle: u64,
    requests_per_cycle: u64,
    payload_bytes: usize,
    payload_checksum: u64,
    cycles: u64,
    scenario_executions: u64,
    expected_faults: u64,
    unexpected_failures: u64,
    checkpoints: usize,
    cpu_time_ms: f64,
    voluntary_context_switches: u64,
    involuntary_context_switches: u64,
    requests: ServerCounters,
    memory: MemorySummary,
}

fn main() -> ExitCode {
    match run_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("mantle-replay-fault-soak: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run_main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let config = parse_config(&args)?;
    run_soak(&config)
}

fn parse_config(args: &[String]) -> Result<Config> {
    let duration_seconds = bounded_u64(
        args,
        "--duration-seconds",
        1,
        FULL_DURATION_SECONDS,
        FULL_DURATION_SECONDS,
    )?;
    let checkpoint_seconds = bounded_u64(args, "--checkpoint-seconds", 1, 3_600, 60)?;
    let cycle_delay_ms = bounded_u64(args, "--cycle-delay-ms", 0, MAX_CYCLE_DELAY_MS, 5_000)?;
    let checkpoint_count = duration_seconds
        .div_ceil(checkpoint_seconds)
        .saturating_add(2);
    if checkpoint_count > MAX_CHECKPOINTS {
        return Err(format!(
            "replay/fault configuration would emit {checkpoint_count} checkpoints; limit is {MAX_CHECKPOINTS}"
        )
        .into());
    }
    Ok(Config {
        duration_seconds,
        checkpoint_seconds,
        cycle_delay_ms,
    })
}

fn bounded_u64(
    args: &[String],
    name: &str,
    minimum: u64,
    maximum: u64,
    default: u64,
) -> Result<u64> {
    let parsed = args
        .windows(2)
        .find(|pair| pair[0] == name)
        .map_or_else(|| Ok(default), |pair| pair[1].parse())?;
    if !(minimum..=maximum).contains(&parsed) {
        return Err(format!("{name} must be between {minimum} and {maximum}").into());
    }
    Ok(parsed)
}

fn run_soak(config: &Config) -> Result<()> {
    let (payload, server) = warmed_server()?;
    let payload_checksum = checksum(&payload);

    let counters_before = read_proc_counters()?;
    let clock_ticks = clock_ticks_per_second()?;
    let started = Instant::now();
    let duration = Duration::from_secs(config.duration_seconds);
    let checkpoint_period = Duration::from_secs(config.checkpoint_seconds);
    let cycle_delay = Duration::from_millis(config.cycle_delay_ms);
    let capacity = usize::try_from(
        config
            .duration_seconds
            .div_ceil(config.checkpoint_seconds)
            .saturating_add(2),
    )?;
    let mut memory_samples = Vec::with_capacity(capacity);
    let mut cycles = 0_u64;
    let mut next_checkpoint = checkpoint_period;
    let mut output = io::BufWriter::new(io::stdout().lock());
    emit_checkpoint(
        &mut output,
        started,
        cycles,
        server.counters()?,
        &mut memory_samples,
    )?;

    while started.elapsed() < duration {
        run_cycle(&server, &payload)?;
        cycles = cycles.saturating_add(1);
        let request_counters = server.counters()?;
        if !request_counters.matches_cycles(cycles) {
            return Err(format!("request-count oracle diverged after cycle {cycles}").into());
        }
        if started.elapsed() >= next_checkpoint {
            emit_checkpoint(
                &mut output,
                started,
                cycles,
                request_counters,
                &mut memory_samples,
            )?;
            next_checkpoint = next_checkpoint.saturating_add(checkpoint_period);
        }
        thread::sleep(cycle_delay.min(duration.saturating_sub(started.elapsed())));
    }

    let request_counters = server.counters()?;
    emit_checkpoint(
        &mut output,
        started,
        cycles,
        request_counters,
        &mut memory_samples,
    )?;
    let counters_after = read_proc_counters()?;
    let cpu_ticks = counters_after
        .cpu_ticks
        .saturating_sub(counters_before.cpu_ticks);
    let cpu_ticks = u32::try_from(cpu_ticks).map_err(|_| "soak CPU tick delta exceeds u32")?;
    let result = SoakResult {
        schema_version: 1,
        kind: "result",
        status: "PASS",
        configured_duration_seconds: config.duration_seconds,
        elapsed_seconds: started.elapsed().as_secs_f64(),
        checkpoint_seconds: config.checkpoint_seconds,
        cycle_delay_ms: config.cycle_delay_ms,
        scenarios_per_cycle: SCENARIOS_PER_CYCLE,
        expected_faults_per_cycle: EXPECTED_FAULTS_PER_CYCLE,
        requests_per_cycle: REQUESTS_PER_CYCLE,
        payload_bytes: PAYLOAD_BYTES,
        payload_checksum,
        cycles,
        scenario_executions: cycles.saturating_mul(SCENARIOS_PER_CYCLE),
        expected_faults: cycles.saturating_mul(EXPECTED_FAULTS_PER_CYCLE),
        unexpected_failures: 0,
        checkpoints: memory_samples.len(),
        cpu_time_ms: f64::from(cpu_ticks) * 1_000.0 / f64::from(clock_ticks),
        voluntary_context_switches: counters_after
            .voluntary_context_switches
            .saturating_sub(counters_before.voluntary_context_switches),
        involuntary_context_switches: counters_after
            .involuntary_context_switches
            .saturating_sub(counters_before.involuntary_context_switches),
        requests: request_counters,
        memory: summarize_memory(&memory_samples)?,
    };
    serde_json::to_writer(&mut output, &result)?;
    writeln!(output)?;
    output.flush()?;
    Ok(())
}

fn warmed_server() -> Result<(Arc<[u8]>, ReplayServer)> {
    let payload: Arc<[u8]> = (0..PAYLOAD_BYTES)
        .map(|index| u8::try_from((index * 31 + 7) % 251).expect("payload byte is bounded"))
        .collect::<Vec<_>>()
        .into();
    let server = ReplayServer::start(Arc::clone(&payload))?;
    run_cycle(&server, &payload)?;
    let warmup_counters = server.counters()?;
    if !warmup_counters.matches_cycles(1) {
        return Err(
            format!("replay/fault warm-up request counts diverged: {warmup_counters:?}").into(),
        );
    }

    // The retained campaign excludes warm-up traffic from its exact per-cycle request oracle.
    {
        let mut counters = server
            .shared
            .counters
            .lock()
            .map_err(|_| "replay counter lock poisoned")?;
        *counters = ServerCounters::default();
    }
    Ok((payload, server))
}

fn run_cycle(server: &ReplayServer, payload: &[u8]) -> Result<()> {
    let mut replay = HttpRangeInput::open(server.url("range"), range_options(0, 1))?;
    let mut body = Vec::new();
    replay.read_to_end(&mut body)?;
    require_body("range replay", &body, payload)?;

    let mut redirected = HttpRangeInput::open(server.url("range-start"), range_options(1, 0))?;
    body.clear();
    redirected.read_to_end(&mut body)?;
    require_body("range redirect", &body, payload)?;

    let mut retried = HttpRangeInput::open(server.url("range-retry"), range_options(0, 1))?;
    body.clear();
    retried.read_to_end(&mut body)?;
    require_body("range retry", &body, payload)?;

    let mut truncated = HttpRangeInput::open(server.url("range-truncated"), range_options(0, 0))?;
    let error = truncated
        .read_to_end(&mut Vec::new())
        .expect_err("truncated range must fail");
    if error.kind() != io::ErrorKind::UnexpectedEof {
        return Err(format!("truncated range returned {error}").into());
    }

    let error = HttpRangeInput::open(server.url("range-wrong"), range_options(0, 0))
        .err()
        .ok_or("wrong range unexpectedly opened")?;
    if !error.to_string().contains("Content-Range begins") {
        return Err(format!("wrong range returned {error}").into());
    }

    let mut chunked = HttpStreamInput::open(server.url("stream-chunked"), stream_options())?;
    body.clear();
    chunked.read_to_end(&mut body)?;
    require_body("chunked stream", &body, b"chunked-replay")?;

    let mut truncated_stream =
        HttpStreamInput::open(server.url("stream-truncated"), stream_options())?;
    let error = truncated_stream
        .read_to_end(&mut Vec::new())
        .expect_err("truncated stream must fail");
    if error.kind() != io::ErrorKind::UnexpectedEof {
        return Err(format!("truncated stream returned {error}").into());
    }

    let remote = remote_client(64, 1)?;
    let request = RemoteHttpRequest::get(server.url("remote-retry"))?;
    let response = remote.execute(&request)?;
    require_body("remote retry", response.body(), b"remote-ok")?;

    let remote = remote_client(64, 0)?;
    let request = RemoteHttpRequest::get(server.url("remote-oversized"))?;
    let error = remote
        .execute(&request)
        .err()
        .ok_or("oversized remote response unexpectedly succeeded")?;
    if error.kind() != RemoteHttpErrorKind::ResponseTooLarge {
        return Err(format!("oversized remote response returned {error}").into());
    }
    Ok(())
}

fn range_options(max_redirects: u32, max_retries: u32) -> HttpRangeOptions {
    HttpRangeOptions {
        range_window_bytes: RANGE_WINDOW_BYTES,
        max_source_bytes: PAYLOAD_BYTES as u64,
        connect_timeout: Duration::from_secs(2),
        request_timeout: Duration::from_secs(2),
        max_redirects,
        max_retries,
        network_access: HttpNetworkAccess::AllowPrivateNetworks,
        ..HttpRangeOptions::default()
    }
}

fn stream_options() -> HttpStreamOptions {
    HttpStreamOptions {
        max_response_bytes: 64,
        connect_timeout: Duration::from_secs(2),
        request_timeout: Duration::from_secs(2),
        max_redirects: 0,
        max_retries: 0,
        network_access: HttpNetworkAccess::AllowPrivateNetworks,
        ..HttpStreamOptions::default()
    }
}

fn remote_client(max_response_bytes: u64, max_retries: u32) -> Result<RemoteHttpClient> {
    Ok(RemoteHttpClient::new(RemoteHttpOptions {
        max_response_bytes,
        connect_timeout: Duration::from_secs(2),
        request_timeout: Duration::from_secs(2),
        max_redirects: 0,
        max_retries,
        retry_base_delay: Duration::from_millis(1),
        retry_max_delay: Duration::from_millis(2),
        retry_after_max_delay: Duration::from_millis(2),
        network_access: HttpNetworkAccess::AllowPrivateNetworks,
        ..RemoteHttpOptions::default()
    })?)
}

fn require_body(name: &str, actual: &[u8], expected: &[u8]) -> Result<()> {
    if actual != expected {
        return Err(format!("{name} body fingerprint diverged").into());
    }
    Ok(())
}

fn serve_request(mut stream: TcpStream, shared: &ServerShared) {
    let Ok(request) = read_request(&mut stream) else {
        return;
    };
    let Ok(response) = response_for(&request, shared) else {
        return;
    };
    let _ = write_response(&mut stream, response);
}

fn read_request(stream: &mut TcpStream) -> Result<ReplayRequest> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let mut raw = [0_u8; 16 * 1024];
    let mut used = 0_usize;
    while used < raw.len() {
        let count = stream.read(&mut raw[used..])?;
        if count == 0 {
            return Err("request ended before its headers".into());
        }
        used += count;
        if raw[..used].windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    let text = String::from_utf8_lossy(&raw[..used]);
    let target = text
        .lines()
        .next()
        .and_then(|line| line.split_ascii_whitespace().nth(1))
        .ok_or("request target missing")?;
    let range = text.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if !name.eq_ignore_ascii_case("range") {
            return None;
        }
        let value = value.trim().strip_prefix("bytes=")?;
        let (start, end) = value.split_once('-')?;
        Some((start.parse().ok()?, end.parse().ok()?))
    });
    Ok(ReplayRequest {
        path: target.split('?').next().unwrap_or(target).to_owned(),
        range,
    })
}

fn response_for(request: &ReplayRequest, shared: &ServerShared) -> Result<ReplayResponse> {
    let mut counters = shared
        .counters
        .lock()
        .map_err(|_| "replay counter lock poisoned")?;
    let response = match request.path.as_str() {
        "/range" => {
            counters.range_replay = counters.range_replay.saturating_add(1);
            partial_response(request, &shared.payload, false)?
        }
        "/range-start" => {
            counters.range_redirect = counters.range_redirect.saturating_add(1);
            ReplayResponse::redirect("/range-final")
        }
        "/range-final" => {
            counters.range_final = counters.range_final.saturating_add(1);
            partial_response(request, &shared.payload, false)?
        }
        "/range-retry" => {
            let attempt = counters.range_retry;
            counters.range_retry = counters.range_retry.saturating_add(1);
            if attempt % 2 == 0 {
                ReplayResponse::status("503 Service Unavailable")
            } else {
                partial_response(request, &shared.payload, false)?
            }
        }
        "/range-truncated" => {
            counters.range_truncated = counters.range_truncated.saturating_add(1);
            partial_response(request, &shared.payload, true)?
        }
        "/range-wrong" => {
            counters.range_wrong = counters.range_wrong.saturating_add(1);
            wrong_range_response(&shared.payload)
        }
        "/stream-chunked" => {
            counters.stream_chunked = counters.stream_chunked.saturating_add(1);
            ReplayResponse::chunked(vec![b"chunked-".to_vec(), b"replay".to_vec()])
        }
        "/stream-truncated" => {
            counters.stream_truncated = counters.stream_truncated.saturating_add(1);
            ReplayResponse::truncated("200 OK", b"short".to_vec(), 10)
        }
        "/remote-retry" => {
            let attempt = counters.remote_retry;
            counters.remote_retry = counters.remote_retry.saturating_add(1);
            if attempt % 2 == 0 {
                ReplayResponse::status("503 Service Unavailable")
            } else {
                ReplayResponse::ok(b"remote-ok".to_vec())
            }
        }
        "/remote-oversized" => {
            counters.remote_oversized = counters.remote_oversized.saturating_add(1);
            ReplayResponse::ok(vec![b'x'; 65])
        }
        _ => ReplayResponse::status("404 Not Found"),
    };
    Ok(response)
}

fn partial_response(
    request: &ReplayRequest,
    payload: &[u8],
    truncate: bool,
) -> Result<ReplayResponse> {
    let (start, requested_end) = request.range.ok_or("range request header missing")?;
    let end = requested_end.min(u64::try_from(payload.len())?.saturating_sub(1));
    let start_index = usize::try_from(start)?;
    let end_index = usize::try_from(end)?;
    let complete = payload
        .get(start_index..=end_index)
        .ok_or("range request outside payload")?;
    let body = if truncate {
        complete[..complete.len() / 2].to_vec()
    } else {
        complete.to_vec()
    };
    Ok(ReplayResponse {
        status: "206 Partial Content",
        headers: vec![
            (
                "Content-Range".to_owned(),
                format!("bytes {start}-{end}/{}", payload.len()),
            ),
            ("ETag".to_owned(), "\"phase15-stable\"".to_owned()),
        ],
        declared_length: Some(complete.len()),
        body,
        chunks: None,
    })
}

fn wrong_range_response(payload: &[u8]) -> ReplayResponse {
    ReplayResponse {
        status: "206 Partial Content",
        headers: vec![(
            "Content-Range".to_owned(),
            format!("bytes 1-{RANGE_WINDOW_BYTES}/{}", payload.len()),
        )],
        body: payload[1..=RANGE_WINDOW_BYTES].to_vec(),
        chunks: None,
        declared_length: Some(RANGE_WINDOW_BYTES),
    }
}

fn write_response(stream: &mut TcpStream, response: ReplayResponse) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {}\r\nConnection: close\r\n",
        response.status
    )?;
    for (name, value) in response.headers {
        write!(stream, "{name}: {value}\r\n")?;
    }
    if let Some(chunks) = response.chunks {
        stream.write_all(b"Transfer-Encoding: chunked\r\n\r\n")?;
        for chunk in chunks {
            write!(stream, "{:x}\r\n", chunk.len())?;
            stream.write_all(&chunk)?;
            stream.write_all(b"\r\n")?;
        }
        stream.write_all(b"0\r\n\r\n")?;
    } else {
        write!(
            stream,
            "Content-Length: {}\r\n\r\n",
            response.declared_length.unwrap_or(response.body.len())
        )?;
        stream.write_all(&response.body)?;
    }
    stream.flush()?;
    stream.shutdown(Shutdown::Both)
}

fn emit_checkpoint(
    output: &mut impl Write,
    started: Instant,
    cycles: u64,
    requests: ServerCounters,
    samples: &mut Vec<ProcMemory>,
) -> Result<()> {
    if samples.len() >= usize::try_from(MAX_CHECKPOINTS)? {
        return Err("replay/fault checkpoint limit exceeded".into());
    }
    let memory = read_proc_memory()?;
    samples.push(memory);
    let checkpoint = Checkpoint {
        schema_version: 1,
        kind: "checkpoint",
        sequence: samples.len(),
        elapsed_seconds: started.elapsed().as_secs_f64(),
        cycles,
        scenario_executions: cycles.saturating_mul(SCENARIOS_PER_CYCLE),
        expected_faults: cycles.saturating_mul(EXPECTED_FAULTS_PER_CYCLE),
        requests: requests.total(),
        current_rss_kib: memory.current_rss_kib,
        current_pss_kib: memory.current_pss_kib,
        peak_rss_kib: memory.peak_rss_kib,
        threads: memory.threads,
    };
    serde_json::to_writer(&mut *output, &checkpoint)?;
    writeln!(output)?;
    output.flush()?;
    Ok(())
}

fn summarize_memory(samples: &[ProcMemory]) -> Result<MemorySummary> {
    if samples.is_empty() {
        return Err("replay/fault soak did not record memory".into());
    }
    let window = samples.len().min(MEMORY_WINDOW);
    let first = &samples[..window];
    let last = &samples[samples.len() - window..];
    let rss_at_start = median(first.iter().map(|sample| sample.current_rss_kib));
    let rss_at_end = median(last.iter().map(|sample| sample.current_rss_kib));
    let proportional_at_start = median(first.iter().map(|sample| sample.current_pss_kib));
    let proportional_at_end = median(last.iter().map(|sample| sample.current_pss_kib));
    Ok(MemorySummary {
        first_window_samples: window,
        last_window_samples: window,
        first_rss_median_kib: rss_at_start,
        last_rss_median_kib: rss_at_end,
        rss_growth_kib: signed_growth(rss_at_end, rss_at_start),
        first_pss_median_kib: proportional_at_start,
        last_pss_median_kib: proportional_at_end,
        pss_growth_kib: signed_growth(proportional_at_end, proportional_at_start),
        peak_rss_kib: samples
            .iter()
            .map(|sample| sample.peak_rss_kib)
            .max()
            .unwrap_or(0),
        max_threads: samples
            .iter()
            .map(|sample| sample.threads)
            .max()
            .unwrap_or(0),
    })
}

fn median(values: impl Iterator<Item = u64>) -> u64 {
    let mut values = values.collect::<Vec<_>>();
    values.sort_unstable();
    values[values.len() / 2]
}

fn signed_growth(end: u64, start: u64) -> i64 {
    i64::try_from(i128::from(end) - i128::from(start)).unwrap_or(i64::MAX)
}

fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn read_proc_counters() -> Result<ProcCounters> {
    let stat = fs::read_to_string("/proc/self/stat")?;
    let fields = stat
        .rsplit_once(") ")
        .ok_or("unexpected /proc/self/stat format")?
        .1
        .split_whitespace()
        .collect::<Vec<_>>();
    let user_ticks: u64 = fields.get(11).ok_or("missing user CPU ticks")?.parse()?;
    let system_ticks: u64 = fields.get(12).ok_or("missing system CPU ticks")?.parse()?;
    let status = fs::read_to_string("/proc/self/status")?;
    Ok(ProcCounters {
        cpu_ticks: user_ticks.saturating_add(system_ticks),
        voluntary_context_switches: status_value(&status, "voluntary_ctxt_switches:")?,
        involuntary_context_switches: status_value(&status, "nonvoluntary_ctxt_switches:")?,
    })
}

fn read_proc_memory() -> Result<ProcMemory> {
    let status = fs::read_to_string("/proc/self/status")?;
    let smaps = fs::read_to_string("/proc/self/smaps_rollup")?;
    Ok(ProcMemory {
        peak_rss_kib: status_value(&status, "VmHWM:")?,
        current_rss_kib: status_value(&status, "VmRSS:")?,
        current_pss_kib: status_value(&smaps, "Pss:")?,
        threads: status_value(&status, "Threads:")?,
    })
}

fn status_value(text: &str, key: &str) -> Result<u64> {
    text.lines()
        .find_map(|line| {
            line.strip_prefix(key)
                .and_then(|rest| rest.split_whitespace().next())
                .and_then(|value| value.parse().ok())
        })
        .ok_or_else(|| format!("missing {key} in Linux process data").into())
}

fn clock_ticks_per_second() -> Result<u32> {
    let output = Command::new("getconf").arg("CLK_TCK").output()?;
    if !output.status.success() {
        return Err("getconf CLK_TCK failed".into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().parse()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_the_bounded_full_checkpoint_schedule() {
        let full = [
            "--duration-seconds".to_owned(),
            "259200".to_owned(),
            "--checkpoint-seconds".to_owned(),
            "60".to_owned(),
            "--cycle-delay-ms".to_owned(),
            "5000".to_owned(),
        ];
        let config = parse_config(&full).unwrap();
        assert_eq!(config.duration_seconds, FULL_DURATION_SECONDS);
        assert_eq!(config.checkpoint_seconds, 60);
        assert_eq!(config.cycle_delay_ms, 5_000);

        let too_many = full.map(|value| {
            if value == "60" {
                "59".to_owned()
            } else {
                value
            }
        });
        assert!(parse_config(&too_many).is_err());
    }

    #[test]
    fn request_counts_are_exact_and_constant_size() {
        let counters = ServerCounters {
            range_replay: 8,
            range_redirect: 2,
            range_final: 8,
            range_retry: 16,
            range_truncated: 2,
            range_wrong: 2,
            stream_chunked: 2,
            stream_truncated: 2,
            remote_retry: 4,
            remote_oversized: 2,
        };
        assert!(counters.matches_cycles(2));
        assert_eq!(counters.total(), 48);
        assert!(!counters.matches_cycles(3));
    }

    #[test]
    fn summarizes_fixed_memory_edge_windows() {
        let samples = (0_u64..16)
            .map(|index| ProcMemory {
                peak_rss_kib: 400 + index,
                current_rss_kib: if index < 8 { 100 + index } else { 192 + index },
                current_pss_kib: if index < 8 { 80 + index } else { 122 + index },
                threads: 2,
            })
            .collect::<Vec<_>>();
        let summary = summarize_memory(&samples).unwrap();
        assert_eq!(summary.rss_growth_kib, 100);
        assert_eq!(summary.pss_growth_kib, 50);
        assert_eq!(summary.peak_rss_kib, 415);
        assert_eq!(summary.max_threads, 2);
    }
}
