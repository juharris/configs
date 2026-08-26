use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::{Child, Command};
use tokio::sync::{Notify, mpsc, oneshot};

use crate::buttons::ResolvedCommand;
use crate::connections::ConnectionHub;
use crate::messages::{RunSnapshot, RunStatus};

const OUTPUT_CHUNK_BYTES: usize = 8 * 1024;
const OUTPUT_LIMIT_MESSAGE: &str = "\nOutput limit reached.";
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(20);

pub struct ProcessService {
    connections: Arc<ConnectionHub>,
    gate: Arc<ProcessGate>,
    next_run_id: AtomicU64,
    runs: Arc<Mutex<HashMap<String, ActiveRun>>>,
}

struct ActiveRun {
    cancel: oneshot::Sender<()>,
    connection_id: u64,
}

struct GateState {
    active: usize,
    limit: usize,
}

struct ProcessGate {
    notify: Notify,
    state: Mutex<GateState>,
}

struct ProcessPermit {
    gate: Arc<ProcessGate>,
}

impl ProcessService {
    pub fn new(connections: Arc<ConnectionHub>) -> Arc<Self> {
        Arc::new(Self {
            connections,
            gate: Arc::new(ProcessGate {
                notify: Notify::new(),
                state: Mutex::new(GateState {
                    active: 0,
                    limit: 1,
                }),
            }),
            next_run_id: AtomicU64::new(1),
            runs: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn cancel(&self, connection_id: u64, run_id: &str) -> Result<(), ProcessError> {
        let mut runs = self.runs.lock().map_err(|_| ProcessError::Lock)?;
        if runs
            .get(run_id)
            .is_none_or(|active| active.connection_id != connection_id)
        {
            return Err(ProcessError::InvalidRun);
        }
        let active = runs
            .remove(run_id)
            .expect("run ownership was checked before removal");
        drop(runs);
        let _ = active.cancel.send(());
        Ok(())
    }

    pub fn start(&self, connection_id: u64, command: ResolvedCommand) -> RunSnapshot {
        let run_id = format!("run-{}", self.next_run_id.fetch_add(1, Ordering::Relaxed));
        let snapshot = RunSnapshot {
            exit_code: None,
            id: run_id.clone(),
            label: command.label.clone(),
            output: String::new(),
            preview: command.preview.clone(),
            status: RunStatus::Queued,
        };
        let (cancel, cancel_receiver) = oneshot::channel();
        if let Ok(mut runs) = self.runs.lock() {
            runs.insert(
                run_id.clone(),
                ActiveRun {
                    cancel,
                    connection_id,
                },
            );
        }
        let running_snapshot = snapshot.clone();
        let connections = self.connections.clone();
        let gate = self.gate.clone();
        let runs = self.runs.clone();
        gate.set_limit(command.max_concurrent_commands);
        tokio::spawn(async move {
            run(
                connections,
                connection_id,
                command,
                gate,
                running_snapshot,
                cancel_receiver,
            )
            .await;
            if let Ok(mut runs) = runs.lock() {
                runs.remove(&run_id);
            }
        });
        snapshot
    }
}

impl ProcessGate {
    async fn acquire(self: &Arc<Self>) -> Result<ProcessPermit, ProcessError> {
        loop {
            let notified = self.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            {
                let mut state = self.state.lock().map_err(|_| ProcessError::Lock)?;
                if state.active < state.limit {
                    state.active += 1;
                    return Ok(ProcessPermit { gate: self.clone() });
                }
            }
            notified.await;
        }
    }

    fn set_limit(&self, limit: usize) {
        if let Ok(mut state) = self.state.lock() {
            state.limit = limit;
        }
        self.notify.notify_waiters();
    }
}

impl Drop for ProcessPermit {
    fn drop(&mut self) {
        if let Ok(mut state) = self.gate.state.lock() {
            state.active = state.active.saturating_sub(1);
        }
        self.gate.notify.notify_one();
    }
}

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("process state lock was poisoned")]
    Lock,
    #[error("the requested run is not active")]
    InvalidRun,
}

async fn run(
    connections: Arc<ConnectionHub>,
    connection_id: u64,
    command: ResolvedCommand,
    gate: Arc<ProcessGate>,
    mut snapshot: RunSnapshot,
    mut cancel: oneshot::Receiver<()>,
) {
    let _permit = tokio::select! {
        result = gate.acquire() => {
            match result {
                Ok(permit) => permit,
                Err(error) => {
                    snapshot.output = error.to_string();
                    snapshot.status = RunStatus::Failed;
                    publish(&connections, connection_id, &snapshot);
                    return;
                }
            }
        }
        _ = &mut cancel => {
            snapshot.status = RunStatus::Cancelled;
            publish(&connections, connection_id, &snapshot);
            return;
        }
    };
    snapshot.status = RunStatus::Running;
    publish(&connections, connection_id, &snapshot);

    let mut process = Command::new(&command.shell);
    process
        .args(["-c", &command.script, "personal-dashboard"])
        .args(&command.arguments)
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped());
    #[cfg(unix)]
    process.process_group(0);
    let mut child = match process.spawn() {
        Ok(child) => child,
        Err(error) => {
            snapshot.output = format!("Could not start the configured command: {error}");
            snapshot.status = RunStatus::Failed;
            publish(&connections, connection_id, &snapshot);
            return;
        }
    };

    let (output_sender, mut output_receiver) = mpsc::unbounded_channel();
    let stderr = child
        .stderr
        .take()
        .expect("configured stderr pipe is available");
    let stdout = child
        .stdout
        .take()
        .expect("configured stdout pipe is available");
    tokio::spawn(read_output(stderr, output_sender.clone()));
    tokio::spawn(read_output(stdout, output_sender));

    let deadline = tokio::time::sleep(command.timeout);
    tokio::pin!(deadline);
    let mut interval = tokio::time::interval(PROCESS_POLL_INTERVAL);
    loop {
        tokio::select! {
            _ = &mut cancel => {
                terminate(&mut child).await;
                snapshot.status = RunStatus::Cancelled;
                publish(&connections, connection_id, &snapshot);
                return;
            }
            _ = &mut deadline => {
                terminate(&mut child).await;
                snapshot.status = RunStatus::TimedOut;
                publish(&connections, connection_id, &snapshot);
                return;
            }
            output = output_receiver.recv() => {
                if let Some(output) = output {
                    if !append_bounded(
                        &mut snapshot.output,
                        &output,
                        command.max_output_bytes,
                    ) {
                        terminate(&mut child).await;
                        snapshot.status = RunStatus::Failed;
                        publish(&connections, connection_id, &snapshot);
                        return;
                    }
                    publish(&connections, connection_id, &snapshot);
                }
            }
            _ = interval.tick() => {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        let mut output_limited = false;
                        while let Some(output) = output_receiver.recv().await {
                            if !append_bounded(
                                &mut snapshot.output,
                                &output,
                                command.max_output_bytes,
                            ) {
                                output_limited = true;
                                break;
                            }
                        }
                        snapshot.exit_code = status.code();
                        snapshot.status = if output_limited {
                            RunStatus::Failed
                        } else if status.success() {
                            RunStatus::Completed
                        } else {
                            RunStatus::Failed
                        };
                        publish(&connections, connection_id, &snapshot);
                        return;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        snapshot.output.push_str(&format!("\nCould not read command status: {error}"));
                        snapshot.status = RunStatus::Failed;
                        publish(&connections, connection_id, &snapshot);
                        return;
                    }
                }
            }
        }
    }
}

fn append_bounded(target: &mut String, value: &str, maximum_bytes: usize) -> bool {
    let remaining = maximum_bytes.saturating_sub(target.len());
    if value.len() <= remaining {
        target.push_str(value);
        return true;
    }
    let content_limit = maximum_bytes.saturating_sub(OUTPUT_LIMIT_MESSAGE.len());
    if target.len() > content_limit {
        truncate_at_character_boundary(target, content_limit);
    }
    let remaining = content_limit.saturating_sub(target.len());
    let mut end = remaining.min(value.len());
    while !value.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    target.push_str(&value[..end]);
    let marker_limit = maximum_bytes.saturating_sub(target.len());
    let mut marker_end = marker_limit.min(OUTPUT_LIMIT_MESSAGE.len());
    while !OUTPUT_LIMIT_MESSAGE.is_char_boundary(marker_end) {
        marker_end = marker_end.saturating_sub(1);
    }
    target.push_str(&OUTPUT_LIMIT_MESSAGE[..marker_end]);
    false
}

fn truncate_at_character_boundary(value: &mut String, maximum_bytes: usize) {
    let mut end = maximum_bytes.min(value.len());
    while !value.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    value.truncate(end);
}

fn publish(connections: &ConnectionHub, connection_id: u64, snapshot: &RunSnapshot) {
    if let Err(error) = connections.publish_run(connection_id, snapshot.clone()) {
        tracing::error!(%error, run_id = %snapshot.id, "could not publish command run");
    }
}

async fn read_output(mut reader: impl AsyncRead + Unpin, sender: mpsc::UnboundedSender<String>) {
    let mut buffer = [0_u8; OUTPUT_CHUNK_BYTES];
    loop {
        let Ok(read) = reader.read(&mut buffer).await else {
            return;
        };
        if read == 0 {
            return;
        }
        if sender
            .send(String::from_utf8_lossy(&buffer[..read]).into_owned())
            .is_err()
        {
            return;
        }
    }
}

async fn terminate(child: &mut Child) {
    #[cfg(unix)]
    if let Some(process_id) = child.id() {
        unsafe {
            libc::kill(-(process_id as i32), libc::SIGKILL);
        }
    }
    let _ = child.kill().await;
    let _ = child.wait().await;
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use super::{ProcessError, ProcessService};
    use crate::buttons::ResolvedCommand;
    use crate::connections::ConnectionHub;
    use crate::messages::{RunStatus, ServerEvent, ServerMessage};

    #[tokio::test]
    async fn streams_output_once_and_reports_completion() {
        let connections = ConnectionHub::new();
        let mut connection = connections.register().unwrap();
        let service = ProcessService::new(connections);
        let run = service.start(connection.id, command("printf once", 1_024));

        let completed = terminal_run(&mut connection.receiver).await;

        assert_eq!(completed.id, run.id);
        assert_eq!(completed.output, "once");
        assert_eq!(completed.status, RunStatus::Completed);
    }

    #[tokio::test]
    async fn bounds_output_and_preserves_run_ownership_on_invalid_cancellation() {
        let connections = ConnectionHub::new();
        let mut connection = connections.register().unwrap();
        let service = ProcessService::new(connections);
        let limited = service.start(connection.id, command("printf '%0100d' 0", 64));

        let completed = terminal_run(&mut connection.receiver).await;

        assert_eq!(completed.id, limited.id);
        assert!(completed.output.len() <= 64);
        assert!(completed.output.ends_with("Output limit reached."));
        assert_eq!(completed.status, RunStatus::Failed);

        let cancellable = service.start(connection.id, command("sleep 30", 1_024));
        assert!(matches!(
            service.cancel(connection.id + 1, &cancellable.id),
            Err(ProcessError::InvalidRun)
        ));
        service.cancel(connection.id, &cancellable.id).unwrap();
        let cancelled = terminal_run(&mut connection.receiver).await;
        assert_eq!(cancelled.id, cancellable.id);
        assert_eq!(cancelled.status, RunStatus::Cancelled);
    }

    fn command(script: &str, max_output_bytes: usize) -> ResolvedCommand {
        ResolvedCommand {
            arguments: Vec::new(),
            label: "Test".to_owned(),
            max_concurrent_commands: 1,
            max_output_bytes,
            preview: script.to_owned(),
            script: script.to_owned(),
            shell: PathBuf::from("/bin/bash"),
            timeout: Duration::from_secs(2),
        }
    }

    async fn terminal_run(
        receiver: &mut tokio::sync::mpsc::Receiver<ServerMessage>,
    ) -> crate::messages::RunSnapshot {
        loop {
            let message = tokio::time::timeout(Duration::from_secs(3), receiver.recv())
                .await
                .expect("run event timed out")
                .expect("run event channel closed");
            if let ServerMessage::Event {
                event: ServerEvent::RunUpdated { run },
                ..
            } = message
                && matches!(
                    run.status,
                    RunStatus::Cancelled
                        | RunStatus::Completed
                        | RunStatus::Failed
                        | RunStatus::TimedOut
                )
            {
                return run;
            }
        }
    }
}
