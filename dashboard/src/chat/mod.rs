mod conversations;
mod listener;
mod recovery;
mod requests;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;

use conversations::{ChatEffect, Conversations, TurnOutcome, TurnRequest};
use listener::ChatMessage;

use crate::commands::{CommandTemplate, TemplateError, TemplateValues};
use crate::config::{ConfigService, ConfigurationSnapshot, ValidatedChat, ValidatedRootConfig};
use crate::execution::{
    CaptureLimits, ExecutionError, ShellScript, Termination, capture, exit_status_code,
    stderr_detail,
};

const MESSAGE_QUEUE_CAPACITY: usize = 64;

/// Connects configured chat commands to coding-agent turns for as long as the dashboard runs.
///
/// The runtime verifies the configured connection and propagates unrecoverable failures to the server.
pub struct ChatRuntime {
    configuration_receiver: watch::Receiver<Option<ConfigurationSnapshot>>,
    readiness: watch::Sender<bool>,
}

impl ChatRuntime {
    pub fn new(config_service: &ConfigService) -> Self {
        Self {
            configuration_receiver: config_service.subscribe(),
            readiness: watch::channel(false).0,
        }
    }

    pub fn readiness(&self) -> watch::Receiver<bool> {
        self.readiness.subscribe()
    }

    /// Starts the runtime and waits for the configured listener's verified connection.
    pub async fn start(self) -> Result<ChatTask, String> {
        let mut readiness = self.readiness();
        let mut task = ChatTask {
            handle: tokio::spawn(self.run()),
        };
        while !*readiness.borrow() {
            tokio::select! {
                result = task.wait() => {
                    result?;
                    return Err("chat runtime stopped before startup completed".to_owned());
                },
                result = readiness.changed() => {
                    if result.is_err() {
                        task.wait().await?;
                        return Err("chat readiness channel closed".to_owned());
                    }
                }
            }
        }
        Ok(task)
    }

    pub async fn run(mut self) -> Result<(), String> {
        let (delivery_sender, delivery_receiver) = mpsc::unbounded_channel();
        let (message_sender, mut message_receiver) = mpsc::channel(MESSAGE_QUEUE_CAPACITY);
        let (turn_sender, mut turn_receiver) = mpsc::unbounded_channel();
        tokio::spawn(deliver(delivery_receiver));
        let mut state = ChatState {
            conversations: Conversations::default(),
            deliveries: delivery_sender,
            readiness: self.readiness,
            listener: None,
            messages: message_sender,
            settings: None,
            turn_cancellations: HashMap::new(),
            turns: turn_sender,
        };
        let initial_configuration = self.configuration_receiver.borrow_and_update().clone();
        state.configure(initial_configuration.as_ref());

        loop {
            tokio::select! {
                configuration = self.configuration_receiver.changed() => {
                    if configuration.is_err() {
                        return Ok(());
                    }
                    let configuration = self.configuration_receiver.borrow_and_update().clone();
                    state.configure(configuration.as_ref());
                }
                result = wait_for_listener(&mut state.listener) => {
                    result?;
                    return Err("chat listener stopped unexpectedly".to_owned());
                }
                Some(message) = message_receiver.recv() => state.receive(message),
                Some(finished) = turn_receiver.recv() => state.finish_turn(finished),
            }
        }
    }
}

/// Stops the chat process tree when the server exits, including failed startup.
pub struct ChatTask {
    handle: JoinHandle<Result<(), String>>,
}

impl ChatTask {
    pub async fn wait(&mut self) -> Result<(), String> {
        (&mut self.handle)
            .await
            .map_err(|error| format!("chat runtime failed: {error}"))?
    }
}

impl Drop for ChatTask {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

/// Is one reply or acknowledgement command waiting to run.
struct Delivery {
    kind: &'static str,
    limits: CaptureLimits,
    script: ShellScript,
}

struct FinishedTurn {
    conversation_id: String,
    outcome: TurnOutcome,
    turn_id: u64,
}

/// Owns the supervised listener so replacing or dropping it stops the listener's process group.
struct ListenerTask {
    handle: JoinHandle<Result<(), String>>,
    settings: ListenerSettings,
}

impl Drop for ListenerTask {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ListenerSettings {
    max_line_bytes: usize,
    recovery: Option<ShellScript>,
    recovery_timeout: Duration,
    restart: Duration,
    restart_attempts: usize,
    script: ShellScript,
    startup_timeout: Duration,
}

/// Holds the validated chat configuration with the application limits that chat commands share.
struct ChatSettings {
    chat: Arc<ValidatedChat>,
    command_limits: CaptureLimits,
    shell: PathBuf,
    turn_limits: CaptureLimits,
    working_directory: PathBuf,
}

impl ChatSettings {
    fn new(configuration: &ValidatedRootConfig) -> Option<Self> {
        let chat = configuration.chat.clone()?;
        let application = &configuration.root.application;
        Some(Self {
            command_limits: CaptureLimits {
                max_output_bytes: application.max_output_bytes_per_run,
                timeout: Duration::from_secs(application.command_timeout_seconds),
            },
            shell: PathBuf::from(&application.shell),
            turn_limits: CaptureLimits {
                max_output_bytes: application.max_output_bytes_per_run,
                timeout: Duration::from_secs(chat.config.agent.timeout_seconds),
            },
            working_directory: PathBuf::from(&chat.config.working_directory),
            chat,
        })
    }

    fn listener(&self) -> ListenerSettings {
        ListenerSettings {
            max_line_bytes: self.command_limits.max_output_bytes,
            recovery: self
                .chat
                .recovery
                .as_ref()
                .map(|template| self.script(template, Vec::new())),
            recovery_timeout: Duration::from_secs(
                self.chat.config.listener.recovery_timeout_seconds,
            ),
            restart_attempts: self.chat.config.listener.restart_attempts,
            restart: Duration::from_secs(self.chat.config.listener.restart_seconds),
            startup_timeout: Duration::from_secs(self.chat.config.listener.startup_timeout_seconds),
            script: self.script(&self.chat.listener, Vec::new()),
        }
    }

    fn resolve(
        &self,
        template: &CommandTemplate,
        values: &TemplateValues,
    ) -> Result<ShellScript, TemplateError> {
        Ok(self.script(template, template.resolve_arguments(values)?))
    }

    fn script(&self, template: &CommandTemplate, arguments: Vec<String>) -> ShellScript {
        ShellScript {
            arguments,
            script: template.script().to_owned(),
            shell: self.shell.clone(),
            working_directory: Some(self.working_directory.clone()),
        }
    }
}

struct ChatState {
    conversations: Conversations,
    deliveries: mpsc::UnboundedSender<Delivery>,
    listener: Option<ListenerTask>,
    readiness: watch::Sender<bool>,
    messages: mpsc::Sender<ChatMessage>,
    settings: Option<Arc<ChatSettings>>,
    /// Interrupts a running turn when its sender is used or dropped.
    turn_cancellations: HashMap<u64, oneshot::Sender<()>>,
    turns: mpsc::UnboundedSender<FinishedTurn>,
}

impl ChatState {
    fn apply(&mut self, effects: Vec<ChatEffect>) {
        let Some(settings) = self.settings.clone() else {
            return;
        };
        for effect in effects {
            match effect {
                ChatEffect::Acknowledge {
                    conversation_id,
                    message_id,
                } => {
                    if let Some(template) = &settings.chat.acknowledgement {
                        let values = TemplateValues::for_conversation(&conversation_id)
                            .with_message_id(&message_id);
                        self.deliver(&settings, "acknowledgement", template, &values);
                    }
                }
                ChatEffect::Interrupt { turn_id } => {
                    if let Some(cancellation) = self.turn_cancellations.remove(&turn_id) {
                        let _ = cancellation.send(());
                    }
                }
                ChatEffect::Reply {
                    conversation_id,
                    text,
                } => {
                    let text = format!("{}{text}", settings.chat.config.reply.prefix);
                    let values =
                        TemplateValues::for_conversation(&conversation_id).with_reply_text(&text);
                    self.deliver(&settings, "reply", &settings.chat.reply, &values);
                }
                ChatEffect::StartTurn(request) => self.start_turn(&settings, request),
            }
        }
    }

    fn configure(&mut self, configuration: Option<&ConfigurationSnapshot>) {
        let settings = configuration
            .and_then(|configuration| ChatSettings::new(&configuration.configuration))
            .map(Arc::new);
        let Some(settings) = settings else {
            if self.settings.take().is_some() {
                tracing::info!("chat is disabled; stopping its listener and running turns");
            }
            self.conversations = Conversations::default();
            self.listener = None;
            self.readiness.send_replace(true);
            // Dropping each cancellation sender interrupts its turn.
            self.turn_cancellations.clear();
            return;
        };

        let listener = settings.listener();
        if self
            .listener
            .as_ref()
            .is_none_or(|active| active.settings != listener)
        {
            self.readiness.send_replace(false);
            self.listener = Some(ListenerTask {
                handle: tokio::spawn(supervise_listener(
                    listener.clone(),
                    self.messages.clone(),
                    self.readiness.clone(),
                )),
                settings: listener,
            });
        }
        self.settings = Some(settings);
    }

    fn deliver(
        &self,
        settings: &ChatSettings,
        kind: &'static str,
        template: &CommandTemplate,
        values: &TemplateValues,
    ) {
        match settings.resolve(template, values) {
            Ok(script) => {
                let _ = self.deliveries.send(Delivery {
                    kind,
                    limits: settings.command_limits,
                    script,
                });
            }
            Err(error) => tracing::error!(command = kind, %error, "could not resolve chat command"),
        }
    }

    fn finish_turn(&mut self, finished: FinishedTurn) {
        self.turn_cancellations.remove(&finished.turn_id);
        let effects = self.conversations.finish_turn(
            &finished.conversation_id,
            finished.turn_id,
            finished.outcome,
        );
        self.apply(effects);
    }

    fn receive(&mut self, message: ChatMessage) {
        let Some(settings) = self.settings.clone() else {
            return;
        };
        let effects = self.conversations.receive(message, &settings.chat.config);
        self.apply(effects);
    }

    fn start_turn(&mut self, settings: &ChatSettings, request: TurnRequest) {
        let template = if request.resume {
            &settings.chat.resume
        } else {
            &settings.chat.start
        };
        let values = TemplateValues::for_conversation(&request.conversation_id)
            .with_conversation_model(&request.model)
            .with_conversation_session_id(&request.session_id)
            .with_message_text(&request.text);
        let script = settings.resolve(template, &values);
        let (cancellation, cancelled) = oneshot::channel();
        self.turn_cancellations
            .insert(request.turn_id, cancellation);
        let limits = settings.turn_limits;
        let turns = self.turns.clone();
        tokio::spawn(async move {
            let started_at = Instant::now();
            let outcome = match script {
                Ok(script) => run_turn(&script, limits, cancelled).await,
                Err(error) => {
                    tracing::error!(%error, "could not resolve chat agent command");
                    TurnOutcome::Failed("The agent command could not be resolved.".to_owned())
                }
            };
            tracing::info!(
                duration_milliseconds = started_at.elapsed().as_millis(),
                outcome = outcome.category(),
                resume = request.resume,
                turn_id = request.turn_id,
                "chat turn finished"
            );
            let _ = turns.send(FinishedTurn {
                conversation_id: request.conversation_id,
                outcome,
                turn_id: request.turn_id,
            });
        });
    }
}

/// Runs replies and acknowledgements one at a time so they reach the chat in the order they were produced.
async fn deliver(mut deliveries: mpsc::UnboundedReceiver<Delivery>) {
    while let Some(delivery) = deliveries.recv().await {
        let result = capture(
            &delivery.script,
            delivery.limits,
            Termination::Kill,
            std::future::pending::<()>(),
        )
        .await;
        match result {
            Ok(output) if output.status.success() => {}
            Ok(output) => tracing::warn!(
                command = delivery.kind,
                status = %exit_status_code(output.status),
                "chat command failed"
            ),
            Err(error) => tracing::warn!(
                command = delivery.kind,
                category = error.category(),
                "chat command failed"
            ),
        }
    }
}

async fn run_turn(
    script: &ShellScript,
    limits: CaptureLimits,
    cancelled: oneshot::Receiver<()>,
) -> TurnOutcome {
    let output = match capture(script, limits, Termination::Interrupt, cancelled).await {
        Ok(output) => output,
        Err(error) => {
            let message = match error {
                ExecutionError::Cancelled => return TurnOutcome::Interrupted,
                ExecutionError::Timeout => format!(
                    "The agent command timed out after {} seconds.",
                    limits.timeout.as_secs()
                ),
                ExecutionError::OutputLimit => {
                    "The agent command exceeded the configured output limit.".to_owned()
                }
                ExecutionError::OutputRead(_) => {
                    "The agent command output could not be read.".to_owned()
                }
                ExecutionError::Start(_) => "The agent command could not be started.".to_owned(),
                ExecutionError::Status(_) => {
                    "The agent command status could not be read.".to_owned()
                }
            };
            return TurnOutcome::Failed(message);
        }
    };
    if !output.status.success() {
        let status = exit_status_code(output.status);
        return TurnOutcome::Failed(stderr_detail(&output.stderr).map_or_else(
            || format!("The agent command exited with status {status}."),
            |detail| format!("The agent command exited with status {status}.\n{detail}"),
        ));
    }
    TurnOutcome::Completed(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Attempts bounded repair and reconnection, then makes an unavailable chat service fatal.
async fn supervise_listener(
    settings: ListenerSettings,
    messages: mpsc::Sender<ChatMessage>,
    readiness: watch::Sender<bool>,
) -> Result<(), String> {
    let mut repair_instructions = None;
    for attempt in 0..=settings.restart_attempts {
        tracing::info!(
            attempt = attempt + 1,
            max_attempts = settings.restart_attempts + 1,
            timeout_seconds = settings.startup_timeout.as_secs(),
            "connecting chat listener"
        );
        let error = match listener::listen(
            &settings.script,
            settings.max_line_bytes,
            settings.startup_timeout,
            &messages,
            &readiness,
        )
        .await
        {
            Ok(()) => return Ok(()),
            Err(error) => error,
        };
        readiness.send_replace(false);
        tracing::error!(%error, "chat listener unavailable");
        if attempt == settings.restart_attempts {
            let instructions = repair_instructions
                .map(|instructions| format!("\nTo fix: {instructions}"))
                .unwrap_or_default();
            return Err(format!(
                "{error}; exhausted {} connection attempts; stopping Personal Dashboard{instructions}",
                attempt + 1
            ));
        }
        if let Some(recovery) = &settings.recovery {
            let instructions = recovery::recover(
                recovery,
                CaptureLimits {
                    max_output_bytes: settings.max_line_bytes,
                    timeout: settings.recovery_timeout,
                },
            )
            .await
            .map_err(|detail| {
                format!("{error}; {detail}\nChat is unavailable; stopping Personal Dashboard.")
            })?;
            if instructions.is_some() {
                repair_instructions = instructions;
            }
        }
        tracing::warn!(
            delay_seconds = settings.restart.as_secs(),
            next_attempt = attempt + 2,
            "retrying chat listener connection"
        );
        tokio::time::sleep(settings.restart).await;
    }
    unreachable!("the final connection attempt always returns")
}

/// Observes the active task so a supervisor panic cannot leave chat silently unavailable.
async fn wait_for_listener(listener: &mut Option<ListenerTask>) -> Result<(), String> {
    match listener {
        Some(listener) => (&mut listener.handle)
            .await
            .map_err(|error| format!("chat listener supervision failed: {error}"))?,
        None => std::future::pending().await,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::fs;
    use std::path::Path;
    use std::time::Duration;

    use serde_json::json;

    use super::ChatRuntime;
    use crate::config::{
        ChatAgentConfig, ChatCommandConfig, ChatCommandsConfig, ChatConfig, ChatListenerConfig,
        ChatModelsConfig, ChatReplyConfig, ConfigService, RuntimeSchema,
    };
    use crate::messages::OptifySetup;
    use crate::test_support::workspace_tempdir;

    /// Builds the chat configuration that the chat unit tests share.
    pub(crate) fn chat_config() -> ChatConfig {
        let command = |name: &str, description: &str| ChatCommandConfig {
            description: description.to_owned(),
            name: name.to_owned(),
        };
        ChatConfig {
            acknowledgement: None,
            agent: ChatAgentConfig {
                resume: "agent --resume {conversation.session_id} {message.text}".to_owned(),
                start: "agent --session-id {conversation.session_id} {message.text}".to_owned(),
                timeout_seconds: 60,
            },
            commands: ChatCommandsConfig {
                help: command("/help", "Show these commands."),
                model: command("/model", "Show or select the model."),
                start: command("/ai", "Start a conversation."),
                status: command("/status", "Show what is running."),
                steer: command("/steer", "Interrupt the turn and send this message next."),
                stop: command("/stop", "Stop the turn and clear the queue."),
            },
            listener: ChatListenerConfig {
                command: "listen".to_owned(),
                recovery: None,
                recovery_timeout_seconds: 1,
                restart_attempts: 1,
                restart_seconds: 1,
                startup_timeout_seconds: 1,
            },
            models: ChatModelsConfig {
                available: vec!["haiku".to_owned(), "opus".to_owned(), "sonnet".to_owned()],
                default: "opus".to_owned(),
            },
            reply: ChatReplyConfig {
                command: "reply {conversation.id} {reply.text}".to_owned(),
                prefix: ":robot_face: ".to_owned(),
            },
            working_directory: "/tmp".to_owned(),
        }
    }

    #[tokio::test]
    async fn runs_listened_prompts_as_turns_and_replies_after_steering() {
        let directory = workspace_tempdir();
        let replies = directory.path().join("replies");
        let acknowledgements = directory.path().join("acknowledgements");
        let listener = [
            r#"printf '%s\n' '{"ready":true}'"#,
            r#"printf '%s\n' '{"id": "c1", "text": "/ai hello", "thread": null}'"#,
            r#"printf '%s\n' '{"id": "c2", "text": "/ai wait", "thread": null}'"#,
            "sleep 0.5",
            r#"printf '%s\n' '{"id": "m3", "text": "/steer go left", "thread": "c2"}'"#,
            "sleep 30",
        ]
        .join("\n");
        write_configuration(
            directory.path(),
            json!({
                "acknowledgement": format!(
                    "printf '%s %s\\n' {{conversation.id}} {{message.id}} >> '{}'",
                    acknowledgements.display()
                ),
                "agent": {
                    "resume": "printf 'resume %s %s' {conversation.model} {message.text}",
                    "start": "if [ {message.text} = wait ]; then trap 'exit 0' INT; while :; do sleep 0.05; done; fi; printf 'start %s %s' {conversation.model} {message.text}",
                    "timeout_seconds": 10,
                },
                "commands": chat_config().commands,
                "listener": { "command": listener, "recovery_timeout_seconds": 1, "restart_attempts": 1, "restart_seconds": 1, "startup_timeout_seconds": 1 },
                "models": chat_config().models,
                "reply": {
                    "command": format!(
                        "printf '%s|%s\\n' {{conversation.id}} {{reply.text}} >> '{}'",
                        replies.display()
                    ),
                    "prefix": ":robot_face: ",
                },
                "working_directory": directory.path(),
            }),
        );
        let (config_service, _reload_service) =
            ConfigService::new(RuntimeSchema::materialize().unwrap());
        tokio::spawn(ChatRuntime::new(&config_service).run());

        config_service
            .apply_setup(OptifySetup {
                config_directories: vec![directory.path().display().to_string()],
                features: vec!["dashboard".to_owned()],
            })
            .unwrap();

        let mut lines = wait_for_lines(&replies, 2).await;
        lines.sort();
        assert_eq!(
            lines,
            [
                "c1|:robot_face: start opus hello",
                "c2|:robot_face: resume opus go left",
            ]
        );
        let mut acknowledged = wait_for_lines(&acknowledgements, 3).await;
        acknowledged.sort();
        assert_eq!(acknowledged, ["c1 c1", "c2 c2", "c2 m3"]);
    }

    #[tokio::test]
    async fn waits_for_connection_repairs_once_and_stops_the_process_tree_on_shutdown() {
        let directory = workspace_tempdir();
        let mut chat = serde_json::to_value(chat_config()).unwrap();
        chat["working_directory"] = json!(directory.path());
        chat["listener"]["command"] = json!(
            "echo $$ > listener.pid; if [ ! -f repaired ]; then printf '%s\\n' '{\"error\":\"authentication\"}'; else printf '%s\\n' '{\"ready\":true}'; sleep 30 & echo $! > descendant.pid; wait; fi"
        );
        chat["listener"]["recovery"] = json!("touch repaired");
        write_configuration(directory.path(), chat);
        let (service, _reload) = ConfigService::new(RuntimeSchema::materialize().unwrap());
        service
            .apply_setup(OptifySetup {
                config_directories: vec![directory.path().display().to_string()],
                features: vec!["dashboard".to_owned()],
            })
            .unwrap();
        let task = tokio::time::timeout(Duration::from_secs(5), ChatRuntime::new(&service).start())
            .await
            .unwrap()
            .unwrap();
        assert!(directory.path().join("repaired").exists());
        let pid = wait_for_lines(&directory.path().join("descendant.pid"), 1).await[0]
            .parse::<i32>()
            .unwrap();
        assert_eq!(unsafe { libc::kill(pid, 0) }, 0);
        drop(task);
        tokio::time::timeout(Duration::from_secs(5), async {
            while unsafe { libc::kill(pid, 0) } == 0 {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("listener descendant survived shutdown");
    }

    #[tokio::test]
    async fn fails_startup_after_bounded_attempts_or_failed_recovery_without_leaking_output() {
        for (recovery, expected_attempts, detail) in [
            (None, 2, "exhausted 2 connection attempts"),
            (Some("printf secret-token >&2; exit 9"), 1, "exit status 9"),
            (
                Some(r#"printf '%s\n' '{"instructions":"Run chat-tool authenticate."}'"#),
                2,
                "To fix: Run chat-tool authenticate.",
            ),
        ] {
            let directory = workspace_tempdir();
            let mut chat = serde_json::to_value(chat_config()).unwrap();
            chat["working_directory"] = json!(directory.path());
            chat["listener"]["command"] =
                json!("echo attempt >> attempts; printf '%s\\n' '{\"error\":\"authentication\"}'");
            chat["listener"]["recovery"] = json!(recovery);
            write_configuration(directory.path(), chat);
            let (service, _reload) = ConfigService::new(RuntimeSchema::materialize().unwrap());
            service
                .apply_setup(OptifySetup {
                    config_directories: vec![directory.path().display().to_string()],
                    features: vec!["dashboard".to_owned()],
                })
                .unwrap();
            let error = match tokio::time::timeout(
                Duration::from_secs(5),
                ChatRuntime::new(&service).start(),
            )
            .await
            .unwrap()
            {
                Ok(_) => panic!("unavailable chat must fail startup"),
                Err(error) => error,
            };
            assert!(error.contains("authentication"));
            assert!(error.contains(detail), "{error}");
            assert!(error.contains("stopping Personal Dashboard"));
            assert!(!error.contains("secret-token"));
            assert_eq!(
                fs::read_to_string(directory.path().join("attempts"))
                    .unwrap()
                    .lines()
                    .count(),
                expected_attempts
            );
        }
    }

    #[tokio::test]
    async fn propagates_failure_after_an_established_connection() {
        let directory = workspace_tempdir();
        let mut chat = serde_json::to_value(chat_config()).unwrap();
        chat["working_directory"] = json!(directory.path());
        chat["listener"]["command"] = json!(
            "printf '%s\\n' '{\"ready\":true}'; sleep 0.1; printf '%s\\n' '{\"error\":\"connection\"}'"
        );
        chat["listener"]["restart_attempts"] = json!(0);
        write_configuration(directory.path(), chat);
        let (service, _reload) = ConfigService::new(RuntimeSchema::materialize().unwrap());
        service
            .apply_setup(OptifySetup {
                config_directories: vec![directory.path().display().to_string()],
                features: vec!["dashboard".to_owned()],
            })
            .unwrap();
        let mut task = ChatRuntime::new(&service).start().await.unwrap();
        let error = tokio::time::timeout(Duration::from_secs(5), task.wait())
            .await
            .unwrap()
            .unwrap_err();
        assert!(error.contains("connection failure"));
    }

    async fn wait_for_lines(path: &Path, count: usize) -> Vec<String> {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let lines = fs::read_to_string(path)
                    .map(|content| content.lines().map(str::to_owned).collect::<Vec<_>>())
                    .unwrap_or_default();
                if lines.len() >= count {
                    return lines;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("the chat commands did not write the expected lines")
    }

    fn write_configuration(directory: &Path, chat: serde_json::Value) {
        fs::write(
            directory.join("dashboard.json"),
            serde_json::to_vec_pretty(&json!({
                "options": {
                    "appearance": { "theme": "dark" },
                    "application": {
                        "command_timeout_seconds": 5,
                        "default_refresh_seconds": 300,
                        "max_concurrent_commands": 1,
                        "max_output_bytes_per_run": 65536,
                        "shell": "/bin/bash",
                        "working_directories": [directory],
                    },
                    "autocomplete": {
                        "command": "printf '%s' {autocomplete.request}",
                        "debounce_milliseconds": 100,
                        "instruction": "Complete the draft.",
                        "minimum_characters": 1,
                    },
                    "buttons": {
                        "issues": { "advanced": [], "always": [] },
                        "pull_requests": { "advanced": [], "always": [] },
                    },
                    "chat": chat,
                    "sections": [],
                },
            }))
            .unwrap(),
        )
        .unwrap();
    }
}
