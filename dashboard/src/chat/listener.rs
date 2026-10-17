use std::process::{ExitStatus, Stdio};

use serde::Deserialize;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::sync::mpsc;

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
    fn parse(line: &[u8]) -> Option<Self> {
        let message = serde_json::from_slice::<Self>(line).ok()?;
        let has_blank_id = message.id.trim().is_empty()
            || message
                .thread
                .as_deref()
                .is_some_and(|thread| thread.trim().is_empty());
        (!has_blank_id).then_some(message)
    }
}

enum Line {
    Complete,
    End,
    TooLong,
}

/// Runs the configured listener until it exits and forwards each valid message it prints.
pub async fn listen(
    script: &ShellScript,
    max_line_bytes: usize,
    messages: &mpsc::Sender<ChatMessage>,
) -> Result<ExitStatus, ExecutionError> {
    let mut command = script.command();
    // Listener diagnostics can contain credentials, so they are discarded like other raw output.
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
    loop {
        match read_line(&mut reader, &mut line, max_line_bytes)
            .await
            .map_err(ExecutionError::OutputRead)?
        {
            Line::Complete if line.trim_ascii().is_empty() => {}
            Line::Complete => match ChatMessage::parse(&line) {
                Some(message) => {
                    if messages.send(message).await.is_err() {
                        break;
                    }
                }
                None => tracing::warn!("chat listener printed a line that is not a valid message"),
            },
            Line::End => break,
            Line::TooLong => tracing::warn!(
                max_line_bytes,
                "chat listener printed a line longer than the output limit"
            ),
        }
    }
    child.wait().await.map_err(ExecutionError::Status)
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

    use tokio::sync::mpsc;

    use super::{ChatMessage, listen};
    use crate::execution::ShellScript;

    #[tokio::test]
    async fn forwards_valid_messages_and_skips_invalid_or_oversized_lines() {
        let script = ShellScript {
            arguments: Vec::new(),
            script: [
                r#"printf '%s\n' '{"id": "c1", "text": "/ai hello", "thread": null}'"#,
                r#"printf '%s\n' 'not json'"#,
                r#"printf '%s\n' '{"id": "m1", "text": "x", "thread": "c1", "extra": 1}'"#,
                r#"printf '%s\n' '{"id": " ", "text": "blank id", "thread": null}'"#,
                r#"printf '{"id": "m2", "text": "%0200d", "thread": "c1"}\n' 0"#,
                "printf '\\n'",
                r#"printf '%s\n' '{"id": "m3", "text": "after", "thread": "c1"}'"#,
                "exit 3",
            ]
            .join("\n"),
            shell: PathBuf::from("/bin/bash"),
            working_directory: None,
        };
        let (sender, mut receiver) = mpsc::channel(8);

        let status = listen(&script, 128, &sender).await.unwrap();
        drop(sender);

        assert_eq!(status.code(), Some(3));
        let mut messages = Vec::new();
        while let Some(message) = receiver.recv().await {
            messages.push(message);
        }
        assert_eq!(
            messages,
            vec![
                ChatMessage {
                    id: "c1".to_owned(),
                    text: "/ai hello".to_owned(),
                    thread: None,
                },
                ChatMessage {
                    id: "m3".to_owned(),
                    text: "after".to_owned(),
                    thread: Some("c1".to_owned()),
                },
            ]
        );
    }
}
