use std::env;
use std::error::Error;
use std::hint::black_box;
use std::mem;
use std::path::Path;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use mantle_audio::{
    COMPATIBLE_PCM_SAMPLES, COMPATIBLE_SAMPLE_RATE, EncodedFrameConsumer, EncodedFrameProducer,
    EncodedFrameSlot, EqualizerFactory, FilterPipeline, MAX_ENCODED_FRAME_QUEUE_CAPACITY,
    OpusEncodingQuality, OpusPassthrough, PcmFormat, PcmFrame, PcmOpusEncoder, VolumeLevel,
    encoded_frame_queue,
};
use mantle_media::{
    Codec, EncodedPacket, HttpNetworkAccess, HttpRangeInput, HttpRangeOptions, MediaLimits,
    MediaSession,
};
use serde::Serialize;

use crate::oracle::{DeliveryCounts, DeliveryOracle};
use crate::process::{ProcCounters, ProcSample};

mod oracle;
mod process;

const FRAME_PERIOD: Duration = Duration::from_millis(20);
const SAMPLE_PERIOD: Duration = Duration::from_millis(250);
const FIRST_FRAME_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_TRACKS: usize = 250;
const MAX_WORKERS: usize = 256;
const MAX_INTERVAL_SECONDS: u64 = 60;
const TRACKS_PER_SHARED_WORKER: usize = 25;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Architecture {
    Dedicated,
    SharedPool,
    Hybrid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Workload {
    Synthetic,
    OpusPassthroughLocal,
    Mp3DecodeLocal,
    AacDecodeLocal,
    Mp3EqualizerLocal,
    Mp3DecodeHttp,
    FlacDecodeLocal,
}

#[derive(Debug)]
struct Config {
    architecture: Architecture,
    workload: Workload,
    input: Option<String>,
    tracks: usize,
    workers: usize,
    queue_capacity: usize,
    warmup_seconds: u64,
    measure_seconds: u64,
    repetition: usize,
    synthetic_work: u32,
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

impl Summary {
    const EMPTY: Self = Self {
        min: 0.0,
        median: 0.0,
        p95: 0.0,
        max: 0.0,
        mean: 0.0,
        samples: 0,
    };
}

#[derive(Debug, Serialize)]
struct BenchmarkResult {
    schema_version: u32,
    timestamp_unix_ms: u128,
    architecture: Architecture,
    workload: Workload,
    repetition: usize,
    tracks: usize,
    worker_threads: usize,
    queue_capacity: usize,
    warmup_seconds: u64,
    measure_seconds: u64,
    startup_latency_ms: f64,
    load_latency_ms: Summary,
    cold_load_latency_ms: f64,
    warm_load_latency_ms: Summary,
    first_frame_latency_ms: Summary,
    cpu_core_percent: f64,
    rss_kib: Summary,
    pss_kib: Summary,
    threads: Summary,
    voluntary_context_switches: u64,
    involuntary_context_switches: u64,
    frames_requested: u64,
    frames_delivered: u64,
    frame_underruns: u64,
    skipped_deadlines: u64,
    queue_depth: Summary,
    timestamp_regressions: u64,
    timestamp_discontinuities: u64,
    consumed_frames_per_track: Summary,
    consumed_bytes: u64,
    checksum: u64,
}

enum TrackSource {
    Synthetic {
        track: u64,
        sequence: u64,
        work: u32,
    },
    Opus(Box<OpusTrackSource>),
    Pcm(Box<PcmTrackSource>),
}

struct OpusTrackSource {
    session: MediaSession,
    packet: EncodedPacket,
    router: OpusPassthrough,
}

struct PcmTrackSource {
    session: MediaSession,
    decoded: PcmFrame,
    decoded_offset: usize,
    assembled: [f32; COMPATIBLE_PCM_SAMPLES],
    output: PcmFrame,
    filter: Option<FilterPipeline>,
    encoder: PcmOpusEncoder,
    sequence: u64,
}

impl TrackSource {
    fn synthetic(track: usize, work: u32) -> Result<Self> {
        Ok(Self::Synthetic {
            track: u64::try_from(track)?,
            sequence: 0,
            work,
        })
    }

    fn opus(path: &Path) -> Result<Self> {
        let session = MediaSession::open_file(path, MediaLimits::default())?;
        if session.info().codec != Codec::Opus {
            return Err(format!("{} is not an Opus source", path.display()).into());
        }
        let format = PcmFormat::new(session.info().sample_rate, session.info().channels)?;
        let packet = EncodedPacket::with_capacity(session.limits().max_packet_bytes);
        Ok(Self::Opus(Box::new(OpusTrackSource {
            session,
            packet,
            router: OpusPassthrough::new(format),
        })))
    }

    fn pcm(path: &Path, expected_codec: Codec, equalizer: bool) -> Result<Self> {
        let limits = MediaLimits {
            max_pcm_samples_per_frame: 16 * 1024,
            ..MediaLimits::default()
        };
        let session = MediaSession::open_file(path, limits)?;
        Self::pcm_session(
            session,
            expected_codec,
            equalizer,
            &path.display().to_string(),
        )
    }

    fn pcm_http(url: &str) -> Result<Self> {
        let limits = MediaLimits {
            max_pcm_samples_per_frame: 16 * 1024,
            ..MediaLimits::default()
        };
        let options = HttpRangeOptions {
            network_access: HttpNetworkAccess::AllowPrivateNetworks,
            ..HttpRangeOptions::default()
        };
        let input = HttpRangeInput::open(url, options)?;
        let session = MediaSession::open(Box::new(input), Some("mp3"), limits)?;
        Self::pcm_session(session, Codec::Mp3, false, "loopback HTTP input")
    }

    fn pcm_session(
        session: MediaSession,
        expected_codec: Codec,
        equalizer: bool,
        input_description: &str,
    ) -> Result<Self> {
        if session.info().codec != expected_codec {
            return Err(format!(
                "{input_description} has codec {:?}; workload requires {expected_codec:?}",
                session.info().codec
            )
            .into());
        }
        let format = PcmFormat::new(session.info().sample_rate, session.info().channels)?;
        if format.sample_rate() != COMPATIBLE_SAMPLE_RATE || format.channels() != 2 {
            return Err("decoded benchmark input must be 48 kHz stereo".into());
        }
        let filter = if equalizer {
            let mut pipeline = FilterPipeline::new(format, 1)?;
            pipeline.install_factory(Some(&EqualizerFactory::new()))?;
            Some(pipeline)
        } else {
            None
        };
        Ok(Self::Pcm(Box::new(PcmTrackSource {
            decoded: PcmFrame::with_capacity(session.limits().max_pcm_samples_per_frame),
            decoded_offset: 0,
            assembled: [0.0; COMPATIBLE_PCM_SAMPLES],
            output: PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES),
            filter,
            encoder: PcmOpusEncoder::new(OpusEncodingQuality::MAXIMUM)?,
            session,
            sequence: 0,
        })))
    }

    fn produce(&mut self, output: &mut EncodedFrameSlot) -> Result<()> {
        match self {
            Self::Synthetic {
                track,
                sequence,
                work,
            } => {
                let mut state = track.rotate_left(17) ^ *sequence ^ 0x9e37_79b9_7f4a_7c15;
                for _ in 0..*work {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                }
                black_box(state);
                let mut bytes = [0_u8; 24];
                bytes[..8].copy_from_slice(&track.to_le_bytes());
                bytes[8..16].copy_from_slice(&sequence.to_le_bytes());
                bytes[16..].copy_from_slice(&state.to_le_bytes());
                output.write(
                    &bytes,
                    Some(FRAME_PERIOD.saturating_mul(u32::try_from(*sequence)?)),
                    VolumeLevel::NORMAL,
                )?;
                *sequence = sequence.saturating_add(1);
            }
            Self::Opus(source) => {
                if !source.session.read_encoded(&mut source.packet)? {
                    return Err(
                        "real workload reached EOF during the bounded benchmark interval".into(),
                    );
                }
                if !source
                    .router
                    .route_packet(source.packet.data(), source.packet.timestamp(), output)?
                    .delivered()
                {
                    return Err("real Opus workload unexpectedly required transcoding".into());
                }
            }
            Self::Pcm(source) => source.produce(output)?,
        }
        Ok(())
    }
}

impl PcmTrackSource {
    fn produce(&mut self, encoded: &mut EncodedFrameSlot) -> Result<()> {
        let mut filled = 0_usize;
        while filled < self.assembled.len() {
            if self.decoded_offset == self.decoded.samples().len() {
                if !self.session.read_pcm(&mut self.decoded)? {
                    return Err(
                        "decoded workload reached EOF during the bounded benchmark interval".into(),
                    );
                }
                self.decoded_offset = 0;
                if self.decoded.sample_rate() != COMPATIBLE_SAMPLE_RATE
                    || self.decoded.channels() != 2
                {
                    return Err("decoder changed format during the benchmark".into());
                }
            }
            let available = &self.decoded.samples()[self.decoded_offset..];
            let count = available.len().min(self.assembled.len() - filled);
            self.assembled[filled..filled + count].copy_from_slice(&available[..count]);
            filled += count;
            self.decoded_offset += count;
        }
        let timestamp = FRAME_PERIOD.saturating_mul(u32::try_from(self.sequence)?);
        let format = PcmFormat::new(COMPATIBLE_SAMPLE_RATE, 2)?;
        self.output
            .copy_from_interleaved(&self.assembled, format, Some(timestamp))?;
        if let Some(filter) = &mut self.filter {
            filter.process(&mut self.output)?;
        }
        self.encoder
            .encode(&self.output, encoded, VolumeLevel::NORMAL)?;
        self.sequence = self.sequence.saturating_add(1);
        Ok(())
    }
}

struct TrackWorker {
    source: TrackSource,
    producer: EncodedFrameProducer,
    slot: EncodedFrameSlot,
}

impl TrackWorker {
    fn step(&mut self) -> Result<bool> {
        if self.producer.is_full() {
            return Ok(false);
        }
        self.source.produce(&mut self.slot)?;
        match self.producer.try_push(mem::take(&mut self.slot)) {
            Ok(()) => Ok(true),
            Err(full) => {
                self.slot = full.into_frame();
                Ok(false)
            }
        }
    }
}

struct ConsumerTrack {
    consumer: EncodedFrameConsumer,
    slot: EncodedFrameSlot,
    last_timestamp: Option<Duration>,
    timestamp_regressions: u64,
    timestamp_discontinuities: u64,
    consumed_frames: u64,
    consumed_bytes: u64,
    checksum: u64,
}

impl ConsumerTrack {
    fn consume(&mut self) -> bool {
        if !self.consumer.try_pop_into(&mut self.slot) {
            return false;
        }
        if let Some(timestamp) = self.slot.timestamp() {
            if let Some(previous) = self.last_timestamp {
                if timestamp <= previous {
                    self.timestamp_regressions = self.timestamp_regressions.saturating_add(1);
                }
                if timestamp != previous + FRAME_PERIOD {
                    self.timestamp_discontinuities =
                        self.timestamp_discontinuities.saturating_add(1);
                }
            }
            self.last_timestamp = Some(timestamp);
        }
        self.consumed_frames = self.consumed_frames.saturating_add(1);
        self.consumed_bytes = self
            .consumed_bytes
            .saturating_add(u64::try_from(self.slot.data().len()).unwrap_or(u64::MAX));
        if let Some(first) = self.slot.data().first() {
            self.checksum = self.checksum.rotate_left(5) ^ u64::from(*first);
        }
        if let Some(last) = self.slot.data().last() {
            self.checksum = self.checksum.rotate_left(7) ^ u64::from(*last);
        }
        true
    }
}

struct WorkerControl {
    started: AtomicBool,
    stop: AtomicBool,
    failed: AtomicBool,
    error: Mutex<Option<String>>,
}

impl WorkerControl {
    fn new() -> Self {
        Self {
            started: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            failed: AtomicBool::new(false),
            error: Mutex::new(None),
        }
    }

    fn record_error(&self, error: &dyn Error) {
        let mut stored = self
            .error
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if stored.is_none() {
            *stored = Some(error.to_string());
        }
        self.failed.store(true, Ordering::Release);
        self.stop.store(true, Ordering::Release);
    }

    fn check(&self) -> Result<()> {
        if !self.failed.load(Ordering::Acquire) {
            return Ok(());
        }
        let stored = self
            .error
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Err(stored
            .as_deref()
            .unwrap_or("media worker failed without a diagnostic")
            .to_owned()
            .into())
    }
}

struct Scheduler {
    control: Arc<WorkerControl>,
    joins: Vec<JoinHandle<()>>,
    worker_threads: Vec<thread::Thread>,
}

impl Scheduler {
    fn start(&self) {
        self.control.started.store(true, Ordering::Release);
        self.wake_workers();
    }

    fn check(&self) -> Result<()> {
        self.control.check()
    }

    fn wake_workers(&self) {
        for worker in &self.worker_threads {
            worker.unpark();
        }
    }

    fn shutdown(self) -> Result<()> {
        self.control.stop.store(true, Ordering::Release);
        self.wake_workers();
        for join in self.joins {
            join.join().map_err(|_| "media worker panicked")?;
        }
        self.control.check()
    }
}

struct DepthHistogram {
    counts: Vec<u64>,
    samples: u64,
    sum: u64,
}

impl DepthHistogram {
    fn new(capacity: usize) -> Self {
        Self {
            counts: vec![0; capacity.saturating_add(1)],
            samples: 0,
            sum: 0,
        }
    }

    fn record(&mut self, depth: usize) {
        let index = depth.min(self.counts.len().saturating_sub(1));
        self.counts[index] = self.counts[index].saturating_add(1);
        self.samples = self.samples.saturating_add(1);
        self.sum = self
            .sum
            .saturating_add(u64::try_from(index).unwrap_or(u64::MAX));
    }

    fn summary(&self) -> Summary {
        if self.samples == 0 {
            return Summary::EMPTY;
        }
        let percentile = |numerator: u64, denominator: u64| {
            let rank = self.samples.saturating_mul(numerator).div_ceil(denominator);
            let mut seen = 0_u64;
            self.counts
                .iter()
                .enumerate()
                .find_map(|(depth, count)| {
                    seen = seen.saturating_add(*count);
                    (seen >= rank).then_some(depth)
                })
                .unwrap_or(0)
        };
        let min = self.counts.iter().position(|count| *count > 0).unwrap_or(0);
        let max = self
            .counts
            .iter()
            .rposition(|count| *count > 0)
            .unwrap_or(0);
        Summary {
            min: usize_to_f64(min),
            median: usize_to_f64(percentile(50, 100)),
            p95: usize_to_f64(percentile(95, 100)),
            max: usize_to_f64(max),
            mean: u64_to_f64(self.sum) / u64_to_f64(self.samples),
            samples: usize::try_from(self.samples).unwrap_or(usize::MAX),
        }
    }
}

struct PacedResult {
    counts: DeliveryCounts,
    samples: Vec<ProcSample>,
    queue_depth: Summary,
}

fn main() -> ExitCode {
    let process_started = Instant::now();
    match run_main(process_started) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("mantle-worker-bench: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run_main(process_started: Instant) -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let config = parse_config(&args)?;
    run_benchmark(&config, process_started)
}

fn parse_config(args: &[String]) -> Result<Config> {
    let architecture = match required_value(args, "--architecture")?.as_str() {
        "dedicated" => Architecture::Dedicated,
        "shared-pool" => Architecture::SharedPool,
        "hybrid" => Architecture::Hybrid,
        value => return Err(format!("unsupported architecture {value:?}").into()),
    };
    let workload = match required_value(args, "--workload")?.as_str() {
        "synthetic" => Workload::Synthetic,
        "opus-passthrough-local" => Workload::OpusPassthroughLocal,
        "mp3-decode-local" => Workload::Mp3DecodeLocal,
        "aac-decode-local" => Workload::AacDecodeLocal,
        "mp3-equalizer-local" => Workload::Mp3EqualizerLocal,
        "mp3-decode-http" => Workload::Mp3DecodeHttp,
        "flac-decode-local" => Workload::FlacDecodeLocal,
        value => return Err(format!("unsupported workload {value:?}").into()),
    };
    let tracks = bounded_usize(args, "--tracks", 1, MAX_TRACKS, None)?;
    let default_workers = thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .min(tracks);
    let workers = bounded_usize(args, "--workers", 1, MAX_WORKERS, Some(default_workers))?;
    let queue_capacity = bounded_usize(
        args,
        "--queue-capacity",
        1,
        MAX_ENCODED_FRAME_QUEUE_CAPACITY,
        Some(50),
    )?;
    let warmup_seconds = bounded_u64(args, "--warmup-seconds", 0, MAX_INTERVAL_SECONDS, Some(3))?;
    let measure_seconds = bounded_u64(args, "--measure-seconds", 1, MAX_INTERVAL_SECONDS, Some(8))?;
    let repetition = bounded_usize(args, "--repetition", 1, usize::MAX, Some(1))?;
    let synthetic_work = u32::try_from(bounded_u64(
        args,
        "--synthetic-work",
        0,
        1_000_000,
        Some(2_000),
    )?)?;
    let input = value(args, "--input");
    if workload != Workload::Synthetic && input.is_none() {
        return Err("--input is required for real workloads".into());
    }
    Ok(Config {
        architecture,
        workload,
        input,
        tracks,
        workers,
        queue_capacity,
        warmup_seconds,
        measure_seconds,
        repetition,
        synthetic_work,
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

fn bounded_usize(
    args: &[String],
    name: &str,
    minimum: usize,
    maximum: usize,
    default: Option<usize>,
) -> Result<usize> {
    let parsed = match value(args, name) {
        Some(raw) => raw.parse()?,
        None => default.ok_or_else(|| format!("missing required option {name}"))?,
    };
    if !(minimum..=maximum).contains(&parsed) {
        return Err(format!("{name} must be between {minimum} and {maximum}").into());
    }
    Ok(parsed)
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

#[allow(
    clippy::too_many_lines,
    reason = "keeping setup, paced measurement, and teardown linear makes metric boundaries explicit"
)]
fn run_benchmark(config: &Config, process_started: Instant) -> Result<()> {
    let startup_latency_ms = elapsed_ms(process_started);
    let mut load_latencies = Vec::with_capacity(config.tracks);
    let mut workers = Vec::with_capacity(config.tracks);
    let mut consumers = Vec::with_capacity(config.tracks);
    for track in 0..config.tracks {
        let load_started = Instant::now();
        let source = match config.workload {
            Workload::Synthetic => TrackSource::synthetic(track, config.synthetic_work)?,
            Workload::OpusPassthroughLocal => TrackSource::opus(Path::new(
                config
                    .input
                    .as_deref()
                    .ok_or("real workload is missing its input")?,
            ))?,
            Workload::Mp3DecodeLocal => TrackSource::pcm(
                Path::new(
                    config
                        .input
                        .as_deref()
                        .ok_or("real workload is missing its input")?,
                ),
                Codec::Mp3,
                false,
            )?,
            Workload::AacDecodeLocal => TrackSource::pcm(
                Path::new(
                    config
                        .input
                        .as_deref()
                        .ok_or("real workload is missing its input")?,
                ),
                Codec::AacLc,
                false,
            )?,
            Workload::Mp3EqualizerLocal => TrackSource::pcm(
                Path::new(
                    config
                        .input
                        .as_deref()
                        .ok_or("real workload is missing its input")?,
                ),
                Codec::Mp3,
                true,
            )?,
            Workload::Mp3DecodeHttp => TrackSource::pcm_http(
                config
                    .input
                    .as_deref()
                    .ok_or("real workload is missing its input")?,
            )?,
            Workload::FlacDecodeLocal => TrackSource::pcm(
                Path::new(
                    config
                        .input
                        .as_deref()
                        .ok_or("real workload is missing its input")?,
                ),
                Codec::Flac,
                false,
            )?,
        };
        let (producer, consumer) = encoded_frame_queue(config.queue_capacity)?;
        workers.push(TrackWorker {
            source,
            producer,
            slot: EncodedFrameSlot::new(),
        });
        consumers.push(ConsumerTrack {
            consumer,
            slot: EncodedFrameSlot::new(),
            last_timestamp: None,
            timestamp_regressions: 0,
            timestamp_discontinuities: 0,
            consumed_frames: 0,
            consumed_bytes: 0,
            checksum: 0,
        });
        load_latencies.push(elapsed_ms(load_started));
    }

    let bounded_worker_threads = shared_worker_count(config.tracks, config.workers);
    let worker_threads = match config.architecture {
        Architecture::Dedicated => config.tracks,
        Architecture::SharedPool | Architecture::Hybrid => bounded_worker_threads,
    };
    let scheduler = launch_scheduler(config.architecture, worker_threads, workers);
    let playback_started = Instant::now();
    scheduler.start();
    let first_frame_latencies =
        wait_for_first_frames(&mut consumers, &scheduler, playback_started)?;
    wait_for_prefill(&consumers, &scheduler, config.queue_capacity)?;
    let warmed = consume_paced(
        &mut consumers,
        &scheduler,
        Duration::from_secs(config.warmup_seconds),
        false,
        config.queue_capacity,
    )?;
    if warmed.counts.frame_underruns != 0 || warmed.counts.skipped_deadlines != 0 {
        return Err("warm-up had an underrun or skipped deadline".into());
    }

    let clock_ticks = process::clock_ticks_per_second()?;
    let counters_before = process::read_proc_counters()?;
    let measure_started = Instant::now();
    let measured = consume_paced(
        &mut consumers,
        &scheduler,
        Duration::from_secs(config.measure_seconds),
        true,
        config.queue_capacity,
    )?;
    let measured_elapsed = measure_started.elapsed();
    let counters_after = process::read_proc_counters()?;
    scheduler.shutdown()?;

    let cpu_ticks = counters_after
        .cpu_ticks
        .saturating_sub(counters_before.cpu_ticks);
    let cpu_seconds = u64_to_f64(cpu_ticks) / f64::from(clock_ticks);
    let cpu_core_percent = cpu_seconds / measured_elapsed.as_secs_f64() * 100.0;
    let counts = measured.counts;
    let timestamp_regressions = consumers
        .iter()
        .map(|track| track.timestamp_regressions)
        .sum();
    let timestamp_discontinuities = consumers
        .iter()
        .map(|track| track.timestamp_discontinuities)
        .sum();
    let consumed_frames_per_track = summarize(
        &consumers
            .iter()
            .map(|track| u64_to_f64(track.consumed_frames))
            .collect::<Vec<_>>(),
    );
    let consumed_bytes = consumers.iter().map(|track| track.consumed_bytes).sum();
    let checksum = consumers.iter().fold(0_u64, |checksum, track| {
        checksum.rotate_left(3) ^ track.checksum
    });
    let result = BenchmarkResult {
        schema_version: 1,
        timestamp_unix_ms: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
        architecture: config.architecture,
        workload: config.workload,
        repetition: config.repetition,
        tracks: config.tracks,
        worker_threads,
        queue_capacity: config.queue_capacity,
        warmup_seconds: config.warmup_seconds,
        measure_seconds: config.measure_seconds,
        startup_latency_ms,
        load_latency_ms: summarize(&load_latencies),
        cold_load_latency_ms: load_latencies[0],
        warm_load_latency_ms: summarize(&load_latencies[1..]),
        first_frame_latency_ms: summarize(&first_frame_latencies),
        cpu_core_percent,
        rss_kib: summarize_proc(&measured.samples, |sample| sample.rss_kib),
        pss_kib: summarize_proc(&measured.samples, |sample| sample.pss_kib),
        threads: summarize_proc(&measured.samples, |sample| sample.threads),
        voluntary_context_switches: counter_delta(counters_before, counters_after, |counters| {
            counters.voluntary_context_switches
        }),
        involuntary_context_switches: counter_delta(counters_before, counters_after, |counters| {
            counters.involuntary_context_switches
        }),
        frames_requested: counts.frames_requested,
        frames_delivered: counts.frames_delivered,
        frame_underruns: counts.frame_underruns,
        skipped_deadlines: counts.skipped_deadlines,
        queue_depth: measured.queue_depth,
        timestamp_regressions,
        timestamp_discontinuities,
        consumed_frames_per_track,
        consumed_bytes,
        checksum,
    };
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

fn shared_worker_count(tracks: usize, worker_limit: usize) -> usize {
    tracks.div_ceil(TRACKS_PER_SHARED_WORKER).min(worker_limit)
}

fn launch_scheduler(
    architecture: Architecture,
    worker_threads: usize,
    tracks: Vec<TrackWorker>,
) -> Scheduler {
    let control = Arc::new(WorkerControl::new());
    let joins = match architecture {
        Architecture::Dedicated => tracks
            .into_iter()
            .map(|track| spawn_owned_worker(vec![track], Arc::clone(&control)))
            .collect(),
        Architecture::SharedPool => launch_shared_pool(tracks, worker_threads, &control),
        Architecture::Hybrid => launch_hybrid(tracks, worker_threads, &control),
    };
    let worker_threads = joins.iter().map(|join| join.thread().clone()).collect();
    Scheduler {
        control,
        joins,
        worker_threads,
    }
}

fn spawn_owned_worker(mut tracks: Vec<TrackWorker>, control: Arc<WorkerControl>) -> JoinHandle<()> {
    thread::spawn(move || {
        wait_for_start(&control);
        while !control.stop.load(Ordering::Acquire) {
            let mut progressed = false;
            for track in &mut tracks {
                match track.step() {
                    Ok(produced) => progressed |= produced,
                    Err(error) => {
                        control.record_error(error.as_ref());
                        return;
                    }
                }
            }
            if !progressed {
                thread::park();
            }
        }
    })
}

fn launch_hybrid(
    tracks: Vec<TrackWorker>,
    worker_threads: usize,
    control: &Arc<WorkerControl>,
) -> Vec<JoinHandle<()>> {
    let mut shards = (0..worker_threads).map(|_| Vec::new()).collect::<Vec<_>>();
    for (index, track) in tracks.into_iter().enumerate() {
        shards[index % worker_threads].push(track);
    }
    shards
        .into_iter()
        .map(|shard| spawn_owned_worker(shard, Arc::clone(control)))
        .collect()
}

fn launch_shared_pool(
    tracks: Vec<TrackWorker>,
    worker_threads: usize,
    control: &Arc<WorkerControl>,
) -> Vec<JoinHandle<()>> {
    let track_count = tracks.len();
    let tracks = Arc::new(tracks.into_iter().map(Mutex::new).collect::<Vec<_>>());
    let cursor = Arc::new(AtomicUsize::new(0));
    let mut joins = Vec::with_capacity(worker_threads);
    for _ in 0..worker_threads {
        let tracks = Arc::clone(&tracks);
        let cursor = Arc::clone(&cursor);
        let control = Arc::clone(control);
        joins.push(thread::spawn(move || {
            wait_for_start(&control);
            while !control.stop.load(Ordering::Acquire) {
                let mut progressed = false;
                for _ in 0..track_count {
                    let index = cursor.fetch_add(1, Ordering::Relaxed) % track_count;
                    let Ok(mut track) = tracks[index].try_lock() else {
                        continue;
                    };
                    match track.step() {
                        Ok(produced) => progressed |= produced,
                        Err(error) => {
                            control.record_error(error.as_ref());
                            return;
                        }
                    }
                }
                if !progressed {
                    thread::park();
                }
            }
        }));
    }
    joins
}

fn wait_for_start(control: &WorkerControl) {
    while !control.started.load(Ordering::Acquire) && !control.stop.load(Ordering::Acquire) {
        thread::park_timeout(Duration::from_millis(1));
    }
}

fn wait_for_first_frames(
    consumers: &mut [ConsumerTrack],
    scheduler: &Scheduler,
    started: Instant,
) -> Result<Vec<f64>> {
    let deadline = started + FIRST_FRAME_TIMEOUT;
    let mut latencies = vec![None; consumers.len()];
    while latencies.iter().any(Option::is_none) {
        scheduler.check()?;
        for (index, consumer) in consumers.iter_mut().enumerate() {
            if latencies[index].is_none() && consumer.consume() {
                latencies[index] = Some(elapsed_ms(started));
            }
        }
        scheduler.wake_workers();
        if Instant::now() >= deadline {
            return Err(
                "one or more tracks did not produce a first frame within 15 seconds".into(),
            );
        }
        thread::sleep(Duration::from_micros(100));
    }
    Ok(latencies.into_iter().flatten().collect())
}

fn wait_for_prefill(
    consumers: &[ConsumerTrack],
    scheduler: &Scheduler,
    target_depth: usize,
) -> Result<()> {
    let deadline = Instant::now() + FIRST_FRAME_TIMEOUT;
    while consumers
        .iter()
        .any(|consumer| consumer.consumer.available() < target_depth)
    {
        scheduler.check()?;
        if Instant::now() >= deadline {
            return Err("one or more tracks did not prefill their bounded queue".into());
        }
        thread::sleep(Duration::from_micros(100));
    }
    Ok(())
}

fn consume_paced(
    consumers: &mut [ConsumerTrack],
    scheduler: &Scheduler,
    duration: Duration,
    collect: bool,
    queue_capacity: usize,
) -> Result<PacedResult> {
    let start = Instant::now();
    let end = start + duration;
    let mut next_frame = start;
    let mut next_sample = start;
    let mut oracle = DeliveryOracle::new(consumers.len());
    let mut samples = Vec::with_capacity(
        usize::try_from(duration.as_millis() / SAMPLE_PERIOD.as_millis())
            .unwrap_or(0)
            .saturating_add(1),
    );
    let mut depths = DepthHistogram::new(queue_capacity);
    let ticks = duration.as_nanos() / FRAME_PERIOD.as_nanos();
    if !duration.as_nanos().is_multiple_of(FRAME_PERIOD.as_nanos()) {
        return Err("paced duration must contain a whole number of 20 ms frames".into());
    }

    for _ in 0..ticks {
        scheduler.check()?;
        let current = Instant::now();
        if next_frame > current {
            thread::sleep(next_frame - current);
        }
        let now = Instant::now();
        oracle.observe_lateness(now.saturating_duration_since(next_frame), FRAME_PERIOD);
        oracle.observe_tick(consumers.iter_mut().map(|consumer| {
            let delivered = consumer.consume();
            if collect {
                depths.record(consumer.consumer.available());
            }
            delivered
        }))?;
        scheduler.wake_workers();
        next_frame += FRAME_PERIOD;
        if collect && now >= next_sample {
            samples.push(process::read_proc_sample()?);
            next_sample += SAMPLE_PERIOD;
        }
    }
    let current = Instant::now();
    if end > current {
        thread::sleep(end - current);
    }
    scheduler.check()?;
    if collect && samples.is_empty() {
        samples.push(process::read_proc_sample()?);
    }
    Ok(PacedResult {
        counts: oracle.counts(),
        samples,
        queue_depth: depths.summary(),
    })
}

fn counter_delta(
    before: ProcCounters,
    after: ProcCounters,
    field: impl Fn(ProcCounters) -> u64,
) -> u64 {
    field(after).saturating_sub(field(before))
}

fn summarize_proc(samples: &[ProcSample], field: impl Fn(&ProcSample) -> u64) -> Summary {
    summarize(
        &samples
            .iter()
            .map(|sample| u64_to_f64(field(sample)))
            .collect::<Vec<_>>(),
    )
}

fn summarize(values: &[f64]) -> Summary {
    if values.is_empty() {
        return Summary::EMPTY;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let percentile = |numerator: usize, denominator: usize| {
        let rank = sorted.len().saturating_mul(numerator).div_ceil(denominator);
        sorted[rank.saturating_sub(1)]
    };
    Summary {
        min: sorted[0],
        median: percentile(50, 100),
        p95: percentile(95, 100),
        max: sorted[sorted.len() - 1],
        mean: sorted.iter().sum::<f64>() / usize_to_f64(sorted.len()),
        samples: sorted.len(),
    }
}

fn usize_to_f64(value: usize) -> f64 {
    u64::try_from(value).map_or(u64_to_f64(u64::MAX), u64_to_f64)
}

fn u64_to_f64(value: u64) -> f64 {
    let high = u32::try_from(value >> 32).unwrap_or(u32::MAX);
    let low = u32::try_from(value & u64::from(u32::MAX)).unwrap_or(u32::MAX);
    f64::from(high).mul_add(4_294_967_296.0, f64::from(low))
}

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1_000.0
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use mantle_audio::EncodedFrameSlot;
    use mantle_media::Codec;

    use super::{
        Architecture, TrackSource, Workload, parse_config, shared_worker_count, summarize,
    };

    #[test]
    fn parses_bounded_synthetic_configuration() {
        let args = [
            "--architecture".to_owned(),
            "hybrid".to_owned(),
            "--workload".to_owned(),
            "synthetic".to_owned(),
            "--tracks".to_owned(),
            "10".to_owned(),
            "--workers".to_owned(),
            "3".to_owned(),
        ];
        let config = parse_config(&args).unwrap();
        assert_eq!(config.architecture, Architecture::Hybrid);
        assert_eq!(config.workload, Workload::Synthetic);
        assert_eq!(config.tracks, 10);
        assert_eq!(config.workers, 3);
    }

    #[test]
    fn shared_worker_density_is_bounded_by_track_count_and_limit() {
        assert_eq!(shared_worker_count(1, 28), 1);
        assert_eq!(shared_worker_count(10, 28), 1);
        assert_eq!(shared_worker_count(50, 28), 2);
        assert_eq!(shared_worker_count(100, 28), 4);
        assert_eq!(shared_worker_count(250, 28), 10);
        assert_eq!(shared_worker_count(250, 3), 3);
    }

    #[test]
    fn real_workload_requires_input_and_resource_bounds_are_enforced() {
        let real = [
            "--architecture".to_owned(),
            "shared-pool".to_owned(),
            "--workload".to_owned(),
            "opus-passthrough-local".to_owned(),
            "--tracks".to_owned(),
            "1".to_owned(),
        ];
        assert!(parse_config(&real).is_err());

        let excessive = [
            "--architecture".to_owned(),
            "dedicated".to_owned(),
            "--workload".to_owned(),
            "synthetic".to_owned(),
            "--tracks".to_owned(),
            "251".to_owned(),
        ];
        assert!(parse_config(&excessive).is_err());
    }

    #[test]
    fn summaries_use_nearest_rank_percentiles() {
        let summary = summarize(&[5.0, 1.0, 3.0, 2.0, 4.0]);
        assert!((summary.median - 3.0).abs() < f64::EPSILON);
        assert!((summary.p95 - 5.0).abs() < f64::EPSILON);
        assert!((summary.mean - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn independent_pcm_sources_are_bit_exact_when_interleaved() {
        let input =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/media/fixtures/tone-mp3.mp3");
        let mut sources = (0..4)
            .map(|_| TrackSource::pcm(&input, Codec::Mp3, false).unwrap())
            .collect::<Vec<_>>();
        let mut frames = std::array::from_fn::<_, 4, _>(|_| EncodedFrameSlot::new());
        for _ in 0..50 {
            for (source, frame) in sources.iter_mut().zip(&mut frames) {
                source.produce(frame).unwrap();
            }
            for frame in &frames[1..] {
                assert_eq!(frame.data(), frames[0].data());
            }
        }
    }

    #[test]
    fn independent_pcm_sources_are_bit_exact_across_threads() {
        let input =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/media/fixtures/tone-mp3.mp3");
        let outputs = std::thread::scope(|scope| {
            let joins = (0..4)
                .map(|_| {
                    let input = &input;
                    scope.spawn(move || {
                        let mut source = TrackSource::pcm(input, Codec::Mp3, false).unwrap();
                        let mut frame = EncodedFrameSlot::new();
                        let mut output = Vec::new();
                        for _ in 0..50 {
                            source.produce(&mut frame).unwrap();
                            output.extend_from_slice(frame.data());
                        }
                        output
                    })
                })
                .collect::<Vec<_>>();
            joins
                .into_iter()
                .map(|join| join.join().unwrap())
                .collect::<Vec<_>>()
        });
        for output in &outputs[1..] {
            assert_eq!(output, &outputs[0]);
        }
    }
}
