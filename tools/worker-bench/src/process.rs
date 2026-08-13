use std::error::Error;
use std::fs;
use std::process::Command;

use serde::Serialize;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Clone, Copy, Debug)]
pub struct ProcCounters {
    pub cpu_ticks: u64,
    pub voluntary_context_switches: u64,
    pub involuntary_context_switches: u64,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct ProcSample {
    pub rss_kib: u64,
    pub pss_kib: u64,
    pub threads: u64,
}

pub fn read_proc_counters() -> Result<ProcCounters> {
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

pub fn read_proc_sample() -> Result<ProcSample> {
    let status = fs::read_to_string("/proc/self/status")?;
    let smaps = fs::read_to_string("/proc/self/smaps_rollup")?;
    Ok(ProcSample {
        rss_kib: status_value(&status, "VmRSS:")?,
        pss_kib: status_value(&smaps, "Pss:")?,
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

pub fn clock_ticks_per_second() -> Result<u32> {
    let output = Command::new("getconf").arg("CLK_TCK").output()?;
    if !output.status.success() {
        return Err("getconf CLK_TCK failed".into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().parse()?)
}
