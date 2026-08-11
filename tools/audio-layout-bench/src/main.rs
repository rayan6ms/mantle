use std::env;
use std::error::Error;
use std::hint::black_box;
use std::process::ExitCode;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;

const FRAME_SAMPLES_PER_CHANNEL: usize = 960;
const CHANNELS: usize = 2;
const INTERLEAVED_FRAME_SAMPLES: usize = FRAME_SAMPLES_PER_CHANNEL * CHANNELS;
const DEFAULT_ITERATIONS: u32 = 100_000;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Layout {
    Interleaved,
    Planar,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Workload {
    StereoVolume,
    StereoChannelFilter,
    MonoToStereoVolume,
}

#[derive(Clone, Copy, Debug)]
struct Config {
    layout: Layout,
    workload: Workload,
    repetition: u32,
    iterations: u32,
}

#[derive(Debug, Serialize)]
struct BenchmarkResult {
    schema_version: u32,
    timestamp_unix_ms: u128,
    layout: Layout,
    workload: Workload,
    repetition: u32,
    iterations: u32,
    frames_per_iteration: usize,
    samples_per_channel: usize,
    channels: usize,
    elapsed_ms: f64,
    nanoseconds_per_frame: f64,
    checksum: f64,
}

fn main() -> ExitCode {
    match run_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("mantle-audio-layout-bench: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run_main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let config = parse_config(&args)?;
    let input = make_signal(match config.workload {
        Workload::MonoToStereoVolume => FRAME_SAMPLES_PER_CHANNEL,
        Workload::StereoVolume | Workload::StereoChannelFilter => INTERLEAVED_FRAME_SAMPLES,
    });
    let mut output = vec![0.0_f32; INTERLEAVED_FRAME_SAMPLES];
    let mut left = vec![0.0_f32; FRAME_SAMPLES_PER_CHANNEL];
    let mut right = vec![0.0_f32; FRAME_SAMPLES_PER_CHANNEL];

    let started = Instant::now();
    match (config.layout, config.workload) {
        (Layout::Interleaved, Workload::StereoVolume) => {
            interleaved_stereo_volume(&input, &mut output, config.iterations);
        }
        (Layout::Planar, Workload::StereoVolume) => {
            planar_stereo_volume(
                &input,
                &mut left,
                &mut right,
                &mut output,
                config.iterations,
            );
        }
        (Layout::Interleaved, Workload::StereoChannelFilter) => {
            interleaved_stereo_filter(&input, &mut output, config.iterations);
        }
        (Layout::Planar, Workload::StereoChannelFilter) => {
            planar_stereo_filter(
                &input,
                &mut left,
                &mut right,
                &mut output,
                config.iterations,
            );
        }
        (Layout::Interleaved, Workload::MonoToStereoVolume) => {
            interleaved_mono_to_stereo(&input, &mut output, config.iterations);
        }
        (Layout::Planar, Workload::MonoToStereoVolume) => {
            planar_mono_to_stereo(
                &input,
                &mut left,
                &mut right,
                &mut output,
                config.iterations,
            );
        }
    }
    let elapsed = started.elapsed();
    let checksum = checksum(&output);
    black_box(checksum);
    let iterations = f64::from(config.iterations);
    let result = BenchmarkResult {
        schema_version: 1,
        timestamp_unix_ms: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
        layout: config.layout,
        workload: config.workload,
        repetition: config.repetition,
        iterations: config.iterations,
        frames_per_iteration: 1,
        samples_per_channel: FRAME_SAMPLES_PER_CHANNEL,
        channels: CHANNELS,
        elapsed_ms: elapsed.as_secs_f64() * 1_000.0,
        nanoseconds_per_frame: elapsed.as_secs_f64() * 1_000_000_000.0 / iterations,
        checksum,
    };
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

fn parse_config(args: &[String]) -> Result<Config> {
    let layout = match required_value(args, "--layout")?.as_str() {
        "interleaved" => Layout::Interleaved,
        "planar" => Layout::Planar,
        value => return Err(format!("unsupported layout {value}").into()),
    };
    let workload = match required_value(args, "--workload")?.as_str() {
        "stereo-volume" => Workload::StereoVolume,
        "stereo-channel-filter" => Workload::StereoChannelFilter,
        "mono-to-stereo-volume" => Workload::MonoToStereoVolume,
        value => return Err(format!("unsupported workload {value}").into()),
    };
    let repetition = value(args, "--repetition")
        .unwrap_or_else(|| "1".to_owned())
        .parse()?;
    let iterations = value(args, "--iterations")
        .unwrap_or_else(|| DEFAULT_ITERATIONS.to_string())
        .parse()?;
    if iterations == 0 {
        return Err("--iterations must be non-zero".into());
    }
    Ok(Config {
        layout,
        workload,
        repetition,
        iterations,
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

fn make_signal(samples: usize) -> Vec<f32> {
    let mut state = 0x9e37_79b9_u32;
    (0..samples)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let word = u16::try_from(state & 0xffff).expect("masked signal sample fits u16");
            f32::from(word) / 32_768.0 - 1.0
        })
        .collect()
}

fn gain(iteration: u32) -> f32 {
    if iteration & 1 == 0 { 0.73 } else { 0.91 }
}

fn interleaved_stereo_volume(input: &[f32], output: &mut [f32], iterations: u32) {
    for iteration in 0..iterations {
        let gain = gain(iteration);
        for (target, sample) in output.iter_mut().zip(input) {
            *target = *sample * gain;
        }
        black_box(output[usize::try_from(iteration).expect("u32 fits usize") % output.len()]);
    }
}

fn planar_stereo_volume(
    input: &[f32],
    left: &mut [f32],
    right: &mut [f32],
    output: &mut [f32],
    iterations: u32,
) {
    for iteration in 0..iterations {
        let gain = gain(iteration);
        for ((frame, left), right) in input
            .chunks_exact(2)
            .zip(left.iter_mut())
            .zip(right.iter_mut())
        {
            *left = frame[0] * gain;
            *right = frame[1] * gain;
        }
        interleave(left, right, output);
        black_box(output[usize::try_from(iteration).expect("u32 fits usize") % output.len()]);
    }
}

fn interleaved_stereo_filter(input: &[f32], output: &mut [f32], iterations: u32) {
    for iteration in 0..iterations {
        let mut state = [0.0_f32; 2];
        let gain = gain(iteration);
        for (source, target) in input.chunks_exact(2).zip(output.chunks_exact_mut(2)) {
            state[0] = source[0].mul_add(gain, state[0] * 0.125);
            state[1] = source[1].mul_add(gain, state[1] * 0.125);
            target.copy_from_slice(&state);
        }
        black_box(output[usize::try_from(iteration).expect("u32 fits usize") % output.len()]);
    }
}

fn planar_stereo_filter(
    input: &[f32],
    left: &mut [f32],
    right: &mut [f32],
    output: &mut [f32],
    iterations: u32,
) {
    for iteration in 0..iterations {
        for ((frame, left), right) in input
            .chunks_exact(2)
            .zip(left.iter_mut())
            .zip(right.iter_mut())
        {
            *left = frame[0];
            *right = frame[1];
        }
        filter_channel(left, gain(iteration));
        filter_channel(right, gain(iteration));
        interleave(left, right, output);
        black_box(output[usize::try_from(iteration).expect("u32 fits usize") % output.len()]);
    }
}

fn interleaved_mono_to_stereo(input: &[f32], output: &mut [f32], iterations: u32) {
    for iteration in 0..iterations {
        let gain = gain(iteration);
        for (sample, target) in input.iter().zip(output.chunks_exact_mut(2)) {
            let sample = *sample * gain;
            target.fill(sample);
        }
        black_box(output[usize::try_from(iteration).expect("u32 fits usize") % output.len()]);
    }
}

fn planar_mono_to_stereo(
    input: &[f32],
    left: &mut [f32],
    right: &mut [f32],
    output: &mut [f32],
    iterations: u32,
) {
    for iteration in 0..iterations {
        let gain = gain(iteration);
        for ((sample, left), right) in input.iter().zip(left.iter_mut()).zip(right.iter_mut()) {
            let sample = *sample * gain;
            *left = sample;
            *right = sample;
        }
        interleave(left, right, output);
        black_box(output[usize::try_from(iteration).expect("u32 fits usize") % output.len()]);
    }
}

fn filter_channel(channel: &mut [f32], gain: f32) {
    let mut state = 0.0_f32;
    for sample in channel {
        state = sample.mul_add(gain, state * 0.125);
        *sample = state;
    }
}

fn interleave(left: &[f32], right: &[f32], output: &mut [f32]) {
    for ((left, right), target) in left.iter().zip(right).zip(output.chunks_exact_mut(2)) {
        target[0] = *left;
        target[1] = *right;
    }
}

fn checksum(samples: &[f32]) -> f64 {
    samples.iter().map(|sample| f64::from(*sample)).sum()
}

#[cfg(test)]
mod tests {
    use super::{
        Config, Layout, Workload, interleaved_mono_to_stereo, interleaved_stereo_filter,
        interleaved_stereo_volume, make_signal, parse_config, planar_mono_to_stereo,
        planar_stereo_filter, planar_stereo_volume,
    };

    #[test]
    fn parses_required_layout_workload_and_bounds_iterations() {
        let args = [
            "--layout".to_owned(),
            "planar".to_owned(),
            "--workload".to_owned(),
            "stereo-channel-filter".to_owned(),
            "--repetition".to_owned(),
            "3".to_owned(),
            "--iterations".to_owned(),
            "17".to_owned(),
        ];
        assert!(matches!(
            parse_config(&args),
            Ok(Config {
                layout: Layout::Planar,
                workload: Workload::StereoChannelFilter,
                repetition: 3,
                iterations: 17,
            })
        ));
        let invalid = [
            "--layout".to_owned(),
            "interleaved".to_owned(),
            "--workload".to_owned(),
            "stereo-volume".to_owned(),
            "--iterations".to_owned(),
            "0".to_owned(),
        ];
        assert!(parse_config(&invalid).is_err());
    }

    #[test]
    fn layouts_produce_equal_results_for_every_workload() {
        let stereo = make_signal(16);
        let mono = make_signal(8);
        let mut interleaved = vec![0.0; 16];
        let mut planar = vec![0.0; 16];
        let mut left = vec![0.0; 8];
        let mut right = vec![0.0; 8];

        interleaved_stereo_volume(&stereo, &mut interleaved, 2);
        planar_stereo_volume(&stereo, &mut left, &mut right, &mut planar, 2);
        assert_eq!(interleaved, planar);

        interleaved_stereo_filter(&stereo, &mut interleaved, 2);
        planar_stereo_filter(&stereo, &mut left, &mut right, &mut planar, 2);
        assert_eq!(interleaved, planar);

        interleaved_mono_to_stereo(&mono, &mut interleaved, 2);
        planar_mono_to_stereo(&mono, &mut left, &mut right, &mut planar, 2);
        assert_eq!(interleaved, planar);
    }
}
