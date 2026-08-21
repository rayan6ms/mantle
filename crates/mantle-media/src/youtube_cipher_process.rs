use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::{
    YoutubeCipherChallenge, YoutubeCipherResolver, YoutubeCipherResolverError,
    YoutubeCipherResolverErrorKind, YoutubeCipherSolution,
};

const PROTOCOL_VERSION: u32 = 1;
const RESPONSE_PREFIX: &[u8] = b"MANTLE_YOUTUBE_CIPHER_V1\t";
const MAX_CONFIGURED_PROCESS_BYTES: usize = 64 * 1024 * 1024;
const MAX_CONFIGURED_ARGUMENTS: usize = 32;
const MAX_CONFIGURED_ARGUMENT_BYTES: usize = 16 * 1024;
const MAX_CONFIGURED_TIMEOUT: Duration = Duration::from_mins(2);
const MAX_CONFIGURED_POLL_INTERVAL: Duration = Duration::from_secs(1);
const MAX_CONFIGURED_HEAP_MEGABYTES: u16 = 1_024;

/// Resource ceilings for one isolated cipher-provider subprocess.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct YoutubeProcessCipherOptions {
    pub timeout: Duration,
    pub poll_interval: Duration,
    pub max_request_bytes: usize,
    pub max_stdout_bytes: usize,
    pub max_stderr_bytes: usize,
    pub max_heap_megabytes: u16,
}

impl Default for YoutubeProcessCipherOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(10),
            poll_interval: Duration::from_millis(10),
            max_request_bytes: 32 * 1024 * 1024,
            max_stdout_bytes: 64 * 1024,
            max_stderr_bytes: 64 * 1024,
            max_heap_megabytes: 128,
        }
    }
}

impl YoutubeProcessCipherOptions {
    fn validate(self) -> Result<Self, YoutubeCipherResolverError> {
        if self.timeout.is_zero()
            || self.timeout > MAX_CONFIGURED_TIMEOUT
            || self.poll_interval.is_zero()
            || self.poll_interval > MAX_CONFIGURED_POLL_INTERVAL
            || self.poll_interval > self.timeout
            || self.max_request_bytes == 0
            || self.max_request_bytes > MAX_CONFIGURED_PROCESS_BYTES
            || self.max_stdout_bytes == 0
            || self.max_stdout_bytes > MAX_CONFIGURED_PROCESS_BYTES
            || self.max_stderr_bytes == 0
            || self.max_stderr_bytes > MAX_CONFIGURED_PROCESS_BYTES
            || self.max_heap_megabytes == 0
            || self.max_heap_megabytes > MAX_CONFIGURED_HEAP_MEGABYTES
        {
            return Err(execution_failed());
        }
        Ok(self)
    }
}

/// Isolated, environment-cleared subprocess implementation of the cipher-resolver protocol.
pub struct YoutubeProcessCipherResolver {
    executable: PathBuf,
    arguments: Vec<OsString>,
    options: YoutubeProcessCipherOptions,
}

impl YoutubeProcessCipherResolver {
    /// Creates a provider for a trusted absolute executable and fixed argument vector.
    ///
    /// The executable is invoked directly without a shell. Its environment is cleared, standard
    /// input carries one bounded JSON request, and exactly one prefixed JSON response is accepted.
    ///
    /// # Errors
    ///
    /// Returns an execution error for invalid bounds, a relative or missing executable, excessive
    /// arguments, or non-Unicode arguments.
    pub fn new(
        executable: impl AsRef<Path>,
        arguments: impl IntoIterator<Item = impl AsRef<OsStr>>,
        options: YoutubeProcessCipherOptions,
    ) -> Result<Self, YoutubeCipherResolverError> {
        let options = options.validate()?;
        let executable = executable.as_ref();
        if !executable.is_absolute() || !executable.is_file() {
            return Err(execution_failed());
        }
        let arguments = arguments
            .into_iter()
            .map(|argument| argument.as_ref().to_os_string())
            .collect::<Vec<_>>();
        let argument_bytes = arguments.iter().try_fold(0_usize, |total, argument| {
            argument
                .to_str()
                .and_then(|argument| total.checked_add(argument.len()))
        });
        if arguments.len() > MAX_CONFIGURED_ARGUMENTS
            || argument_bytes.is_none_or(|bytes| bytes > MAX_CONFIGURED_ARGUMENT_BYTES)
        {
            return Err(execution_failed());
        }
        Ok(Self {
            executable: executable.to_owned(),
            arguments,
            options,
        })
    }

    /// Creates the recommended Deno runner for one trusted, self-contained EJS adapter.
    ///
    /// Deno explicitly denies every I/O permission, receives no inherited environment, and has no
    /// configuration or lock-file discovery, permission prompts, remote or npm modules, code cache,
    /// or unbounded V8 old-generation heap. The adapter must therefore contain all reviewed solver
    /// dependencies.
    ///
    /// # Errors
    ///
    /// Returns an execution error when paths or resource bounds are invalid.
    pub fn deno(
        executable: impl AsRef<Path>,
        adapter: impl AsRef<Path>,
        options: YoutubeProcessCipherOptions,
    ) -> Result<Self, YoutubeCipherResolverError> {
        let adapter = adapter.as_ref();
        if !adapter.is_absolute() || !adapter.is_file() {
            return Err(execution_failed());
        }
        let arguments = [
            OsString::from("run"),
            OsString::from("--quiet"),
            OsString::from("--ext=js"),
            OsString::from("--no-code-cache"),
            OsString::from("--no-prompt"),
            OsString::from("--deny-read"),
            OsString::from("--deny-write"),
            OsString::from("--deny-net"),
            OsString::from("--deny-env"),
            OsString::from("--deny-sys"),
            OsString::from("--deny-run"),
            OsString::from("--deny-ffi"),
            OsString::from("--deny-import"),
            OsString::from("--no-remote"),
            OsString::from("--no-lock"),
            OsString::from("--node-modules-dir=none"),
            OsString::from("--no-config"),
            OsString::from("--no-npm"),
            OsString::from("--cached-only"),
            OsString::from(format!(
                "--v8-flags=--max-old-space-size={}",
                options.max_heap_megabytes
            )),
            adapter.as_os_str().to_os_string(),
        ];
        Self::new(executable, arguments, options)
    }

    fn request_bytes(
        &self,
        challenge: &YoutubeCipherChallenge<'_>,
    ) -> Result<Vec<u8>, YoutubeCipherResolverError> {
        let player_script =
            std::str::from_utf8(challenge.player_script()).map_err(|_| execution_failed())?;
        let worst_case_bytes = player_script
            .len()
            .checked_add(challenge.player_script_url().len())
            .and_then(|length| length.checked_add(challenge.signature().map_or(0, str::len)))
            .and_then(|length| length.checked_add(challenge.n_parameter().map_or(0, str::len)))
            .and_then(|length| length.checked_mul(6))
            .and_then(|length| length.checked_add(1024))
            .ok_or_else(execution_failed)?;
        if worst_case_bytes > self.options.max_request_bytes {
            return Err(execution_failed());
        }
        let request = ProcessRequest {
            version: PROTOCOL_VERSION,
            player_script_url: challenge.player_script_url(),
            player_script,
            signature: challenge.signature(),
            n_parameter: challenge.n_parameter(),
            max_output_bytes: challenge.max_output_bytes(),
        };
        let bytes = serde_json::to_vec(&request).map_err(|_| execution_failed())?;
        if bytes.len() > self.options.max_request_bytes {
            return Err(execution_failed());
        }
        Ok(bytes)
    }
}

impl fmt::Debug for YoutubeProcessCipherResolver {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YoutubeProcessCipherResolver")
            .field("executable", &"<redacted>")
            .field("argument_count", &self.arguments.len())
            .field("options", &self.options)
            .finish()
    }
}

impl YoutubeCipherResolver for YoutubeProcessCipherResolver {
    fn resolve(
        &self,
        challenge: &YoutubeCipherChallenge<'_>,
    ) -> Result<YoutubeCipherSolution, YoutubeCipherResolverError> {
        if challenge.cancellation().is_cancelled() {
            return Err(cancelled());
        }
        let request = self.request_bytes(challenge)?;
        let mut child = Command::new(&self.executable)
            .args(&self.arguments)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| execution_failed())?;
        let Some((stdin, stdout, stderr)) = child
            .stdin
            .take()
            .zip(child.stdout.take())
            .zip(child.stderr.take())
            .map(|((stdin, stdout), stderr)| (stdin, stdout, stderr))
        else {
            terminate_child(&mut child);
            return Err(execution_failed());
        };
        let writer = thread::spawn(move || write_request(stdin, &request));
        let stdout_reader = spawn_bounded_reader(stdout, self.options.max_stdout_bytes);
        let stderr_reader = spawn_bounded_reader(stderr, self.options.max_stderr_bytes);
        let status = wait_for_child(
            &mut child,
            challenge,
            self.options.timeout,
            self.options.poll_interval,
        );
        let writer_ok = writer.join().is_ok_and(|result| result.is_ok());
        let stdout = stdout_reader.join().ok().and_then(Result::ok);
        let stderr_ok = stderr_reader.join().is_ok_and(|result| result.is_ok());
        let status = status?;
        if !status.success() || !writer_ok || !stderr_ok {
            return Err(execution_failed());
        }
        parse_response(stdout.as_deref().ok_or_else(execution_failed)?)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProcessRequest<'a> {
    version: u32,
    player_script_url: &'a str,
    player_script: &'a str,
    signature: Option<&'a str>,
    n_parameter: Option<&'a str>,
    max_output_bytes: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProcessResponse {
    version: u32,
    signature: Option<String>,
    n_parameter: Option<String>,
}

fn write_request(mut stdin: impl Write, request: &[u8]) -> Result<(), YoutubeCipherResolverError> {
    stdin.write_all(request).map_err(|_| execution_failed())?;
    stdin.flush().map_err(|_| execution_failed())
}

fn spawn_bounded_reader(
    mut reader: impl Read + Send + 'static,
    max_bytes: usize,
) -> thread::JoinHandle<Result<Vec<u8>, YoutubeCipherResolverError>> {
    thread::spawn(move || {
        let mut output = Vec::with_capacity(max_bytes.min(64 * 1024));
        let mut buffer = [0_u8; 8 * 1024];
        loop {
            let count = reader.read(&mut buffer).map_err(|_| execution_failed())?;
            if count == 0 {
                return Ok(output);
            }
            if output.len().saturating_add(count) > max_bytes {
                return Err(execution_failed());
            }
            output.extend_from_slice(&buffer[..count]);
        }
    })
}

fn wait_for_child(
    child: &mut Child,
    challenge: &YoutubeCipherChallenge<'_>,
    timeout: Duration,
    poll_interval: Duration,
) -> Result<std::process::ExitStatus, YoutubeCipherResolverError> {
    let started = Instant::now();
    loop {
        if challenge.cancellation().is_cancelled() {
            terminate_child(child);
            return Err(cancelled());
        }
        if started.elapsed() >= timeout {
            terminate_child(child);
            return Err(execution_failed());
        }
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {}
            Err(_) => {
                terminate_child(child);
                return Err(execution_failed());
            }
        }
        thread::sleep(poll_interval.min(timeout.saturating_sub(started.elapsed())));
    }
}

fn terminate_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn parse_response(bytes: &[u8]) -> Result<YoutubeCipherSolution, YoutubeCipherResolverError> {
    let mut response = None;
    for line in bytes.split(|byte| *byte == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        let Some(json) = line.strip_prefix(RESPONSE_PREFIX) else {
            continue;
        };
        if response.is_some() {
            return Err(execution_failed());
        }
        response =
            Some(serde_json::from_slice::<ProcessResponse>(json).map_err(|_| execution_failed())?);
    }
    let response = response.ok_or_else(execution_failed)?;
    if response.version != PROTOCOL_VERSION {
        return Err(execution_failed());
    }
    Ok(YoutubeCipherSolution::new(
        response.signature,
        response.n_parameter,
    ))
}

const fn execution_failed() -> YoutubeCipherResolverError {
    YoutubeCipherResolverError::new(YoutubeCipherResolverErrorKind::ExecutionFailed)
}

const fn cancelled() -> YoutubeCipherResolverError {
    YoutubeCipherResolverError::new(YoutubeCipherResolverErrorKind::Cancelled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deno_profile_is_self_contained_and_grants_no_permissions() {
        let executable = std::env::current_exe().unwrap();
        let resolver = YoutubeProcessCipherResolver::deno(
            &executable,
            &executable,
            YoutubeProcessCipherOptions::default(),
        )
        .unwrap();
        let arguments = resolver
            .arguments
            .iter()
            .map(|argument| argument.to_str().unwrap())
            .collect::<Vec<_>>();

        assert!(arguments.contains(&"--no-code-cache"));
        assert!(arguments.contains(&"--no-config"));
        assert!(arguments.contains(&"--no-lock"));
        assert!(arguments.contains(&"--no-npm"));
        assert!(arguments.contains(&"--no-remote"));
        assert!(arguments.contains(&"--cached-only"));
        for permission in ["read", "write", "net", "env", "sys", "run", "ffi", "import"] {
            assert!(arguments.contains(&format!("--deny-{permission}").as_str()));
        }
        assert!(arguments.iter().all(|argument| {
            !argument.starts_with("--allow-") && *argument != "-A" && *argument != "--allow-all"
        }));
        assert!(!format!("{resolver:?}").contains(executable.to_str().unwrap()));
    }
}
