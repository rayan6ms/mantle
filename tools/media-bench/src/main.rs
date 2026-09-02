use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::mem;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use mantle_audio::{
    EncodedFrameConsumer, EncodedFrameProducer, EncodedFrameSlot, OpusPassthrough, PcmFormat,
    encoded_frame_queue,
};
use mantle_media::{
    Codec, EncodedPacket, HttpNetworkAccess, HttpRangeInput, HttpRangeOptions, MediaLimits,
    MediaSession, PcmFrame,
};
use serde::Serialize;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const MAX_SOAK_DURATION_SECONDS: u64 = 24 * 60 * 60;
const MAX_SOAK_CHECKPOINTS: u64 = 1_442;
const MAX_SOAK_CYCLE_DELAY_MS: u64 = 1_000;
const SOAK_MEMORY_WINDOW: usize = 8;
const SOAK_WORKLOADS: [(&str, &str); 5] = [
    ("wav-decode-local", "reference.wav"),
    ("mp3-decode-local", "reference.mp3"),
    ("aac-decode-local", "reference.m4a"),
    ("flac-decode-local", "reference.flac"),
    ("opus-passthrough-local", "reference.webm"),
];

#[derive(Debug)]
enum RunMode {
    Local(PathBuf),
    Http(String),
}

#[derive(Debug)]
struct RunConfig {
    workload: String,
    input: RunMode,
    repetition: usize,
    seek: bool,
}

#[derive(Debug)]
struct ServeConfig {
    root: PathBuf,
    address: String,
}

#[derive(Debug)]
struct SoakConfig {
    fixture_root: PathBuf,
    duration_seconds: u64,
    checkpoint_seconds: u64,
    cycle_delay_ms: u64,
}

#[derive(Clone, Copy, Debug, Serialize)]
struct Summary {
    min: f64,
    median: f64,
    p95: f64,
    max: f64,
    mean: f64,
    samples: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
struct Consumption {
    output_units: u64,
    decoded_samples: u64,
    encoded_bytes: u64,
    checksum: u64,
}

#[derive(Clone, Debug, Serialize)]
struct SoakFingerprint {
    workload: &'static str,
    output_units: u64,
    decoded_samples: u64,
    encoded_bytes: u64,
    checksum: u64,
}

#[derive(Debug, Serialize)]
struct SoakCheckpoint {
    schema_version: u32,
    kind: &'static str,
    checkpoint: usize,
    elapsed_seconds: f64,
    sessions: u64,
    completed_cycles: u64,
    current_rss_kib: u64,
    current_pss_kib: u64,
    peak_rss_kib: u64,
    threads: u64,
}

#[derive(Debug, Serialize)]
struct SoakMemory {
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
struct SoakResult {
    schema_version: u32,
    kind: &'static str,
    status: &'static str,
    configured_duration_seconds: u64,
    elapsed_seconds: f64,
    checkpoint_seconds: u64,
    cycle_delay_ms: u64,
    workloads: usize,
    sessions: u64,
    completed_cycles: u64,
    fingerprint_mismatches: u64,
    checkpoints: usize,
    cpu_time_ms: f64,
    voluntary_context_switches: u64,
    involuntary_context_switches: u64,
    memory: SoakMemory,
    fingerprints: Vec<SoakFingerprint>,
}

enum PlaybackOutput {
    Opus(Box<OpusPlaybackOutput>),
    Pcm(PcmFrame),
}

struct OpusPlaybackOutput {
    packet: EncodedPacket,
    router: OpusPassthrough,
    sender: EncodedFrameProducer,
    receiver: EncodedFrameConsumer,
    write_slot: EncodedFrameSlot,
    read_slot: EncodedFrameSlot,
}

impl PlaybackOutput {
    fn new(session: &MediaSession) -> Result<Self> {
        if session.info().codec == Codec::Opus {
            let format = PcmFormat::new(session.info().sample_rate, session.info().channels)?;
            let (sender, receiver) = encoded_frame_queue(1)?;
            Ok(Self::Opus(Box::new(OpusPlaybackOutput {
                packet: EncodedPacket::with_capacity(session.limits().max_packet_bytes),
                router: OpusPassthrough::new(format),
                sender,
                receiver,
                write_slot: EncodedFrameSlot::new(),
                read_slot: EncodedFrameSlot::new(),
            })))
        } else {
            Ok(Self::Pcm(PcmFrame::with_capacity(
                session.limits().max_pcm_samples_per_frame,
            )))
        }
    }

    fn read(&mut self, session: &mut MediaSession, consumption: &mut Consumption) -> Result<bool> {
        match self {
            Self::Opus(output) => {
                if !session.read_encoded(&mut output.packet)? {
                    return Ok(false);
                }
                if !output
                    .router
                    .route_packet(
                        output.packet.data(),
                        output.packet.timestamp(),
                        &mut output.write_slot,
                    )?
                    .delivered()
                {
                    return Err("benchmark Opus packet unexpectedly requires transcoding".into());
                }
                output
                    .sender
                    .try_push(mem::take(&mut output.write_slot))
                    .map_err(|_| "benchmark frame queue unexpectedly filled")?;
                if !output.receiver.try_pop_into(&mut output.read_slot) {
                    return Err("benchmark frame queue unexpectedly emptied".into());
                }
                record_packet(&output.read_slot, consumption);
                mem::swap(&mut output.write_slot, &mut output.read_slot);
                Ok(true)
            }
            Self::Pcm(frame) => {
                if !session.read_pcm(frame)? {
                    return Ok(false);
                }
                record_frame(frame, consumption);
                Ok(true)
            }
        }
    }

    fn reset(&mut self) {
        if let Self::Opus(output) = self {
            output.router.reset();
            output.receiver.clear();
            output.write_slot.clear();
            output.read_slot.clear();
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
struct BenchmarkResult {
    schema_version: u32,
    timestamp_unix_ms: u128,
    workload: String,
    repetition: usize,
    input_mode: &'static str,
    source_duration_ms: Option<u128>,
    codec: String,
    load_elapsed_ms: f64,
    first_output_elapsed_ms: f64,
    processing_elapsed_ms: f64,
    cpu_time_ms: f64,
    realtime_multiple: Option<f64>,
    seek_latency_ms: Option<Summary>,
    output_units: u64,
    decoded_samples: u64,
    encoded_bytes: u64,
    checksum: u64,
    peak_rss_kib: u64,
    current_rss_kib: u64,
    current_pss_kib: u64,
    threads: u64,
    voluntary_context_switches: u64,
    involuntary_context_switches: u64,
}

fn main() -> ExitCode {
    match run_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("mantle-media-bench: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run_main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.first().map(String::as_str) {
        Some("run") => run_benchmark(parse_run_config(&args[1..])?),
        Some("soak") => run_soak(&parse_soak_config(&args[1..])?),
        Some("serve") => serve(&parse_serve_config(&args[1..])?),
        _ => {
            Err("usage: mantle-media-bench run <options> | soak <options> | serve <options>".into())
        }
    }
}

fn parse_run_config(args: &[String]) -> Result<RunConfig> {
    let workload = required_value(args, "--workload")?;
    let raw_input = required_value(args, "--input")?;
    let repetition = value(args, "--repetition")
        .unwrap_or_else(|| "1".to_owned())
        .parse()?;
    let http = args.iter().any(|argument| argument == "--http");
    let seek = args.iter().any(|argument| argument == "--seek");
    Ok(RunConfig {
        workload,
        input: if http {
            RunMode::Http(raw_input)
        } else {
            RunMode::Local(PathBuf::from(raw_input))
        },
        repetition,
        seek,
    })
}

fn parse_serve_config(args: &[String]) -> Result<ServeConfig> {
    Ok(ServeConfig {
        root: PathBuf::from(required_value(args, "--root")?),
        address: value(args, "--address").unwrap_or_else(|| "127.0.0.1:18081".to_owned()),
    })
}

fn parse_soak_config(args: &[String]) -> Result<SoakConfig> {
    let duration_seconds = bounded_u64(
        args,
        "--duration-seconds",
        1,
        MAX_SOAK_DURATION_SECONDS,
        Some(MAX_SOAK_DURATION_SECONDS),
    )?;
    let checkpoint_seconds = bounded_u64(args, "--checkpoint-seconds", 1, 60 * 60, Some(60))?;
    let cycle_delay_ms = bounded_u64(
        args,
        "--cycle-delay-ms",
        0,
        MAX_SOAK_CYCLE_DELAY_MS,
        Some(250),
    )?;
    let checkpoints = duration_seconds
        .div_ceil(checkpoint_seconds)
        .saturating_add(2);
    if checkpoints > MAX_SOAK_CHECKPOINTS {
        return Err(format!(
            "soak configuration would emit {checkpoints} checkpoints; limit is {MAX_SOAK_CHECKPOINTS}"
        )
        .into());
    }
    Ok(SoakConfig {
        fixture_root: PathBuf::from(required_value(args, "--fixture-root")?),
        duration_seconds,
        checkpoint_seconds,
        cycle_delay_ms,
    })
}

fn value(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}

fn required_value(args: &[String], name: &str) -> Result<String> {
    value(args, name).ok_or_else(|| format!("missing required option {name}").into())
}

fn bounded_u64(
    args: &[String],
    name: &str,
    minimum: u64,
    maximum: u64,
    default: Option<u64>,
) -> Result<u64> {
    let parsed = match value(args, name) {
        Some(raw) => raw.parse()?,
        None => default.ok_or_else(|| format!("missing required option {name}"))?,
    };
    if !(minimum..=maximum).contains(&parsed) {
        return Err(format!("{name} must be between {minimum} and {maximum}").into());
    }
    Ok(parsed)
}

fn run_soak(config: &SoakConfig) -> Result<()> {
    let mut fingerprints = Vec::with_capacity(SOAK_WORKLOADS.len());
    for (workload, filename) in SOAK_WORKLOADS {
        let consumption = consume_local_fixture(&config.fixture_root.join(filename))?;
        fingerprints.push(SoakFingerprint {
            workload,
            output_units: consumption.output_units,
            decoded_samples: consumption.decoded_samples,
            encoded_bytes: consumption.encoded_bytes,
            checksum: consumption.checksum,
        });
    }

    let counters_before = read_proc_counters()?;
    let clock_ticks = clock_ticks_per_second()?;
    let started = Instant::now();
    let duration = Duration::from_secs(config.duration_seconds);
    let checkpoint_period = Duration::from_secs(config.checkpoint_seconds);
    let cycle_delay = Duration::from_millis(config.cycle_delay_ms);
    let checkpoint_capacity = usize::try_from(
        config
            .duration_seconds
            .div_ceil(config.checkpoint_seconds)
            .saturating_add(2),
    )?;
    let mut memory_samples = Vec::with_capacity(checkpoint_capacity);
    let mut sessions = 0_u64;
    let mut next_checkpoint = checkpoint_period;
    let mut output = io::BufWriter::new(io::stdout().lock());
    emit_soak_checkpoint(&mut output, started, sessions, &mut memory_samples)?;

    'soak: while started.elapsed() < duration {
        for (index, (_, filename)) in SOAK_WORKLOADS.iter().enumerate() {
            if started.elapsed() >= duration {
                break 'soak;
            }
            let consumption = consume_local_fixture(&config.fixture_root.join(filename))?;
            let expected = fingerprints.get(index).ok_or("missing soak fingerprint")?;
            if consumption.output_units != expected.output_units
                || consumption.decoded_samples != expected.decoded_samples
                || consumption.encoded_bytes != expected.encoded_bytes
                || consumption.checksum != expected.checksum
            {
                return Err(format!("soak fingerprint mismatch for {}", expected.workload).into());
            }
            sessions = sessions.saturating_add(1);
            if started.elapsed() >= next_checkpoint {
                emit_soak_checkpoint(&mut output, started, sessions, &mut memory_samples)?;
                next_checkpoint = next_checkpoint.saturating_add(checkpoint_period);
            }
            let remaining = duration.saturating_sub(started.elapsed());
            thread_sleep(cycle_delay.min(remaining));
        }
    }

    emit_soak_checkpoint(&mut output, started, sessions, &mut memory_samples)?;
    let counters_after = read_proc_counters()?;
    let cpu_ticks = counters_after
        .cpu_ticks
        .saturating_sub(counters_before.cpu_ticks);
    let cpu_ticks = u32::try_from(cpu_ticks).map_err(|_| "soak CPU tick delta exceeds u32")?;
    let cpu_time_ms = f64::from(cpu_ticks) * 1000.0 / f64::from(clock_ticks);
    let result = SoakResult {
        schema_version: 1,
        kind: "result",
        status: "PASS",
        configured_duration_seconds: config.duration_seconds,
        elapsed_seconds: started.elapsed().as_secs_f64(),
        checkpoint_seconds: config.checkpoint_seconds,
        cycle_delay_ms: config.cycle_delay_ms,
        workloads: SOAK_WORKLOADS.len(),
        sessions,
        completed_cycles: sessions / u64::try_from(SOAK_WORKLOADS.len())?,
        fingerprint_mismatches: 0,
        checkpoints: memory_samples.len(),
        cpu_time_ms,
        voluntary_context_switches: counters_after
            .voluntary_context_switches
            .saturating_sub(counters_before.voluntary_context_switches),
        involuntary_context_switches: counters_after
            .involuntary_context_switches
            .saturating_sub(counters_before.involuntary_context_switches),
        memory: summarize_soak_memory(&memory_samples)?,
        fingerprints,
    };
    serde_json::to_writer(&mut output, &result)?;
    writeln!(output)?;
    output.flush()?;
    Ok(())
}

fn consume_local_fixture(path: &Path) -> Result<Consumption> {
    let mut session = MediaSession::open_file(path, MediaLimits::default())?;
    let mut output = PlaybackOutput::new(&session)?;
    let mut consumption = Consumption::default();
    read_one(&mut session, &mut output, &mut consumption)?;
    consume_remaining(&mut session, &mut output, &mut consumption)?;
    Ok(consumption)
}

fn emit_soak_checkpoint(
    output: &mut impl Write,
    started: Instant,
    sessions: u64,
    memory_samples: &mut Vec<ProcMemory>,
) -> Result<()> {
    if memory_samples.len() >= usize::try_from(MAX_SOAK_CHECKPOINTS)? {
        return Err("soak checkpoint limit exceeded".into());
    }
    let memory = read_proc_memory()?;
    memory_samples.push(memory);
    let checkpoint = SoakCheckpoint {
        schema_version: 1,
        kind: "checkpoint",
        checkpoint: memory_samples.len(),
        elapsed_seconds: started.elapsed().as_secs_f64(),
        sessions,
        completed_cycles: sessions / u64::try_from(SOAK_WORKLOADS.len())?,
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

fn summarize_soak_memory(samples: &[ProcMemory]) -> Result<SoakMemory> {
    if samples.is_empty() {
        return Err("soak did not record a memory sample".into());
    }
    let window = samples.len().min(SOAK_MEMORY_WINDOW);
    let first = &samples[..window];
    let last = &samples[samples.len() - window..];
    let rss_at_start = median_u64(first.iter().map(|sample| sample.current_rss_kib));
    let rss_at_end = median_u64(last.iter().map(|sample| sample.current_rss_kib));
    let proportional_at_start = median_u64(first.iter().map(|sample| sample.current_pss_kib));
    let proportional_at_end = median_u64(last.iter().map(|sample| sample.current_pss_kib));
    Ok(SoakMemory {
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

fn median_u64(values: impl Iterator<Item = u64>) -> u64 {
    let mut values = values.collect::<Vec<_>>();
    values.sort_unstable();
    values[values.len() / 2]
}

fn signed_growth(end: u64, start: u64) -> i64 {
    let difference = i128::from(end) - i128::from(start);
    i64::try_from(difference).unwrap_or_else(|_| {
        if difference.is_negative() {
            i64::MIN
        } else {
            i64::MAX
        }
    })
}

fn thread_sleep(duration: Duration) {
    if !duration.is_zero() {
        std::thread::sleep(duration);
    }
}

fn run_benchmark(config: RunConfig) -> Result<()> {
    let counters_before = read_proc_counters()?;
    let clock_ticks = clock_ticks_per_second()?;
    let load_started = Instant::now();
    let mut session = match &config.input {
        RunMode::Local(path) => MediaSession::open_file(path, MediaLimits::default())?,
        RunMode::Http(url) => {
            let options = HttpRangeOptions {
                network_access: HttpNetworkAccess::AllowPrivateNetworks,
                ..HttpRangeOptions::default()
            };
            let input = HttpRangeInput::open(url, options)?;
            MediaSession::open(Box::new(input), Some("mp3"), MediaLimits::default())?
        }
    };
    let load_elapsed_ms = elapsed_ms(load_started);
    let codec = session.info().codec;
    let duration = session.info().duration;
    let mut output = PlaybackOutput::new(&session)?;
    let processing_started = Instant::now();
    let first_started = Instant::now();
    let mut consumption = Consumption::default();
    read_one(&mut session, &mut output, &mut consumption)?;
    let first_output_elapsed_ms = elapsed_ms(first_started);
    let seek_latency_ms = if config.seek {
        Some(measure_seeks(&mut session, &mut output, &mut consumption)?)
    } else {
        consume_remaining(&mut session, &mut output, &mut consumption)?;
        None
    };
    let processing_elapsed_ms = elapsed_ms(processing_started);
    let counters_after = read_proc_counters()?;
    let memory = read_proc_memory()?;
    let cpu_ticks = counters_after
        .cpu_ticks
        .saturating_sub(counters_before.cpu_ticks);
    let cpu_ticks = u32::try_from(cpu_ticks).map_err(|_| "CPU tick delta exceeds u32")?;
    let cpu_time_ms = f64::from(cpu_ticks) * 1000.0 / f64::from(clock_ticks);
    let realtime_multiple = if config.seek {
        None
    } else {
        duration.map(|media| media.as_secs_f64() / processing_started.elapsed().as_secs_f64())
    };
    let result = BenchmarkResult {
        schema_version: 1,
        timestamp_unix_ms: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
        workload: config.workload,
        repetition: config.repetition,
        input_mode: match config.input {
            RunMode::Local(_) => "local",
            RunMode::Http(_) => "http",
        },
        source_duration_ms: duration.map(|value| value.as_millis()),
        codec: format!("{codec:?}"),
        load_elapsed_ms,
        first_output_elapsed_ms,
        processing_elapsed_ms,
        cpu_time_ms,
        realtime_multiple,
        seek_latency_ms,
        output_units: consumption.output_units,
        decoded_samples: consumption.decoded_samples,
        encoded_bytes: consumption.encoded_bytes,
        checksum: consumption.checksum,
        peak_rss_kib: memory.peak_rss_kib,
        current_rss_kib: memory.current_rss_kib,
        current_pss_kib: memory.current_pss_kib,
        threads: memory.threads,
        voluntary_context_switches: counters_after
            .voluntary_context_switches
            .saturating_sub(counters_before.voluntary_context_switches),
        involuntary_context_switches: counters_after
            .involuntary_context_switches
            .saturating_sub(counters_before.involuntary_context_switches),
    };
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

fn read_one(
    session: &mut MediaSession,
    output: &mut PlaybackOutput,
    consumption: &mut Consumption,
) -> Result<()> {
    if !output.read(session, consumption)? {
        return Err("media ended before its first output frame".into());
    }
    Ok(())
}

fn consume_remaining(
    session: &mut MediaSession,
    output: &mut PlaybackOutput,
    consumption: &mut Consumption,
) -> Result<()> {
    while output.read(session, consumption)? {
        // The complete-read benchmark intentionally runs without playback pacing.
    }
    Ok(())
}

fn measure_seeks(
    session: &mut MediaSession,
    output: &mut PlaybackOutput,
    consumption: &mut Consumption,
) -> Result<Summary> {
    let mut latencies = Vec::with_capacity(10);
    for target_ms in [
        10_000_u64, 40_000, 15_000, 45_000, 20_000, 50_000, 25_000, 35_000, 5_000, 30_000,
    ] {
        let started = Instant::now();
        let target = Duration::from_millis(target_ms);
        session.seek(target)?;
        output.reset();
        read_one(session, output, consumption)?;
        latencies.push(elapsed_ms(started));
    }
    Ok(summarize(&latencies))
}

fn record_frame(frame: &PcmFrame, consumption: &mut Consumption) {
    consumption.output_units = consumption.output_units.saturating_add(1);
    consumption.decoded_samples = consumption
        .decoded_samples
        .saturating_add(u64::try_from(frame.samples().len()).unwrap_or(u64::MAX));
    if let Some(first) = frame.samples().first() {
        consumption.checksum = consumption.checksum.rotate_left(5) ^ u64::from(first.to_bits());
    }
    if let Some(last) = frame.samples().last() {
        consumption.checksum = consumption.checksum.rotate_left(7) ^ u64::from(last.to_bits());
    }
}

fn record_packet(packet: &EncodedFrameSlot, consumption: &mut Consumption) {
    consumption.output_units = consumption.output_units.saturating_add(1);
    consumption.encoded_bytes = consumption
        .encoded_bytes
        .saturating_add(u64::try_from(packet.data().len()).unwrap_or(u64::MAX));
    if let Some(first) = packet.data().first() {
        consumption.checksum = consumption.checksum.rotate_left(5) ^ u64::from(*first);
    }
    if let Some(last) = packet.data().last() {
        consumption.checksum = consumption.checksum.rotate_left(7) ^ u64::from(*last);
    }
}

fn read_proc_counters() -> Result<ProcCounters> {
    let stat = fs::read_to_string("/proc/self/stat")?;
    let after_comm = stat
        .rsplit_once(") ")
        .ok_or("unexpected /proc/self/stat format")?
        .1;
    let fields = after_comm.split_whitespace().collect::<Vec<_>>();
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

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

fn summarize(values: &[f64]) -> Summary {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let percentile = |numerator: usize, denominator: usize| {
        let scaled = (sorted.len() - 1).saturating_mul(numerator);
        sorted[scaled.div_ceil(denominator)]
    };
    let count = u32::try_from(sorted.len()).expect("summary sample count fits u32");
    Summary {
        min: sorted[0],
        median: percentile(1, 2),
        p95: percentile(95, 100),
        max: sorted[sorted.len() - 1],
        mean: sorted.iter().sum::<f64>() / f64::from(count),
        samples: sorted.len(),
    }
}

fn serve(config: &ServeConfig) -> Result<()> {
    let root = config.root.canonicalize()?;
    let listener = TcpListener::bind(&config.address)?;
    eprintln!(
        "mantle-media-bench fixture server listening on {}",
        config.address
    );
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = serve_connection(stream, &root) {
                    eprintln!("fixture request failed: {error}");
                }
            }
            Err(error) => eprintln!("fixture accept failed: {error}"),
        }
    }
    Ok(())
}

fn serve_connection(mut stream: TcpStream, root: &Path) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut request = [0_u8; 16 * 1024];
    let used = read_request_headers(&mut stream, &mut request)?;
    let request = String::from_utf8_lossy(&request[..used]);
    let mut lines = request.lines();
    let first = lines.next().ok_or("empty HTTP request")?;
    let mut parts = first.split_whitespace();
    let method = parts.next().ok_or("missing HTTP method")?;
    let raw_path = parts.next().ok_or("missing HTTP path")?;
    if !matches!(method, "GET" | "HEAD") {
        write_status(&mut stream, "405 Method Not Allowed")?;
        return Ok(());
    }
    let relative = raw_path
        .split_once('?')
        .map_or(raw_path, |(path, _)| path)
        .trim_start_matches('/');
    if relative.is_empty()
        || !relative
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    {
        write_status(&mut stream, "400 Bad Request")?;
        return Ok(());
    }
    let path = root.join(relative);
    let Ok(mut file) = File::open(&path) else {
        write_status(&mut stream, "404 Not Found")?;
        return Ok(());
    };
    let length = file.metadata()?.len();
    let range = lines.find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("range").then_some(value.trim())
    });
    let (status, start, end_exclusive) = parse_range(range, length)?;
    let response_length = end_exclusive.saturating_sub(start);
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Length: {response_length}\r\nAccept-Ranges: bytes\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n"
    )?;
    if status.starts_with("206") {
        writeln!(
            stream,
            "Content-Range: bytes {start}-{}/{length}\r",
            end_exclusive - 1
        )?;
    }
    stream.write_all(b"\r\n")?;
    if method == "GET" {
        file.seek(SeekFrom::Start(start))?;
        io::copy(&mut file.take(response_length), &mut stream)?;
    }
    eprintln!(
        "fixture_response status={} range_start={start} range_end={}",
        status.split_whitespace().next().unwrap_or("unknown"),
        end_exclusive - 1
    );
    Ok(())
}

fn read_request_headers(stream: &mut TcpStream, buffer: &mut [u8]) -> Result<usize> {
    let mut used = 0_usize;
    while used < buffer.len() {
        let count = stream.read(&mut buffer[used..])?;
        if count == 0 {
            break;
        }
        used += count;
        if buffer[..used]
            .windows(4)
            .any(|window| window == b"\r\n\r\n")
        {
            return Ok(used);
        }
    }
    Err("HTTP request headers exceeded their limit or ended early".into())
}

fn parse_range(range: Option<&str>, length: u64) -> Result<(&'static str, u64, u64)> {
    let Some(range) = range else {
        return Ok(("200 OK", 0, length));
    };
    let value = range
        .strip_prefix("bytes=")
        .ok_or("unsupported HTTP range unit")?;
    let (start, end) = value.split_once('-').ok_or("malformed HTTP range")?;
    let start: u64 = start.parse()?;
    let end_exclusive = end.parse::<u64>()?.saturating_add(1).min(length);
    if start >= end_exclusive || start >= length {
        return Err("unsatisfiable HTTP range".into());
    }
    Ok(("206 Partial Content", start, end_exclusive))
}

fn write_status(stream: &mut TcpStream, status: &str) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_with_nearest_rank_percentiles() {
        let result = summarize(&[5.0, 1.0, 3.0, 2.0, 4.0]);
        assert!((result.min - 1.0).abs() < f64::EPSILON);
        assert!((result.median - 3.0).abs() < f64::EPSILON);
        assert!((result.p95 - 5.0).abs() < f64::EPSILON);
        assert!((result.max - 5.0).abs() < f64::EPSILON);
        assert!((result.mean - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn parses_closed_bounded_ranges() {
        assert_eq!(parse_range(None, 100).unwrap(), ("200 OK", 0, 100));
        assert_eq!(
            parse_range(Some("bytes=10-19"), 100).unwrap(),
            ("206 Partial Content", 10, 20)
        );
        assert!(parse_range(Some("bytes=10-"), 100).is_err());
        assert!(parse_range(Some("bytes=100-101"), 100).is_err());
    }

    #[test]
    fn parses_run_modes_without_retaining_input_in_results() {
        let args = [
            "--workload".to_owned(),
            "mp3-decode-http".to_owned(),
            "--input".to_owned(),
            "http://127.0.0.1/media?secret=value".to_owned(),
            "--http".to_owned(),
            "--seek".to_owned(),
        ];
        let config = parse_run_config(&args).unwrap();
        assert!(matches!(config.input, RunMode::Http(_)));
        assert!(config.seek);
    }

    #[test]
    fn accepts_only_bounded_soak_checkpoint_schedules() {
        let full = [
            "--fixture-root".to_owned(),
            "/fixtures".to_owned(),
            "--duration-seconds".to_owned(),
            "86400".to_owned(),
            "--checkpoint-seconds".to_owned(),
            "60".to_owned(),
            "--cycle-delay-ms".to_owned(),
            "250".to_owned(),
        ];
        let config = parse_soak_config(&full).unwrap();
        assert_eq!(config.duration_seconds, MAX_SOAK_DURATION_SECONDS);
        assert_eq!(config.checkpoint_seconds, 60);
        assert_eq!(config.cycle_delay_ms, 250);

        let too_long = full.clone().map(|value| {
            if value == "86400" {
                "86401".to_owned()
            } else {
                value
            }
        });
        assert!(parse_soak_config(&too_long).is_err());

        let too_many_checkpoints = full.map(|value| {
            if value == "60" {
                "59".to_owned()
            } else {
                value
            }
        });
        assert!(parse_soak_config(&too_many_checkpoints).is_err());
    }

    #[test]
    fn summarizes_soak_memory_from_fixed_edge_windows() {
        let samples = (0_u64..16)
            .map(|index| ProcMemory {
                peak_rss_kib: 400 + index,
                current_rss_kib: if index < 8 { 100 + index } else { 192 + index },
                current_pss_kib: if index < 8 { 80 + index } else { 122 + index },
                threads: 1 + u64::from(index == 10),
            })
            .collect::<Vec<_>>();
        let summary = summarize_soak_memory(&samples).unwrap();
        assert_eq!(summary.first_window_samples, SOAK_MEMORY_WINDOW);
        assert_eq!(summary.last_window_samples, SOAK_MEMORY_WINDOW);
        assert_eq!(summary.first_rss_median_kib, 104);
        assert_eq!(summary.last_rss_median_kib, 204);
        assert_eq!(summary.rss_growth_kib, 100);
        assert_eq!(summary.first_pss_median_kib, 84);
        assert_eq!(summary.last_pss_median_kib, 134);
        assert_eq!(summary.pss_growth_kib, 50);
        assert_eq!(summary.peak_rss_kib, 415);
        assert_eq!(summary.max_threads, 2);
    }
}
