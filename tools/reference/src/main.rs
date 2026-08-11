use std::env;
use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use jni::objects::{JObject, JValue};
use jni::{InitArgsBuilder, JNIVersion, JavaVM, jni_sig, jni_str};
use serde::Serialize;

mod inventory;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug)]
struct RunConfig {
    classpath: String,
    input: Option<String>,
    tracks: usize,
    warmup_seconds: u64,
    measure_seconds: u64,
    filter: bool,
    http: bool,
    seek: bool,
    workload: String,
    repetition: usize,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct ProcSample {
    rss_kib: u32,
    pss_kib: u32,
    threads: u32,
}

#[derive(Debug, Clone, Copy)]
struct ProcCounters {
    cpu_ticks: u32,
    voluntary_context_switches: u64,
    involuntary_context_switches: u64,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct JvmCounters {
    gc_collections: i64,
    gc_time_ms: i64,
    heap_used_bytes: i64,
}

#[derive(Debug, Serialize)]
struct BenchmarkResult {
    schema_version: u32,
    timestamp_unix_ms: u128,
    workload: String,
    repetition: usize,
    input: Option<String>,
    input_mode: &'static str,
    tracks: usize,
    filter: bool,
    warmup_seconds: u64,
    measure_seconds: u64,
    startup_latency_ms: f64,
    load_latency_ms: Summary,
    first_frame_latency_ms: Summary,
    seek_latency_ms: Option<Summary>,
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
    gc_collections: i64,
    gc_time_ms: i64,
    heap_used_start_bytes: i64,
    heap_used_end_bytes: i64,
}

#[derive(Debug, Serialize)]
struct Summary {
    min: f64,
    median: f64,
    p95: f64,
    max: f64,
    mean: f64,
    samples: usize,
}

fn main() -> ExitCode {
    match run_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("mantle-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run_main() -> Result<()> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("benchmark") => run_benchmark(parse_run_config(&args.collect::<Vec<_>>())?),
        Some("inventory") => inventory::run(&args.collect::<Vec<_>>()),
        Some("seed-classification") => inventory::seed_classification(&args.collect::<Vec<_>>()),
        Some("serve") => {
            let root = required_value(&args.collect::<Vec<_>>(), "--root")?;
            serve(Path::new(&root), "127.0.0.1:18080")
        }
        _ => Err(
            "usage: mantle-reference benchmark <options> | inventory <options> | seed-classification <options> | serve --root <directory>"
                .into(),
        ),
    }
}

fn parse_run_config(args: &[String]) -> Result<RunConfig> {
    let classpath = required_value(args, "--classpath")?;
    let workload = required_value(args, "--workload")?;
    let tracks = value(args, "--tracks")
        .unwrap_or_else(|| "1".to_owned())
        .parse()?;
    let warmup_seconds = value(args, "--warmup-seconds")
        .unwrap_or_else(|| "3".to_owned())
        .parse()?;
    let measure_seconds = value(args, "--measure-seconds")
        .unwrap_or_else(|| "8".to_owned())
        .parse()?;
    let repetition = value(args, "--repetition")
        .unwrap_or_else(|| "1".to_owned())
        .parse()?;

    Ok(RunConfig {
        classpath,
        input: value(args, "--input"),
        tracks,
        warmup_seconds,
        measure_seconds,
        filter: args.iter().any(|arg| arg == "--filter"),
        http: args.iter().any(|arg| arg == "--http"),
        seek: args.iter().any(|arg| arg == "--seek"),
        workload,
        repetition,
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

fn run_benchmark(config: RunConfig) -> Result<()> {
    if config.tracks > 0 && config.input.is_none() {
        return Err("--input is required when --tracks is non-zero".into());
    }

    let vm_start = Instant::now();
    let classpath_option = format!("-Djava.class.path={}", config.classpath);
    let vm_args = InitArgsBuilder::new()
        .version(JNIVersion::V1_8)
        .option(&classpath_option)
        .option("-Xms64m")
        .option("-Xmx2g")
        .option("-XX:+UseG1GC")
        .option("-Dorg.slf4j.simpleLogger.defaultLogLevel=warn")
        .build()?;
    let vm = JavaVM::new(vm_args)?;
    vm.attach_current_thread(|jni| run_benchmark_attached(jni, config, vm_start))
}

// Keeping this linear makes the lifetime of the JVM-local player and track references explicit.
#[allow(clippy::too_many_lines)]
fn run_benchmark_attached(
    jni: &mut jni::Env<'_>,
    config: RunConfig,
    vm_start: Instant,
) -> Result<()> {
    let manager = jni.new_object(
        jni_str!("com/sedmelluq/discord/lavaplayer/player/DefaultAudioPlayerManager"),
        jni_sig!("()V"),
        &[],
    )?;

    if config.tracks > 0 {
        register_source(jni, &manager, config.http)?;
    }
    let startup_latency_ms = elapsed_ms(vm_start);

    let mut players = Vec::with_capacity(config.tracks);
    let mut tracks = Vec::with_capacity(config.tracks);
    let mut load_latencies = Vec::with_capacity(config.tracks);
    let mut starts = Vec::with_capacity(config.tracks);

    for _ in 0..config.tracks {
        let input = jni.new_string(config.input.as_deref().unwrap_or_default())?;
        let reference = jni.new_object(
            jni_str!("com/sedmelluq/discord/lavaplayer/track/AudioReference"),
            jni_sig!("(Ljava/lang/String;Ljava/lang/String;)V"),
            &[
                JValue::Object(input.as_ref()),
                JValue::Object(&JObject::null()),
            ],
        )?;
        let load_start = Instant::now();
        let track = jni
            .call_method(
                &manager,
                jni_str!("loadItemSync"),
                jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/track/AudioReference;)Lcom/sedmelluq/discord/lavaplayer/track/AudioItem;"),
                &[JValue::Object(&reference)],
            )?
            .l()?;
        load_latencies.push(elapsed_ms(load_start));
        if track.is_null() {
            return Err("Lavaplayer returned no track for the benchmark input".into());
        }

        let player = jni
            .call_method(
                &manager,
                jni_str!("createPlayer"),
                jni_sig!("()Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;"),
                &[],
            )?
            .l()?;
        if config.filter {
            let factory = jni.new_object(
                jni_str!("com/sedmelluq/discord/lavaplayer/filter/equalizer/EqualizerFactory"),
                jni_sig!("()V"),
                &[],
            )?;
            jni.call_method(
                &player,
                jni_str!("setFilterFactory"),
                jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/filter/PcmFilterFactory;)V"),
                &[JValue::Object(&factory)],
            )?;
            jni.delete_local_ref(factory);
        }
        let start = Instant::now();
        let started = jni
            .call_method(
                &player,
                jni_str!("startTrack"),
                jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;Z)Z"),
                &[JValue::Object(&track), JValue::Bool(false)],
            )?
            .z()?;
        if !started {
            return Err("Lavaplayer refused to start a benchmark track".into());
        }
        starts.push(start);
        players.push(player);
        tracks.push(track);
        jni.delete_local_ref(reference);
        jni.delete_local_ref(input);
    }

    let mut first_frame_latencies = vec![None; players.len()];
    let first_frame_deadline = Instant::now() + Duration::from_secs(15);
    while first_frame_latencies.iter().any(Option::is_none) && Instant::now() < first_frame_deadline
    {
        for (index, player) in players.iter().enumerate() {
            if first_frame_latencies[index].is_none() && provide_frame(jni, player)?.is_some() {
                first_frame_latencies[index] = Some(elapsed_ms(starts[index]));
            }
        }
        thread::sleep(Duration::from_millis(1));
    }
    if first_frame_latencies.iter().any(Option::is_none) {
        return Err("one or more tracks did not produce a frame within 15 seconds".into());
    }

    consume_for(
        jni,
        &players,
        Duration::from_secs(config.warmup_seconds),
        false,
    )?;

    let proc_before = read_proc_counters()?;
    let jvm_before = read_jvm_counters(jni)?;
    let measure_start = Instant::now();
    let consumption = consume_for(
        jni,
        &players,
        Duration::from_secs(config.measure_seconds),
        true,
    )?;
    let measured_seconds = measure_start.elapsed().as_secs_f64();
    let proc_after = read_proc_counters()?;
    let jvm_after = read_jvm_counters(jni)?;

    let seek_latency_ms = if config.seek {
        Some(measure_seeks(jni, &players[0], &tracks[0])?)
    } else {
        None
    };

    jni.call_method(&manager, jni_str!("shutdown"), jni_sig!("()V"), &[])?;

    let clock_ticks = clock_ticks_per_second()?;
    let cpu_ticks = proc_after.cpu_ticks.saturating_sub(proc_before.cpu_ticks);
    let result = BenchmarkResult {
        schema_version: 1,
        timestamp_unix_ms: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
        workload: config.workload,
        repetition: config.repetition,
        input: config.input,
        input_mode: if config.http { "http" } else { "local" },
        tracks: config.tracks,
        filter: config.filter,
        warmup_seconds: config.warmup_seconds,
        measure_seconds: config.measure_seconds,
        startup_latency_ms,
        load_latency_ms: summarize(&load_latencies),
        first_frame_latency_ms: summarize(
            &first_frame_latencies
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
        ),
        seek_latency_ms,
        cpu_core_percent: (f64::from(cpu_ticks) / f64::from(clock_ticks)) / measured_seconds
            * 100.0,
        rss_kib: summarize(
            &consumption
                .samples
                .iter()
                .map(|sample| f64::from(sample.rss_kib))
                .collect::<Vec<_>>(),
        ),
        pss_kib: summarize(
            &consumption
                .samples
                .iter()
                .map(|sample| f64::from(sample.pss_kib))
                .collect::<Vec<_>>(),
        ),
        threads: summarize(
            &consumption
                .samples
                .iter()
                .map(|sample| f64::from(sample.threads))
                .collect::<Vec<_>>(),
        ),
        voluntary_context_switches: proc_after
            .voluntary_context_switches
            .saturating_sub(proc_before.voluntary_context_switches),
        involuntary_context_switches: proc_after
            .involuntary_context_switches
            .saturating_sub(proc_before.involuntary_context_switches),
        frames_requested: consumption.frames_requested,
        frames_delivered: consumption.frames_delivered,
        frame_underruns: consumption.frame_underruns,
        skipped_deadlines: consumption.skipped_deadlines,
        gc_collections: jvm_after.gc_collections - jvm_before.gc_collections,
        gc_time_ms: jvm_after.gc_time_ms - jvm_before.gc_time_ms,
        heap_used_start_bytes: jvm_before.heap_used_bytes,
        heap_used_end_bytes: jvm_after.heap_used_bytes,
    };
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

fn register_source(jni: &mut jni::Env<'_>, manager: &JObject<'_>, http: bool) -> Result<()> {
    if http {
        let source = jni.new_object(
            jni_str!("com/sedmelluq/discord/lavaplayer/source/http/HttpAudioSourceManager"),
            jni_sig!("()V"),
            &[],
        )?;
        jni.call_method(
            manager,
            jni_str!("registerSourceManager"),
            jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/source/AudioSourceManager;)V"),
            &[JValue::Object(&source)],
        )?;
        jni.delete_local_ref(source);
    } else {
        jni.call_static_method(
            jni_str!("com/sedmelluq/discord/lavaplayer/source/AudioSourceManagers"),
            jni_str!("registerLocalSource"),
            jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayerManager;)V"),
            &[JValue::Object(manager)],
        )?;
    }
    Ok(())
}

fn provide_frame(jni: &mut jni::Env<'_>, player: &JObject<'_>) -> Result<Option<i64>> {
    let frame = jni
        .call_method(
            player,
            jni_str!("provide"),
            jni_sig!("()Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioFrame;"),
            &[],
        )?
        .l()?;
    if frame.is_null() {
        Ok(None)
    } else {
        let timecode = jni
            .call_method(&frame, jni_str!("getTimecode"), jni_sig!("()J"), &[])?
            .j()?;
        jni.delete_local_ref(frame);
        Ok(Some(timecode))
    }
}

struct Consumption {
    samples: Vec<ProcSample>,
    frames_requested: u64,
    frames_delivered: u64,
    frame_underruns: u64,
    skipped_deadlines: u64,
}

fn consume_for(
    jni: &mut jni::Env<'_>,
    players: &[JObject<'_>],
    duration: Duration,
    collect: bool,
) -> Result<Consumption> {
    let start = Instant::now();
    let end = start + duration;
    let frame_period = Duration::from_millis(20);
    let mut next_frame = start;
    let mut next_sample = start;
    let mut output = Consumption {
        samples: Vec::new(),
        frames_requested: 0,
        frames_delivered: 0,
        frame_underruns: 0,
        skipped_deadlines: 0,
    };

    while Instant::now() < end {
        let now = Instant::now();
        if now >= next_frame {
            if now.saturating_duration_since(next_frame) >= frame_period {
                output.skipped_deadlines = output.skipped_deadlines.saturating_add(
                    (now.saturating_duration_since(next_frame).as_millis() / 20)
                        .try_into()
                        .unwrap_or(u64::MAX),
                );
            }
            for player in players {
                output.frames_requested += 1;
                if provide_frame(jni, player)?.is_some() {
                    output.frames_delivered += 1;
                } else {
                    output.frame_underruns += 1;
                }
            }
            next_frame += frame_period;
        }
        if collect && now >= next_sample {
            output.samples.push(read_proc_sample()?);
            next_sample += Duration::from_millis(250);
        }
        let wake = next_frame.min(next_sample).min(end);
        if wake > Instant::now() {
            thread::sleep(wake - Instant::now());
        }
    }
    if collect && output.samples.is_empty() {
        output.samples.push(read_proc_sample()?);
    }
    Ok(output)
}

fn measure_seeks(
    jni: &mut jni::Env<'_>,
    player: &JObject<'_>,
    track: &JObject<'_>,
) -> Result<Summary> {
    let mut latencies = Vec::with_capacity(10);
    for target in [
        10_000_i64, 40_000, 15_000, 45_000, 20_000, 50_000, 25_000, 35_000, 5_000, 30_000,
    ] {
        let start = Instant::now();
        jni.call_method(
            track,
            jni_str!("setPosition"),
            jni_sig!("(J)V"),
            &[JValue::Long(target)],
        )?;
        let deadline = start + Duration::from_secs(5);
        let mut observed_min = i64::MAX;
        let mut observed_max = i64::MIN;
        loop {
            if let Some(timecode) = provide_frame(jni, player)? {
                observed_min = observed_min.min(timecode);
                observed_max = observed_max.max(timecode);
                if timecode.abs_diff(target) <= 1_000 {
                    latencies.push(elapsed_ms(start));
                    break;
                }
            }
            if Instant::now() >= deadline {
                return Err(
                    format!(
                        "seek to {target} ms did not complete within 5 seconds; observed {observed_min}..={observed_max} ms"
                    )
                    .into(),
                );
            }
            thread::sleep(Duration::from_millis(1));
        }
    }
    Ok(summarize(&latencies))
}

fn read_proc_sample() -> Result<ProcSample> {
    let status = fs::read_to_string("/proc/self/status")?;
    let rss_kib = status_value(&status, "VmRSS:")?.try_into()?;
    let threads = status_value(&status, "Threads:")?.try_into()?;
    let smaps = fs::read_to_string("/proc/self/smaps_rollup")?;
    let pss_kib = status_value(&smaps, "Pss:")?.try_into()?;
    Ok(ProcSample {
        rss_kib,
        pss_kib,
        threads,
    })
}

fn read_proc_counters() -> Result<ProcCounters> {
    let stat = fs::read_to_string("/proc/self/stat")?;
    let after_comm = stat
        .rsplit_once(") ")
        .ok_or("unexpected /proc/self/stat format")?
        .1;
    let fields = after_comm.split_whitespace().collect::<Vec<_>>();
    let user_ticks: u32 = fields.get(11).ok_or("missing user CPU ticks")?.parse()?;
    let system_ticks: u32 = fields.get(12).ok_or("missing system CPU ticks")?.parse()?;
    let status = fs::read_to_string("/proc/self/status")?;
    Ok(ProcCounters {
        cpu_ticks: user_ticks.saturating_add(system_ticks),
        voluntary_context_switches: status_value(&status, "voluntary_ctxt_switches:")?,
        involuntary_context_switches: status_value(&status, "nonvoluntary_ctxt_switches:")?,
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

fn read_jvm_counters(jni: &mut jni::Env<'_>) -> Result<JvmCounters> {
    let beans = jni
        .call_static_method(
            jni_str!("java/lang/management/ManagementFactory"),
            jni_str!("getGarbageCollectorMXBeans"),
            jni_sig!("()Ljava/util/List;"),
            &[],
        )?
        .l()?;
    let count = jni
        .call_method(&beans, jni_str!("size"), jni_sig!("()I"), &[])?
        .i()?;
    let mut gc_collections = 0;
    let mut gc_time_ms = 0;
    for index in 0..count {
        let bean = jni
            .call_method(
                &beans,
                jni_str!("get"),
                jni_sig!("(I)Ljava/lang/Object;"),
                &[JValue::Int(index)],
            )?
            .l()?;
        let collections = jni
            .call_method(&bean, jni_str!("getCollectionCount"), jni_sig!("()J"), &[])?
            .j()?;
        let time = jni
            .call_method(&bean, jni_str!("getCollectionTime"), jni_sig!("()J"), &[])?
            .j()?;
        gc_collections += collections.max(0);
        gc_time_ms += time.max(0);
        jni.delete_local_ref(bean);
    }
    jni.delete_local_ref(beans);

    let memory = jni
        .call_static_method(
            jni_str!("java/lang/management/ManagementFactory"),
            jni_str!("getMemoryMXBean"),
            jni_sig!("()Ljava/lang/management/MemoryMXBean;"),
            &[],
        )?
        .l()?;
    let usage = jni
        .call_method(
            &memory,
            jni_str!("getHeapMemoryUsage"),
            jni_sig!("()Ljava/lang/management/MemoryUsage;"),
            &[],
        )?
        .l()?;
    let heap_used_bytes = jni
        .call_method(&usage, jni_str!("getUsed"), jni_sig!("()J"), &[])?
        .j()?;
    jni.delete_local_ref(usage);
    jni.delete_local_ref(memory);

    Ok(JvmCounters {
        gc_collections,
        gc_time_ms,
        heap_used_bytes,
    })
}

fn clock_ticks_per_second() -> Result<u32> {
    let output = std::process::Command::new("getconf")
        .arg("CLK_TCK")
        .output()?;
    if !output.status.success() {
        return Err("getconf CLK_TCK failed".into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().parse()?)
}

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

fn summarize(values: &[f64]) -> Summary {
    if values.is_empty() {
        return Summary {
            min: 0.0,
            median: 0.0,
            p95: 0.0,
            max: 0.0,
            mean: 0.0,
            samples: 0,
        };
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let percentile = |numerator: usize, denominator: usize| {
        let scaled = (sorted.len() - 1).saturating_mul(numerator);
        let index = scaled.div_ceil(denominator);
        sorted[index]
    };
    let count =
        u32::try_from(sorted.len()).expect("benchmark summary has at most u32::MAX samples");
    Summary {
        min: sorted[0],
        median: percentile(1, 2),
        p95: percentile(95, 100),
        max: sorted[sorted.len() - 1],
        mean: sorted.iter().sum::<f64>() / f64::from(count),
        samples: sorted.len(),
    }
}

fn serve(root: &Path, address: &str) -> Result<()> {
    let root = root.canonicalize()?;
    let listener = TcpListener::bind(address)?;
    eprintln!("serving {} on http://{address}", root.display());
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let root = root.clone();
                thread::spawn(move || {
                    if let Err(error) = serve_connection(stream, &root) {
                        eprintln!("HTTP fixture request failed: {error}");
                    }
                });
            }
            Err(error) => eprintln!("HTTP fixture accept failed: {error}"),
        }
    }
    Ok(())
}

fn serve_connection(mut stream: TcpStream, root: &Path) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut request = [0_u8; 16 * 1024];
    let count = stream.read(&mut request)?;
    let request = String::from_utf8_lossy(&request[..count]);
    let mut lines = request.lines();
    let first = lines.next().ok_or("empty HTTP request")?;
    let mut parts = first.split_whitespace();
    let method = parts.next().ok_or("missing HTTP method")?;
    let raw_path = parts.next().ok_or("missing HTTP path")?;
    if method != "GET" && method != "HEAD" {
        stream.write_all(
            b"HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )?;
        return Ok(());
    }
    let relative = raw_path.trim_start_matches('/');
    if relative.contains("..") || relative.contains('\\') {
        stream.write_all(
            b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )?;
        return Ok(());
    }
    let path = root.join(relative).canonicalize()?;
    if !path.starts_with(root) || !path.is_file() {
        stream.write_all(
            b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )?;
        return Ok(());
    }
    let data = fs::read(path)?;
    let range = lines.find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("range")
            .then(|| value.trim().to_owned())
    });
    let (status, start, end) = parse_range(range.as_deref(), data.len())?;
    let length = end - start;
    let mut headers = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {length}\r\nAccept-Ranges: bytes\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n"
    );
    if status.starts_with("206") {
        use std::fmt::Write as _;
        write!(
            headers,
            "Content-Range: bytes {start}-{}/{total}\r\n",
            end - 1,
            total = data.len()
        )?;
    }
    headers.push_str("\r\n");
    stream.write_all(headers.as_bytes())?;
    if method == "GET" {
        stream.write_all(&data[start..end])?;
    }
    Ok(())
}

fn parse_range(range: Option<&str>, length: usize) -> Result<(&'static str, usize, usize)> {
    let Some(range) = range else {
        return Ok(("200 OK", 0, length));
    };
    let value = range
        .strip_prefix("bytes=")
        .ok_or("unsupported HTTP range unit")?;
    let (start, end) = value.split_once('-').ok_or("malformed HTTP range")?;
    let start: usize = start.parse()?;
    let end = if end.is_empty() {
        length
    } else {
        end.parse::<usize>()?.saturating_add(1).min(length)
    };
    if start >= end || start >= length {
        return Err("unsatisfiable HTTP range".into());
    }
    Ok(("206 Partial Content", start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_uses_nearest_rank_percentiles() {
        let summary = summarize(&[1.0, 2.0, 3.0, 4.0]);
        assert!((summary.min - 1.0).abs() < f64::EPSILON);
        assert!((summary.median - 3.0).abs() < f64::EPSILON);
        assert!((summary.p95 - 4.0).abs() < f64::EPSILON);
        assert!((summary.mean - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn parses_open_ended_http_range() -> Result<()> {
        assert_eq!(
            parse_range(Some("bytes=4-"), 10)?,
            ("206 Partial Content", 4, 10)
        );
        Ok(())
    }

    #[test]
    fn rejects_invalid_http_range() {
        assert!(parse_range(Some("items=1-2"), 10).is_err());
        assert!(parse_range(Some("bytes=10-"), 10).is_err());
    }
}
