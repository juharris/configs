use std::future::Future;
use std::path::PathBuf;
use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;

/// Gives an interrupted coding agent time to end its turn and save its session before the process group is killed.
const INTERRUPT_GRACE_PERIOD: Duration = Duration::from_secs(10);
const MAX_ERROR_DETAIL_CHARACTERS: usize = 2_000;
const OUTPUT_CHUNK_BYTES: usize = 8 * 1024;

/// Bounds one captured command by its combined stdout and stderr size and its running time.
#[derive(Clone, Copy, Debug)]
pub struct CaptureLimits {
    pub max_output_bytes: usize,
    pub timeout: Duration,
}

/// Holds the exit status and complete output of a captured command.
#[derive(Debug)]
pub struct CapturedOutput {
    pub status: ExitStatus,
    pub stderr: Vec<u8>,
    pub stdout: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("command was cancelled")]
    Cancelled,
    #[error("command exceeded the configured output limit")]
    OutputLimit,
    #[error("could not read command output: {0}")]
    OutputRead(#[source] std::io::Error),
    #[error("could not start the configured command: {0}")]
    Start(#[source] std::io::Error),
    #[error("could not read command status: {0}")]
    Status(#[source] std::io::Error),
    #[error("command timed out")]
    Timeout,
}

impl ExecutionError {
    /// Names the failure for logs without including command output.
    pub fn category(&self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::OutputLimit => "output_limit",
            Self::OutputRead(_) => "output_read",
            Self::Start(_) => "start",
            Self::Status(_) => "status",
            Self::Timeout => "timeout",
        }
    }
}

/// Kills a command's process group when dropped so an aborted task never leaves a pipeline running.
pub struct ProcessGroupGuard {
    process_id: Option<u32>,
}

impl ProcessGroupGuard {
    pub fn new(child: &Child) -> Self {
        Self {
            process_id: child.id(),
        }
    }

    /// Leaves the process group alone, for example after its shell exits and its background work should continue.
    pub fn disarm(&mut self) {
        self.process_id = None;
    }
}

impl Drop for ProcessGroupGuard {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(process_id) = self.process_id {
            signal_process_group(process_id, libc::SIGKILL);
        }
    }
}

/// Describes a compiled configured script with the positional values for its placeholders.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShellScript {
    pub arguments: Vec<String>,
    pub script: String,
    pub shell: PathBuf,
    pub working_directory: Option<PathBuf>,
}

impl ShellScript {
    /// Builds a command that runs the script in its own process group with piped output and no input.
    pub fn command(&self) -> Command {
        let mut command = Command::new(&self.shell);
        command
            .args(["-c", &self.script, "personal-dashboard"])
            .args(&self.arguments)
            .kill_on_drop(true)
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped());
        if let Some(working_directory) = &self.working_directory {
            command.current_dir(working_directory);
        }
        #[cfg(unix)]
        command.process_group(0);
        command
    }
}

/// Selects how a running command stops after cancellation or a timeout.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Termination {
    /// Interrupts the process group first so a coding agent can end its turn cleanly.
    Interrupt,
    Kill,
}

struct BoundedOutput {
    bytes: Vec<u8>,
    exceeded: bool,
}

/// Runs a script to completion and captures its output unless it is cancelled, times out, or exceeds its output limit.
pub async fn capture(
    script: &ShellScript,
    limits: CaptureLimits,
    termination: Termination,
    cancellation: impl Future,
) -> Result<CapturedOutput, ExecutionError> {
    let mut child = script.command().spawn().map_err(ExecutionError::Start)?;
    let mut process_group = ProcessGroupGuard::new(&child);
    let stderr = child
        .stderr
        .take()
        .expect("piped command stderr is available");
    let stdout = child
        .stdout
        .take()
        .expect("piped command stdout is available");
    let remaining = Arc::new(AtomicUsize::new(limits.max_output_bytes));
    let mut stderr_reader = tokio::spawn(read_bounded(stderr, remaining.clone()));
    let mut stdout_reader = tokio::spawn(read_bounded(stdout, remaining));

    let stopped = {
        // Output is read inside the deadline because a background process can keep a pipe open after the shell exits.
        let completed = async {
            let status = child.wait().await.map_err(ExecutionError::Status)?;
            let stderr = join_reader(&mut stderr_reader).await?;
            let stdout = join_reader(&mut stdout_reader).await?;
            Ok::<_, ExecutionError>((status, stderr, stdout))
        };
        tokio::select! {
            completed = completed => Ok(completed),
            _ = cancellation => Err(ExecutionError::Cancelled),
            _ = tokio::time::sleep(limits.timeout) => Err(ExecutionError::Timeout),
        }
    };
    let (status, stderr, stdout) = match stopped {
        Ok(completed) => completed?,
        Err(error) => {
            match termination {
                Termination::Interrupt => interrupt(&mut child).await,
                Termination::Kill => terminate(&mut child).await,
            }
            process_group.disarm();
            stderr_reader.abort();
            stdout_reader.abort();
            return Err(error);
        }
    };
    process_group.disarm();

    if stderr.exceeded || stdout.exceeded {
        return Err(ExecutionError::OutputLimit);
    }
    Ok(CapturedOutput {
        status,
        stderr: stderr.bytes,
        stdout: stdout.bytes,
    })
}

/// Describes an exit status for people and logs, using `unknown` when a signal ended the process.
pub fn exit_status_code(status: ExitStatus) -> String {
    status
        .code()
        .map_or_else(|| "unknown".to_owned(), |code| code.to_string())
}

/// Interrupts the complete process group, waits for the shell to exit, and then kills any remaining processes.
pub async fn interrupt(child: &mut Child) {
    #[cfg(unix)]
    if let Some(process_id) = child.id() {
        signal_process_group(process_id, libc::SIGINT);
        let _ = tokio::time::timeout(INTERRUPT_GRACE_PERIOD, child.wait()).await;
        // Tool processes started by an agent can outlive the interrupted shell.
        signal_process_group(process_id, libc::SIGKILL);
    }
    terminate(child).await;
}

/// Reads output until the shared byte budget is exhausted, then keeps draining so the process never blocks on a full pipe.
async fn read_bounded(
    mut reader: impl AsyncRead + Unpin,
    remaining: Arc<AtomicUsize>,
) -> Result<BoundedOutput, std::io::Error> {
    let mut bytes = Vec::new();
    let mut exceeded = false;
    let mut buffer = [0_u8; OUTPUT_CHUNK_BYTES];
    loop {
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            return Ok(BoundedOutput { bytes, exceeded });
        }
        let available = remaining
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |available| {
                Some(available.saturating_sub(read))
            })
            .expect("the remaining output count always updates");
        let accepted = available.min(read);
        bytes.extend_from_slice(&buffer[..accepted]);
        exceeded |= accepted < read;
    }
}

/// Keeps the end of stderr, where commands usually report their failure, without control characters.
pub fn stderr_detail(stderr: &[u8]) -> Option<String> {
    let sanitized = String::from_utf8_lossy(stderr)
        .chars()
        .filter(|character| !character.is_control() || matches!(character, '\n' | '\t'))
        .collect::<String>();
    let sanitized = sanitized.trim();
    if sanitized.is_empty() {
        return None;
    }

    let character_count = sanitized.chars().count();
    if character_count <= MAX_ERROR_DETAIL_CHARACTERS {
        return Some(sanitized.to_owned());
    }

    let retained = sanitized
        .chars()
        .skip(character_count - MAX_ERROR_DETAIL_CHARACTERS)
        .collect::<String>();
    Some(format!("…\n{}", retained.trim_start()))
}

/// Kills the complete process group and reaps the shell.
pub async fn terminate(child: &mut Child) {
    #[cfg(unix)]
    if let Some(process_id) = child.id() {
        // Bash pipelines can outlive the shell unless the complete process group is terminated.
        signal_process_group(process_id, libc::SIGKILL);
    }
    let _ = child.kill().await;
    let _ = child.wait().await;
}

async fn join_reader(
    reader: &mut JoinHandle<Result<BoundedOutput, std::io::Error>>,
) -> Result<BoundedOutput, ExecutionError> {
    reader
        .await
        .map_err(|error| ExecutionError::OutputRead(std::io::Error::other(error)))?
        .map_err(ExecutionError::OutputRead)
}

#[cfg(unix)]
fn signal_process_group(process_id: u32, signal: libc::c_int) {
    // A negative process ID addresses the process group that `process_group(0)` created for the shell.
    unsafe {
        libc::kill(-(process_id as i32), signal);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use assert_matches::assert_matches;

    use super::{
        CaptureLimits, ExecutionError, MAX_ERROR_DETAIL_CHARACTERS, ShellScript, Termination,
        capture, stderr_detail,
    };
    use crate::test_support::workspace_tempdir;

    #[test]
    fn bounds_and_filters_stderr_details() {
        let stderr = format!(
            "discarded\0{}\u{7}",
            "x".repeat(MAX_ERROR_DETAIL_CHARACTERS + 1)
        );

        let detail = stderr_detail(stderr.as_bytes()).unwrap();

        assert!(detail.starts_with("…\n"));
        assert_eq!(detail.chars().count(), MAX_ERROR_DETAIL_CHARACTERS + 2);
        assert!(
            detail
                .chars()
                .all(|character| { !character.is_control() || matches!(character, '\n' | '\t') })
        );
    }

    #[tokio::test]
    async fn captures_output_with_positional_arguments_in_the_working_directory() {
        let directory = workspace_tempdir();
        let script = ShellScript {
            arguments: vec!["value; with spaces".to_owned()],
            script: r#"printf '%s|%s' "$1" "$(basename "$PWD")"; printf 'warning' >&2"#.to_owned(),
            shell: PathBuf::from("/bin/bash"),
            working_directory: Some(directory.path().to_owned()),
        };

        let output = capture(
            &script,
            limits(),
            Termination::Kill,
            std::future::pending::<()>(),
        )
        .await
        .unwrap();

        let directory_name = directory.path().file_name().unwrap().to_str().unwrap();
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("value; with spaces|{directory_name}")
        );
        assert_eq!(output.stderr, b"warning");
    }

    #[tokio::test]
    async fn interrupts_a_cancelled_command_and_waits_for_it_to_finish() {
        let directory = workspace_tempdir();
        let marker = directory.path().join("interrupted");
        let script = ShellScript {
            arguments: vec![marker.display().to_string()],
            script: r#"trap 'touch "$1"; exit 0' INT; while true; do sleep 0.05; done"#.to_owned(),
            shell: PathBuf::from("/bin/bash"),
            working_directory: None,
        };

        let started = Instant::now();
        let result = capture(
            &script,
            limits(),
            Termination::Interrupt,
            tokio::time::sleep(Duration::from_millis(200)),
        )
        .await;

        assert_matches!(result, Err(ExecutionError::Cancelled));
        assert!(marker.exists());
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[tokio::test]
    async fn kills_a_command_that_exceeds_its_timeout() {
        let script = ShellScript {
            arguments: Vec::new(),
            script: "sleep 30".to_owned(),
            shell: PathBuf::from("/bin/bash"),
            working_directory: None,
        };

        let result = capture(
            &script,
            CaptureLimits {
                max_output_bytes: 1024,
                timeout: Duration::from_millis(100),
            },
            Termination::Kill,
            std::future::pending::<()>(),
        )
        .await;

        assert_matches!(result, Err(ExecutionError::Timeout));
    }

    #[tokio::test]
    async fn rejects_output_beyond_the_shared_limit() {
        let script = ShellScript {
            arguments: Vec::new(),
            script: "printf '12345'; printf '67890' >&2".to_owned(),
            shell: PathBuf::from("/bin/bash"),
            working_directory: None,
        };

        let result = capture(
            &script,
            CaptureLimits {
                max_output_bytes: 8,
                timeout: Duration::from_secs(5),
            },
            Termination::Kill,
            std::future::pending::<()>(),
        )
        .await;

        assert_matches!(result, Err(ExecutionError::OutputLimit));
    }

    fn limits() -> CaptureLimits {
        CaptureLimits {
            max_output_bytes: 1024,
            timeout: Duration::from_secs(5),
        }
    }
}
