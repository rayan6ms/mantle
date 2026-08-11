mod runner_source;
mod schema;
mod trace;

use crate::schema::Scenario;
use crate::trace::{Backend, Trace};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("mantle-oracle: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.first().map(String::as_str) {
        Some("validate") => {
            load_scenario(&required_path(&args, "--scenario")?)?;
            Ok(())
        }
        Some("protocol") => {
            let scenario = load_scenario(&required_path(&args, "--scenario")?)?;
            write(&required_path(&args, "--output")?, scenario.protocol().as_bytes())
        }
        Some("write-runner") => {
            let backend = Backend::parse(&required_value(&args, "--backend")?)?;
            let source = match backend {
                Backend::Reference => runner_source::REFERENCE,
                Backend::Mantle => runner_source::MANTLE,
            };
            write(&required_path(&args, "--output")?, source.as_bytes())
        }
        Some("normalize") => {
            let scenario = load_scenario(&required_path(&args, "--scenario")?)?;
            let backend = Backend::parse(&required_value(&args, "--backend")?)?;
            let trace = trace::normalize(&scenario, backend, &required_path(&args, "--input")?)?;
            write_json(&required_path(&args, "--output")?, &trace)
        }
        Some("assert-deterministic") => {
            let first = load_trace(&required_path(&args, "--first")?)?;
            let second = load_trace(&required_path(&args, "--second")?)?;
            if first != second {
                return Err("normalized traces are not deterministic".into());
            }
            Ok(())
        }
        Some("compare") => {
            let reference = load_trace(&required_path(&args, "--reference")?)?;
            let mantle = load_trace(&required_path(&args, "--mantle")?)?;
            let comparison = trace::compare(&reference, &mantle)?;
            write_json(&required_path(&args, "--output")?, &comparison)
        }
        _ => Err("usage: mantle-oracle <validate|protocol|write-runner|normalize|assert-deterministic|compare> [options]".into()),
    }
}

fn load_scenario(path: &Path) -> Result<Scenario> {
    let scenario: Scenario = serde_json::from_slice(&fs::read(path)?)?;
    scenario.validate()?;
    Ok(scenario)
}

fn load_trace(path: &Path) -> Result<Trace> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    write(path, &bytes)
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    Ok(())
}

fn required_path(args: &[String], name: &str) -> Result<PathBuf> {
    Ok(PathBuf::from(required_value(args, name)?))
}

fn required_value(args: &[String], name: &str) -> Result<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
        .ok_or_else(|| format!("missing required option {name}").into())
}
