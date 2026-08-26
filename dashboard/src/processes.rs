use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::{Child, Command};
use tokio::sync::{Notify, mpsc, oneshot};

use crate::buttons::ResolvedCommand;
use crate::connections::ConnectionHub;
use crate::messages::{AutocompleteSnapshot, AutocompleteStatus, RunSnapshot, RunStatus};

const OUTPUT_CHUNK_BYTES: usize = 8 * 1024;
const OUTPUT_LIMIT_MESSAGE: &str = "\nOutput limit reached.";
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(20);

pub struct ProcessService {
    autocompletes: Arc<Mutex<HashMap<AutocompleteKey, ActiveAutocomplete>>>,
    connections: Arc<ConnectionHub>,
    gate: Arc<ProcessGate>,
    next_run_id: AtomicU64,
    runs: Arc<Mutex<HashMap<String, ActiveRun>>>,
}

pub struct AutocompleteInvocation {
    pub arguments: Vec<String>,
    pub max_concurrent_commands: usize,
    pub max_output_bytes: usize,
    pub script: String,
    pub shell: PathBuf,
    pub timeout: Duration,
}

struct ActiveAutocomplete {
    autocomplete_id: String,
    cancel: oneshot::Sender<()>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct AutocompleteKey {
    connection_id: u64,
    editor_id: String,
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

    pub fn cancel_autocomplete(
        &self,
        connection_id: u64,
        editor_id: &str,
    ) -> Result<(), ProcessError> {
        let key = AutocompleteKey {
            connection_id,
            editor_id: editor_id.to_owned(),
        };
        if let Some(active) = self
            .autocompletes
            .lock()
            .map_err(|_| ProcessError::Lock)?
            .remove(&key)
        {
            let _ = active.cancel.send(());
        }
        Ok(())
    }

    pub fn cancel_autocompletes(&self, connection_id: u64) {
        let Ok(mut autocompletes) = self.autocompletes.lock() else {
            return;
        };
        let keys = autocompletes
            .keys()
            .filter(|key| key.connection_id == connection_id)
            .cloned()
            .collect::<Vec<_>>();
        for key in keys {
            if let Some(active) = autocompletes.remove(&key) {
                let _ = active.cancel.send(());
            }
        }
    }

    pub fn new(connections: Arc<ConnectionHub>) -> Arc<Self> {
        Arc::new(Self {
            autocompletes: Arc::new(Mutex::new(HashMap::new())),
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

    pub fn start_autocomplete(
        &self,
        connection_id: u64,
        editor_id: String,
        autocomplete_id: String,
        invocation: AutocompleteInvocation,
    ) -> Result<(), ProcessError> {
        let key = AutocompleteKey {
            connection_id,
            editor_id: editor_id.clone(),
        };
        let (cancel, cancel_receiver) = oneshot::channel();
        let mut autocompletes = self.autocompletes.lock().map_err(|_| ProcessError::Lock)?;
        if let Some(active) = autocompletes.remove(&key) {
            let _ = active.cancel.send(());
        }
        autocompletes.insert(
            key.clone(),
            ActiveAutocomplete {
                autocomplete_id: autocomplete_id.clone(),
                cancel,
            },
        );
        drop(autocompletes);

        let active_autocompletes = self.autocompletes.clone();
        let connections = self.connections.clone();
        let gate = self.gate.clone();
        gate.set_limit(invocation.max_concurrent_commands);
        tokio::spawn(async move {
            let outcome = run_autocomplete(invocation, gate, cancel_receiver).await;
            let current = active_autocompletes.lock().ok().and_then(|mut active| {
                let is_current = active
                    .get(&key)
                    .is_some_and(|active| active.autocomplete_id == autocomplete_id);
                is_current.then(|| active.remove(&key)).flatten()
            });
            if current.is_none() {
                return;
            }
            let (error, status, suggestion) = match outcome {
                AutocompleteOutcome::Cancelled => return,
                AutocompleteOutcome::Completed(suggestion) => {
                    (None, AutocompleteStatus::Completed, Some(suggestion))
                }
                AutocompleteOutcome::Failed(error) => {
                    (Some(error), AutocompleteStatus::Failed, None)
                }
            };
            let _ = connections.publish_autocomplete(
                connection_id,
                AutocompleteSnapshot {
                    autocomplete_id,
                    editor_id,
                    error,
                    status,
                    suggestion,
                },
            );
        });
        Ok(())
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

enum AutocompleteOutcome {
    Cancelled,
    Completed(String),
    Failed(String),
}

struct BoundedOutput {
    bytes: Vec<u8>,
    exceeded: bool,
}

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("process state lock was poisoned")]
    Lock,
    #[error("the requested run is not active")]
    InvalidRun,
}

async fn run_autocomplete(
    invocation: AutocompleteInvocation,
    gate: Arc<ProcessGate>,
    mut cancel: oneshot::Receiver<()>,
) -> AutocompleteOutcome {
    let _permit = tokio::select! {
        result = gate.acquire() => {
            match result {
                Ok(permit) => permit,
                Err(_) => {
                    return AutocompleteOutcome::Failed(
                        "The autocomplete command could not reserve process capacity.".to_owned(),
                    );
                }
            }
        }
        _ = &mut cancel => return AutocompleteOutcome::Cancelled,
    };

    let mut process = Command::new(&invocation.shell);
    process
        .args(["-c", &invocation.script, "personal-dashboard"])
        .args(&invocation.arguments)
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped());
    #[cfg(unix)]
    process.process_group(0);
    let mut child = match process.spawn() {
        Ok(child) => child,
        Err(_) => {
            return AutocompleteOutcome::Failed(
                "The configured autocomplete command could not be started.".to_owned(),
            );
        }
    };

    let remaining = Arc::new(AtomicUsize::new(invocation.max_output_bytes));
    let stderr = child
        .stderr
        .take()
        .expect("configured autocomplete stderr pipe is available");
    let stdout = child
        .stdout
        .take()
        .expect("configured autocomplete stdout pipe is available");
    let mut stderr_reader = tokio::spawn(read_bounded_output(stderr, remaining.clone()));
    let mut stdout_reader = tokio::spawn(read_bounded_output(stdout, remaining));
    let deadline = tokio::time::sleep(invocation.timeout);
    tokio::pin!(deadline);
    let status = tokio::select! {
        _ = &mut cancel => {
            terminate(&mut child).await;
            stderr_reader.abort();
            stdout_reader.abort();
            return AutocompleteOutcome::Cancelled;
        }
        _ = &mut deadline => {
            terminate(&mut child).await;
            stderr_reader.abort();
            stdout_reader.abort();
            return AutocompleteOutcome::Failed(
                "The configured autocomplete command timed out.".to_owned(),
            );
        }
        status = child.wait() => match status {
            Ok(status) => status,
            Err(_) => {
                stderr_reader.abort();
                stdout_reader.abort();
                return AutocompleteOutcome::Failed(
                    "The autocomplete command status could not be read.".to_owned(),
                );
            }
        },
    };
    let stderr = match (&mut stderr_reader).await {
        Ok(Ok(output)) => output,
        _ => {
            stdout_reader.abort();
            return AutocompleteOutcome::Failed(
                "The autocomplete command output could not be read.".to_owned(),
            );
        }
    };
    let stdout = match (&mut stdout_reader).await {
        Ok(Ok(output)) => output,
        _ => {
            return AutocompleteOutcome::Failed(
                "The autocomplete command output could not be read.".to_owned(),
            );
        }
    };
    if stderr.exceeded || stdout.exceeded {
        return AutocompleteOutcome::Failed(
            "The autocomplete command exceeded the configured output limit.".to_owned(),
        );
    }
    if !status.success() {
        return AutocompleteOutcome::Failed(
            "The configured autocomplete command failed.".to_owned(),
        );
    }
    let suggestion = String::from_utf8_lossy(&stdout.bytes).trim().to_owned();
    if suggestion.is_empty() {
        return AutocompleteOutcome::Failed(
            "The configured autocomplete command returned no suggestion.".to_owned(),
        );
    }
    AutocompleteOutcome::Completed(suggestion)
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

async fn read_bounded_output(
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

    use super::{AutocompleteInvocation, ProcessError, ProcessService};
    use crate::buttons::ResolvedCommand;
    use crate::connections::ConnectionHub;
    use crate::messages::{AutocompleteStatus, RunStatus, ServerEvent, ServerMessage};

    #[tokio::test]
    async fn completes_autocomplete_and_cancels_the_superseded_editor_process() {
        let connections = ConnectionHub::new();
        let mut connection = connections.register().unwrap();
        let service = ProcessService::new(connections);
        service
            .start_autocomplete(
                connection.id,
                "editor-1".to_owned(),
                "autocomplete-1".to_owned(),
                autocomplete_invocation("sleep 30", Vec::new()),
            )
            .unwrap();
        service
            .start_autocomplete(
                connection.id,
                "editor-1".to_owned(),
                "autocomplete-2".to_owned(),
                autocomplete_invocation(
                    "printf '%s' \"$1\"",
                    vec!["Focus on the boundary.".to_owned()],
                ),
            )
            .unwrap();

        let autocomplete = terminal_autocomplete(&mut connection.receiver).await;

        assert_eq!(autocomplete.autocomplete_id, "autocomplete-2");
        assert_eq!(autocomplete.editor_id, "editor-1");
        assert_eq!(autocomplete.status, AutocompleteStatus::Completed);
        assert_eq!(
            autocomplete.suggestion.as_deref(),
            Some("Focus on the boundary.")
        );
    }

    #[tokio::test]
    async fn fails_autocomplete_that_exceeds_its_output_limit() {
        let connections = ConnectionHub::new();
        let mut connection = connections.register().unwrap();
        let service = ProcessService::new(connections);
        let mut invocation = autocomplete_invocation("printf '%0100d' 0", Vec::new());
        invocation.max_output_bytes = 64;
        service
            .start_autocomplete(
                connection.id,
                "editor-1".to_owned(),
                "autocomplete-1".to_owned(),
                invocation,
            )
            .unwrap();

        let autocomplete = terminal_autocomplete(&mut connection.receiver).await;

        assert_eq!(autocomplete.status, AutocompleteStatus::Failed);
        assert_eq!(
            autocomplete.error.as_deref(),
            Some("The autocomplete command exceeded the configured output limit.")
        );
        assert_eq!(autocomplete.suggestion, None);
    }

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

    fn autocomplete_invocation(script: &str, arguments: Vec<String>) -> AutocompleteInvocation {
        AutocompleteInvocation {
            arguments,
            max_concurrent_commands: 1,
            max_output_bytes: 1_024,
            script: script.to_owned(),
            shell: PathBuf::from("/bin/bash"),
            timeout: Duration::from_secs(2),
        }
    }

    async fn terminal_autocomplete(
        receiver: &mut tokio::sync::mpsc::Receiver<ServerMessage>,
    ) -> crate::messages::AutocompleteSnapshot {
        loop {
            let message = tokio::time::timeout(Duration::from_secs(3), receiver.recv())
                .await
                .expect("autocomplete event timed out")
                .expect("autocomplete event channel closed");
            if let ServerMessage::Event {
                event: ServerEvent::AutocompleteUpdated { autocomplete },
                ..
            } = message
            {
                return autocomplete;
            }
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
