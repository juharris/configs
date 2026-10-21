use std::time::Duration;

use serde::Deserialize;
use tokio::sync::oneshot;

use crate::execution::{
    CaptureLimits, ExecutionError, ShellScript, Termination, capture_with_stdout, exit_status_code,
    stderr_detail,
};

const PROGRESS_INTERVAL: Duration = Duration::from_secs(10);

/// Accepts only the recovery wrapper's explicit, credential-free diagnostic events.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, untagged)]
enum RecoveryEvent {
    Error { error: String },
    Instructions { instructions: String },
    Progress { progress: String },
}

/// Retains actionable diagnostics even when the command is killed at its deadline.
#[derive(Default)]
struct RecoveryReport {
    error: Option<String>,
    instructions: Option<String>,
    line: Vec<u8>,
    progress: Option<String>,
}

impl RecoveryReport {
    fn describe(&self, detail: &str) -> String {
        let mut message = format!("chat listener recovery failed ({detail})");
        if let Some(error) = &self.error {
            message.push_str(&format!("\nReason: {error}"));
        }
        if let Some(progress) = &self.progress {
            message.push_str(&format!("\nLast recovery step: {progress}"));
        }
        match &self.instructions {
            Some(instructions) => message.push_str(&format!("\nTo fix: {instructions}")),
            None => message.push_str(
                "\nTo investigate: run the configured chat.listener.recovery command in a terminal, fix the reported problem, then restart the dashboard.\nConfigure the recovery wrapper to emit instructions for a specific repair command.",
            ),
        }
        message
    }

    fn receive(&mut self, bytes: &[u8]) {
        for part in bytes.split_inclusive(|byte| *byte == b'\n') {
            self.line.extend_from_slice(part);
            if part.ends_with(b"\n") {
                self.record_line();
            }
        }
    }

    fn record_line(&mut self) {
        // Raw tool output is never logged; wrappers must translate it into safe diagnostic events.
        let event = serde_json::from_slice::<RecoveryEvent>(&self.line);
        self.line.clear();
        match event {
            Ok(RecoveryEvent::Error { error }) => {
                let error = stderr_detail(error.as_bytes()).unwrap_or_else(|| {
                    "The recovery command reported failure without an explanation.".to_owned()
                });
                tracing::error!(message = %error, "chat listener recovery reported failure");
                self.error = Some(error);
            }
            Ok(RecoveryEvent::Instructions { instructions }) => {
                if let Some(instructions) = stderr_detail(instructions.as_bytes()) {
                    tracing::warn!(message = %instructions, "chat listener repair instructions");
                    self.instructions = Some(instructions);
                }
            }
            Ok(RecoveryEvent::Progress { progress }) => {
                if let Some(progress) = stderr_detail(progress.as_bytes()) {
                    tracing::warn!(message = %progress, "chat listener recovery progress");
                    self.progress = Some(progress);
                }
            }
            Err(_) => {}
        }
    }
}

/// Streams safe progress, reports elapsed time, and preserves repair instructions on every failure.
pub async fn recover(
    script: &ShellScript,
    limits: CaptureLimits,
) -> Result<Option<String>, String> {
    tracing::warn!(
        timeout_seconds = limits.timeout.as_secs(),
        "starting chat listener recovery; startup waits for repair and reconnection"
    );
    let mut report = RecoveryReport::default();
    let (failure, failed) = oneshot::channel();
    let mut failure = Some(failure);
    let started = tokio::time::Instant::now();
    let mut heartbeat = tokio::time::interval_at(started + PROGRESS_INTERVAL, PROGRESS_INTERVAL);
    let result = {
        let command = capture_with_stdout(script, limits, Termination::Kill, failed, |bytes| {
            report.receive(bytes);
            if report.error.is_some()
                && let Some(failure) = failure.take()
            {
                let _ = failure.send(());
            }
        });
        tokio::pin!(command);
        loop {
            tokio::select! {
                result = &mut command => break result,
                _ = heartbeat.tick() => {
                    let elapsed = started.elapsed();
                    tracing::warn!(
                        elapsed_seconds = elapsed.as_secs(),
                        remaining_seconds = limits.timeout.saturating_sub(elapsed).as_secs(),
                        "chat listener recovery still running; follow the repair instructions above"
                    );
                }
            }
        }
    };
    if !report.line.is_empty() {
        report.record_line();
    }
    let detail = match result {
        _ if report.error.is_some() => "recovery reported failure".to_owned(),
        Ok(output) if output.status.success() => {
            tracing::warn!("chat listener recovery completed; verifying the connection next");
            return Ok(report.instructions);
        }
        Ok(output) => format!("exit status {}", exit_status_code(output.status)),
        Err(ExecutionError::Timeout) => format!(
            "timed out after {} seconds; the recovery process was stopped",
            limits.timeout.as_secs()
        ),
        Err(ExecutionError::Start(error)) => format!("could not start recovery: {error}"),
        Err(error) => error.category().to_owned(),
    };
    Err(report.describe(&detail))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use super::recover;
    use crate::execution::{CaptureLimits, ShellScript};

    #[tokio::test]
    async fn preserves_instructions_and_progress_on_failure_and_timeout_without_raw_output() {
        for (command, detail) in [
            ("exit 7", "exit status 7"),
            ("sleep 30", "timed out after"),
            (
                r#"printf '%s\n' '{"error":"Login window was closed."}'; sleep 30"#,
                "Login window was closed.",
            ),
        ] {
            let script = script(&format!(
                r#"printf '%s\n' 'secret-token' '{{"unexpected":"secret-cookie"}}' '{{"progress":"secret-token","unexpected":true}}' '{{"progress":"secret-token","error":"secret-cookie"}}' '{{"instructions":"Run chat-tool authenticate, then restart."}}' '{{"progress":"Waiting for login."}}'; printf secret-cookie >&2; {command}"#,
            ));
            let error = recover(&script, limits()).await.unwrap_err();

            assert!(error.contains(detail), "{error}");
            assert!(error.contains("Waiting for login."), "{error}");
            assert!(
                error.contains("Run chat-tool authenticate, then restart."),
                "{error}"
            );
            assert!(!error.contains("secret-"), "{error}");
        }
    }

    #[tokio::test]
    async fn recognizes_fragmented_events_and_rejects_reported_errors_on_successful_exit() {
        let script = script(
            r#"printf '%s' '{"instructions":"Run '; sleep 0.02; printf '%s\n' 'chat-tool authenticate."}' '{"error":"Chrome is missing."}'"#,
        );
        let error = recover(&script, limits()).await.unwrap_err();
        assert!(error.contains("Chrome is missing."), "{error}");
        assert!(error.contains("Run chat-tool authenticate."), "{error}");
    }

    #[tokio::test]
    async fn retains_repair_instructions_after_success_and_explains_silent_failure() {
        let instructions = recover(
            &script(r#"printf '%s\n' '{"instructions":"Run chat-tool authenticate."}'"#),
            limits(),
        )
        .await
        .unwrap();
        assert_eq!(instructions.as_deref(), Some("Run chat-tool authenticate."));

        let error = recover(&script("exit 9"), limits()).await.unwrap_err();
        assert!(
            error.contains("chat.listener.recovery command in a terminal"),
            "{error}"
        );
    }

    fn limits() -> CaptureLimits {
        CaptureLimits {
            max_output_bytes: 4096,
            timeout: Duration::from_millis(200),
        }
    }

    fn script(command: &str) -> ShellScript {
        ShellScript {
            arguments: Vec::new(),
            script: command.to_owned(),
            shell: PathBuf::from("/bin/bash"),
            working_directory: None,
        }
    }
}
