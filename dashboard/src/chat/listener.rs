use std::process::Stdio;
use std::time::Duration;

use serde::Deserialize;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::sync::{mpsc, watch};

use crate::execution::{ExecutionError, ProcessGroupGuard, ShellScript};

/// Is one incoming message that the configured listener prints as a line of JSON.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ChatMessage {
    /// Identifies the message and, for a message that starts a conversation, its thread.
    pub id: String,
    pub text: String,
    /// Identifies the thread that a reply belongs to, or is null for a message outside a thread.
    pub thread: Option<String>,
}

impl ChatMessage {
    fn is_valid(&self) -> bool {
        let has_blank_id = self.id.trim().is_empty()
            || self
                .thread
                .as_deref()
                .is_some_and(|thread| thread.trim().is_empty());
        !has_blank_id
    }
}

enum Line {
    Complete,
    End,
    TooLong,
}

/// Contains only controlled categories so listener diagnostics cannot leak credentials.
#[derive(Debug, thiserror::Error)]
pub enum ListenerError {
    #[error("chat listener reported {0}")]
    Reported(ListenerFailure),
    #[error("chat listener command failed: {0}")]
    Execution(#[from] ExecutionError),
    #[error("chat listener exited with status {0}")]
    Exited(String),
    #[error("chat listener printed an invalid protocol event")]
    Protocol,
    #[error("chat listener did not confirm its connection within {0} seconds")]
    StartupTimeout(u64),
}

/// Restricts failures from external commands to safe, actionable categories.
#[derive(Debug, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum ListenerFailure {
    #[error("an authentication failure; refresh the chat client's credentials")]
    Authentication,
    #[error("a connection failure; check the chat service and network")]
    Connection,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ListenerEvent {
    Message(ChatMessage),
    Ready(ReadyEvent),
    Failed(FailureEvent),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadyEvent {
    ready: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FailureEvent {
    error: ListenerFailure,
}

/// Waits for connection readiness before forwarding messages and fails on a broken stream.
pub async fn listen(
    script: &ShellScript,
    max_line_bytes: usize,
    startup_timeout: Duration,
    messages: &mpsc::Sender<ChatMessage>,
    readiness: &watch::Sender<bool>,
) -> Result<(), ListenerError> {
    let mut command = script.command();
    // Listener diagnostics can contain credentials, so only typed protocol failures are exposed.
    command.stderr(Stdio::null());
    let mut child = command.spawn().map_err(ExecutionError::Start)?;
    // The group also stops after a normal exit because a listener's pipeline can outlive its shell.
    let _process_group = ProcessGroupGuard::new(&child);
    let stdout = child
        .stdout
        .take()
        .expect("piped listener stdout is available");
    let mut reader = BufReader::new(stdout);
    let mut line = Vec::new();
    let deadline = tokio::time::sleep(startup_timeout);
    tokio::pin!(deadline);
    let mut connected = false;
    loop {
        let next = tokio::select! {
            next = read_line(&mut reader, &mut line, max_line_bytes) => next.map_err(ExecutionError::OutputRead)?,
            _ = &mut deadline, if !connected => return Err(ListenerError::StartupTimeout(startup_timeout.as_secs())),
        };
        match next {
            Line::Complete if line.trim_ascii().is_empty() => {}
            Line::Complete => {
                let event = serde_json::from_slice::<ListenerEvent>(&line)
                    .map_err(|_| ListenerError::Protocol)?;
                match event {
                    ListenerEvent::Ready(ReadyEvent { ready: true }) => {
                        connected = true;
                        readiness.send_replace(true);
                        tracing::info!("chat listener connected");
                    }
                    ListenerEvent::Failed(event) => {
                        return Err(ListenerError::Reported(event.error));
                    }
                    ListenerEvent::Message(message) if connected => {
                        if !message.is_valid() {
                            return Err(ListenerError::Protocol);
                        }
                        if messages.send(message).await.is_err() {
                            return Ok(());
                        }
                    }
                    _ => return Err(ListenerError::Protocol),
                }
            }
            Line::End => {
                // A descendant can close stdout while the listener shell remains stuck.
                let status = tokio::time::timeout(startup_timeout, child.wait())
                    .await
                    .map_err(|_| ListenerError::StartupTimeout(startup_timeout.as_secs()))?
                    .map_err(ExecutionError::Status)?;
                return Err(ListenerError::Exited(crate::execution::exit_status_code(
                    status,
                )));
            }
            Line::TooLong => return Err(ListenerError::Protocol),
        }
    }
}

/// Reads one line without buffering more than the limit, discarding the rest of a longer line.
async fn read_line(
    reader: &mut (impl AsyncBufRead + Unpin),
    line: &mut Vec<u8>,
    max_line_bytes: usize,
) -> std::io::Result<Line> {
    // One extra byte leaves room for the newline of a line that is exactly at the limit.
    let limit = u64::try_from(max_line_bytes)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    line.clear();
    if (&mut *reader).take(limit).read_until(b'\n', line).await? == 0 {
        return Ok(Line::End);
    }
    if line.ends_with(b"\n") || (line.len() as u64) < limit {
        return Ok(Line::Complete);
    }
    loop {
        line.clear();
        let read = (&mut *reader).take(limit).read_until(b'\n', line).await?;
        if read == 0 || line.ends_with(b"\n") {
            line.clear();
            return Ok(Line::TooLong);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use tokio::sync::{mpsc, watch};

    use super::{ListenerError, listen};
    use crate::execution::ShellScript;

    #[tokio::test]
    async fn forwards_valid_messages_after_readiness_and_reports_safe_failures() {
        let (messages, mut received) = mpsc::channel(8);
        let (ready, readiness) = watch::channel(false);
        let script = script(
            r#"printf '%s\n' '{"ready":true}' '{"id":"c1","text":"/ai hello","thread":null}' '{"error":"authentication"}'"#,
        );
        let error = listen(&script, 128, Duration::from_secs(1), &messages, &ready)
            .await
            .unwrap_err();
        assert!(*readiness.borrow());
        assert_eq!(received.recv().await.unwrap().id, "c1");
        assert!(error.to_string().contains("authentication"));
    }

    #[tokio::test]
    async fn rejects_invalid_oversized_and_premature_messages_without_echoing_them() {
        for output in [
            "not json".to_owned(),
            r#"{"id":"c1","text":"before ready","thread":null}"#.to_owned(),
            r#"{"ready":false}"#.to_owned(),
            r#"{"error":"secret-token"}"#.to_owned(),
            "x".repeat(200),
            format!(
                "{{\"ready\":true}}\n{}",
                r#"{"id":" ","text":"blank id","thread":null}"#
            ),
        ] {
            let (messages, mut received) = mpsc::channel(8);
            let (ready, _) = watch::channel(false);
            let error = listen(
                &script(&format!("printf '%s\\n' '{output}'")),
                128,
                Duration::from_secs(1),
                &messages,
                &ready,
            )
            .await
            .unwrap_err();
            assert!(matches!(error, ListenerError::Protocol));
            assert!(!error.to_string().contains("secret-token"));
            assert!(received.try_recv().is_err());
        }
    }

    #[tokio::test]
    async fn times_out_a_live_process_that_never_connects_and_rejects_clean_exit() {
        for (command, timeout) in [("sleep 30", true), ("exit 0", false)] {
            let (messages, _) = mpsc::channel(8);
            let (ready, readiness) = watch::channel(false);
            let error = listen(
                &script(command),
                128,
                Duration::from_millis(30),
                &messages,
                &ready,
            )
            .await
            .unwrap_err();
            assert_eq!(matches!(error, ListenerError::StartupTimeout(_)), timeout);
            assert!(!*readiness.borrow());
        }
    }

    fn script(source: &str) -> ShellScript {
        ShellScript {
            arguments: Vec::new(),
            script: source.to_owned(),
            shell: PathBuf::from("/bin/bash"),
            working_directory: None,
        }
    }
}
