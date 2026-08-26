use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::process::{ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::{Child, Command};
use tokio::sync::{Notify, mpsc, oneshot};

use crate::buttons::ResolvedCommand;
use crate::connections::ConnectionHub;
use crate::messages::{AutocompleteSnapshot, AutocompleteStatus, RunSnapshot, RunStatus};

// A short grace catches synchronous launcher failures without making the dashboard own the work.
const DETACHED_STARTUP_WINDOW: Duration = Duration::from_millis(100);
const OUTPUT_CHUNK_BYTES: usize = 8 * 1024;
const OUTPUT_LIMIT_MESSAGE: &str = "\nDisplay output truncated; command continues.";
const OUTPUT_QUEUE_CAPACITY: usize = 16;
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(20);
const RUN_LOG_CAPACITY: usize = 100;

pub struct ProcessService {
    autocompletes: Arc<Mutex<HashMap<AutocompleteKey, ActiveAutocomplete>>>,
    connections: Arc<ConnectionHub>,
    gate: Arc<ProcessGate>,
    latest_runs: Arc<Mutex<HashMap<String, RunSnapshot>>>,
    next_run_id: AtomicU64,
    run_logs: Arc<Mutex<VecDeque<RunSnapshot>>>,
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
    connection_id: String,
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

#[derive(Clone)]
struct RunPublisher {
    connection_id: String,
    connections: Arc<ConnectionHub>,
    latest_runs: Arc<Mutex<HashMap<String, RunSnapshot>>>,
    run_logs: Arc<Mutex<VecDeque<RunSnapshot>>>,
}

impl RunPublisher {
    fn publish(&self, snapshot: &RunSnapshot) {
        if let Ok(mut run_logs) = self.run_logs.lock() {
            record_run(&mut run_logs, snapshot);
        }
        let is_latest = self.latest_runs.lock().is_ok_and(|mut runs| {
            if runs
                .get(&self.connection_id)
                .is_some_and(|current| current.id != snapshot.id)
            {
                return false;
            }
            runs.insert(self.connection_id.clone(), snapshot.clone());
            true
        });
        if let Err(error) = self
            .connections
            .publish_run(&self.connection_id, snapshot.clone())
        {
            tracing::error!(%error, run_id = %snapshot.id, "could not publish command run");
        }
        if !is_latest {
            tracing::debug!(run_id = %snapshot.id, "published an older command run to the log");
        }
    }
}

impl ProcessService {
    pub fn cancel(&self, connection_id: &str, run_id: &str) -> Result<(), ProcessError> {
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
            latest_runs: Arc::new(Mutex::new(HashMap::new())),
            next_run_id: AtomicU64::new(1),
            run_logs: Arc::new(Mutex::new(VecDeque::with_capacity(RUN_LOG_CAPACITY))),
            runs: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn history(&self) -> Result<Vec<RunSnapshot>, ProcessError> {
        self.run_logs
            .lock()
            .map(|runs| runs.iter().rev().cloned().collect())
            .map_err(|_| ProcessError::Lock)
    }

    pub fn snapshot(&self, connection_id: &str) -> Result<Option<RunSnapshot>, ProcessError> {
        self.latest_runs
            .lock()
            .map(|runs| runs.get(connection_id).cloned())
            .map_err(|_| ProcessError::Lock)
    }

    pub fn start(&self, connection_id: &str, command: ResolvedCommand) -> RunSnapshot {
        let run_id = format!("run-{}", self.next_run_id.fetch_add(1, Ordering::Relaxed));
        let snapshot = RunSnapshot {
            created_at: current_time_milliseconds(),
            exit_code: None,
            id: run_id.clone(),
            label: command.label.clone(),
            output: String::new(),
            preview: command.preview.clone(),
            status: RunStatus::Queued,
        };
        let (cancel, cancel_receiver) = oneshot::channel();
        if let Ok(mut latest_runs) = self.latest_runs.lock() {
            latest_runs.insert(connection_id.to_owned(), snapshot.clone());
        }
        if let Ok(mut run_logs) = self.run_logs.lock() {
            record_run(&mut run_logs, &snapshot);
        }
        if let Ok(mut runs) = self.runs.lock() {
            runs.insert(
                run_id.clone(),
                ActiveRun {
                    cancel,
                    connection_id: connection_id.to_owned(),
                },
            );
        }
        let running_snapshot = snapshot.clone();
        let gate = self.gate.clone();
        let latest_runs = self.latest_runs.clone();
        let run_logs = self.run_logs.clone();
        let runs = self.runs.clone();
        let connection_id = connection_id.to_owned();
        let publisher = RunPublisher {
            connection_id,
            connections: self.connections.clone(),
            latest_runs,
            run_logs,
        };
        gate.set_limit(command.max_concurrent_commands);
        tokio::spawn(async move {
            run(command, gate, publisher, running_snapshot, cancel_receiver).await;
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
    command: ResolvedCommand,
    gate: Arc<ProcessGate>,
    publisher: RunPublisher,
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
                    publisher.publish(&snapshot);
                    return;
                }
            }
        }
        _ = &mut cancel => {
            snapshot.status = RunStatus::Cancelled;
            publisher.publish(&snapshot);
            return;
        }
    };
    snapshot.status = RunStatus::Running;
    publisher.publish(&snapshot);

    let mut process = Command::new(&command.shell);
    process
        .args(["-c", &command.script, "personal-dashboard"])
        .args(&command.arguments)
        .kill_on_drop(!command.detached)
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
            publisher.publish(&snapshot);
            return;
        }
    };

    let output_limited = Arc::new(AtomicBool::new(false));
    let (output_sender, mut output_receiver) = mpsc::channel(OUTPUT_QUEUE_CAPACITY);
    let stderr = child
        .stderr
        .take()
        .expect("configured stderr pipe is available");
    let stdout = child
        .stdout
        .take()
        .expect("configured stdout pipe is available");
    tokio::spawn(read_output(
        stderr,
        output_sender.clone(),
        output_limited.clone(),
    ));
    tokio::spawn(read_output(stdout, output_sender, output_limited.clone()));

    if command.detached {
        run_detached(
            child,
            output_receiver,
            output_limited,
            publisher,
            snapshot,
            cancel,
            command.max_output_bytes,
        )
        .await;
        return;
    }

    let deadline = tokio::time::sleep(command.timeout);
    tokio::pin!(deadline);
    let mut interval = tokio::time::interval(PROCESS_POLL_INTERVAL);
    let mut output_open = true;
    loop {
        tokio::select! {
            _ = &mut cancel => {
                terminate(&mut child).await;
                snapshot.status = RunStatus::Cancelled;
                publisher.publish(&snapshot);
                return;
            }
            _ = &mut deadline => {
                terminate(&mut child).await;
                snapshot.status = RunStatus::TimedOut;
                publisher.publish(&snapshot);
                return;
            }
            output = output_receiver.recv(), if output_open => {
                if let Some(output) = output {
                    if append_output(
                        &mut snapshot.output,
                        &output,
                        command.max_output_bytes,
                        &output_limited,
                    ) {
                        publisher.publish(&snapshot);
                    }
                } else {
                    output_open = false;
                }
            }
            _ = interval.tick() => {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        while let Some(output) = output_receiver.recv().await {
                            if append_output(
                                &mut snapshot.output,
                                &output,
                                command.max_output_bytes,
                                &output_limited,
                            ) {
                                publisher.publish(&snapshot);
                            }
                        }
                        snapshot.exit_code = status.code();
                        snapshot.status = if status.success() {
                            RunStatus::Completed
                        } else {
                            RunStatus::Failed
                        };
                        publisher.publish(&snapshot);
                        return;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        snapshot.output.push_str(&format!("\nCould not read command status: {error}"));
                        snapshot.status = RunStatus::Failed;
                        publisher.publish(&snapshot);
                        return;
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_detached(
    mut child: Child,
    mut output_receiver: mpsc::Receiver<String>,
    output_limited: Arc<AtomicBool>,
    publisher: RunPublisher,
    mut snapshot: RunSnapshot,
    mut cancel: oneshot::Receiver<()>,
    maximum_output_bytes: usize,
) {
    let startup_deadline = tokio::time::sleep(DETACHED_STARTUP_WINDOW);
    tokio::pin!(startup_deadline);
    let mut interval = tokio::time::interval(PROCESS_POLL_INTERVAL);
    let mut output_open = true;
    loop {
        tokio::select! {
            _ = &mut cancel => {
                terminate(&mut child).await;
                snapshot.status = RunStatus::Cancelled;
                publisher.publish(&snapshot);
                return;
            }
            _ = &mut startup_deadline => {
                drain_available_output(
                    &mut output_receiver,
                    &mut snapshot.output,
                    maximum_output_bytes,
                    &output_limited,
                );
                match child.try_wait() {
                    Ok(Some(status)) => finish_detached_run(
                        status,
                        &mut output_receiver,
                        &output_limited,
                        &publisher,
                        &mut snapshot,
                        maximum_output_bytes,
                    ).await,
                    Ok(None) => {
                        mark_started(&mut snapshot);
                        publisher.publish(&snapshot);
                        tokio::spawn(reap_detached(child, output_receiver, output_limited));
                    }
                    Err(error) => {
                        snapshot.output.push_str(&format!("\nCould not read command status: {error}"));
                        snapshot.status = RunStatus::Failed;
                        publisher.publish(&snapshot);
                    }
                }
                return;
            }
            output = output_receiver.recv(), if output_open => {
                if let Some(output) = output {
                    if append_output(
                            &mut snapshot.output,
                            &output,
                            maximum_output_bytes,
                            &output_limited,
                        )
                    {
                        publisher.publish(&snapshot);
                    }
                } else {
                    output_open = false;
                }
            }
            _ = interval.tick() => {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        finish_detached_run(
                            status,
                            &mut output_receiver,
                            &output_limited,
                            &publisher,
                            &mut snapshot,
                            maximum_output_bytes,
                        ).await;
                        return;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        snapshot.output.push_str(&format!("\nCould not read command status: {error}"));
                        snapshot.status = RunStatus::Failed;
                        publisher.publish(&snapshot);
                        return;
                    }
                }
            }
        }
    }
}

fn drain_available_output(
    output_receiver: &mut mpsc::Receiver<String>,
    output: &mut String,
    maximum_output_bytes: usize,
    output_limited: &AtomicBool,
) {
    while let Ok(value) = output_receiver.try_recv() {
        append_output(output, &value, maximum_output_bytes, output_limited);
    }
}

async fn finish_detached_run(
    status: ExitStatus,
    output_receiver: &mut mpsc::Receiver<String>,
    output_limited: &AtomicBool,
    publisher: &RunPublisher,
    snapshot: &mut RunSnapshot,
    maximum_output_bytes: usize,
) {
    while let Some(output) = output_receiver.recv().await {
        append_output(
            &mut snapshot.output,
            &output,
            maximum_output_bytes,
            output_limited,
        );
    }
    if status.success() {
        mark_started(snapshot);
    } else {
        snapshot.exit_code = status.code();
        snapshot.status = RunStatus::Failed;
    }
    publisher.publish(snapshot);
}

fn mark_started(snapshot: &mut RunSnapshot) {
    if snapshot.output.is_empty() {
        snapshot.output = "Command started.".to_owned();
    }
    snapshot.exit_code = None;
    snapshot.status = RunStatus::Started;
}

async fn reap_detached(
    mut child: Child,
    mut output_receiver: mpsc::Receiver<String>,
    output_limited: Arc<AtomicBool>,
) {
    output_limited.store(true, Ordering::Release);
    while output_receiver.recv().await.is_some() {}
    if let Err(error) = child.wait().await {
        tracing::debug!(%error, "could not reap detached command");
    }
}

fn current_time_milliseconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn record_run(run_logs: &mut VecDeque<RunSnapshot>, snapshot: &RunSnapshot) {
    if let Some(index) = run_logs.iter().position(|run| run.id == snapshot.id) {
        run_logs[index] = snapshot.clone();
        return;
    }
    if run_logs.len() == RUN_LOG_CAPACITY {
        run_logs.pop_front();
    }
    run_logs.push_back(snapshot.clone());
}

fn append_output(
    target: &mut String,
    value: &str,
    maximum_bytes: usize,
    output_limited: &AtomicBool,
) -> bool {
    if output_limited.load(Ordering::Acquire) {
        return false;
    }
    if !append_bounded(target, value, maximum_bytes) {
        output_limited.store(true, Ordering::Release);
    }
    true
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

async fn read_output(
    mut reader: impl AsyncRead + Unpin,
    sender: mpsc::Sender<String>,
    output_limited: Arc<AtomicBool>,
) {
    let mut buffer = [0_u8; OUTPUT_CHUNK_BYTES];
    loop {
        let Ok(read) = reader.read(&mut buffer).await else {
            return;
        };
        if read == 0 {
            return;
        }
        if output_limited.load(Ordering::Acquire) {
            continue;
        }
        if sender
            .send(String::from_utf8_lossy(&buffer[..read]).into_owned())
            .await
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
    use std::collections::VecDeque;
    use std::path::PathBuf;
    use std::time::Duration;

    use super::{
        AutocompleteInvocation, ProcessError, ProcessService, RUN_LOG_CAPACITY, record_run,
    };
    use crate::buttons::ResolvedCommand;
    use crate::connections::ConnectionHub;
    use crate::messages::{AutocompleteStatus, RunSnapshot, RunStatus, ServerEvent, ServerMessage};

    #[test]
    fn bounds_run_history_and_updates_existing_snapshots() {
        let mut run_logs = VecDeque::new();
        for index in 0..=RUN_LOG_CAPACITY {
            record_run(
                &mut run_logs,
                &RunSnapshot {
                    created_at: index as u64,
                    exit_code: None,
                    id: format!("run-{index}"),
                    label: "Test".to_owned(),
                    output: String::new(),
                    preview: format!("command {index}"),
                    status: RunStatus::Queued,
                },
            );
        }

        assert_eq!(run_logs.len(), RUN_LOG_CAPACITY);
        assert_eq!(run_logs.front().unwrap().id, "run-1");
        let mut updated = run_logs[49].clone();
        updated.output = "failure".to_owned();
        updated.status = RunStatus::Failed;
        record_run(&mut run_logs, &updated);
        assert_eq!(run_logs[49], updated);
    }

    #[tokio::test]
    async fn completes_autocomplete_and_cancels_the_superseded_editor_process() {
        let connections = ConnectionHub::new();
        let mut connection = connections.register(None).unwrap();
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
        let mut connection = connections.register(None).unwrap();
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
        let mut connection = connections.register(None).unwrap();
        let service = ProcessService::new(connections);
        let run = service.start(&connection.connection_id, command("printf once", 1_024));

        let completed = terminal_run(&mut connection.receiver).await;

        assert_eq!(completed.id, run.id);
        assert_eq!(completed.output, "once");
        assert_eq!(completed.status, RunStatus::Completed);
    }

    #[tokio::test]
    async fn bounds_output_without_stopping_the_process_and_preserves_run_ownership() {
        let connections = ConnectionHub::new();
        let mut connection = connections.register(None).unwrap();
        let service = ProcessService::new(connections);
        let limited = service.start(
            &connection.connection_id,
            command("printf '%0100d' 0; exit 0", 64),
        );

        let completed = terminal_run(&mut connection.receiver).await;

        assert_eq!(completed.id, limited.id);
        assert!(completed.output.len() <= 64);
        assert!(
            completed
                .output
                .ends_with("Display output truncated; command continues.")
        );
        assert_eq!(completed.exit_code, Some(0));
        assert_eq!(completed.status, RunStatus::Completed);

        let cancellable = service.start(&connection.connection_id, command("sleep 30", 1_024));
        assert!(matches!(
            service.cancel("connection-other", &cancellable.id),
            Err(ProcessError::InvalidRun)
        ));
        service
            .cancel(&connection.connection_id, &cancellable.id)
            .unwrap();
        let cancelled = terminal_run(&mut connection.receiver).await;
        assert_eq!(cancelled.id, cancellable.id);
        assert_eq!(cancelled.status, RunStatus::Cancelled);
    }

    #[tokio::test]
    async fn continues_a_run_across_connection_replacement() {
        let connections = ConnectionHub::new();
        let connection = connections.register(None).unwrap();
        let connection_id = connection.connection_id.clone();
        let service = ProcessService::new(connections.clone());
        let run = service.start(
            &connection_id,
            command("sleep 0.1; printf reconnected", 1_024),
        );
        connections.unregister(connection.id);
        let mut reconnected = connections.register(Some(connection_id.clone())).unwrap();

        let completed = terminal_run(&mut reconnected.receiver).await;

        assert_eq!(completed.id, run.id);
        assert_eq!(completed.output, "reconnected");
        assert_eq!(completed.status, RunStatus::Completed);
        assert_eq!(service.snapshot(&connection_id).unwrap(), Some(completed));
    }

    #[tokio::test]
    async fn detaches_after_startup_and_reports_immediate_failure() {
        let connections = ConnectionHub::new();
        let mut connection = connections.register(None).unwrap();
        let service = ProcessService::new(connections);
        let directory =
            tempfile::tempdir_in(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target")).unwrap();
        let marker = directory.path().join("detached-finished");
        let mut detached = command("sleep 0.25; printf finished > \"$1\"", 1_024);
        detached.arguments = vec![marker.display().to_string()];
        detached.detached = true;

        let run = service.start(&connection.connection_id, detached);
        let started = terminal_run(&mut connection.receiver).await;

        assert_eq!(started.id, run.id);
        assert_eq!(started.output, "Command started.");
        assert_eq!(started.status, RunStatus::Started);
        assert!(matches!(
            service.cancel(&connection.connection_id, &run.id),
            Err(ProcessError::InvalidRun)
        ));
        tokio::time::timeout(Duration::from_secs(3), async {
            while !marker.exists() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("detached command did not continue after startup");

        let mut failed = command("printf failure >&2; exit 23", 1_024);
        failed.detached = true;
        let failed_run = service.start(&connection.connection_id, failed);
        let failure = terminal_run(&mut connection.receiver).await;

        assert_eq!(failure.id, failed_run.id);
        assert_eq!(failure.exit_code, Some(23));
        assert_eq!(failure.output, "failure");
        assert_eq!(failure.status, RunStatus::Failed);
        let history = service.history().unwrap();
        assert_eq!(history.first(), Some(&failure));
        assert_eq!(
            history.first().unwrap().preview,
            "printf failure >&2; exit 23"
        );
    }

    fn command(script: &str, max_output_bytes: usize) -> ResolvedCommand {
        ResolvedCommand {
            arguments: Vec::new(),
            detached: false,
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
                        | RunStatus::Started
                        | RunStatus::TimedOut
                )
            {
                return run;
            }
        }
    }
}
