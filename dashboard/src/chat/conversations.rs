use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use super::listener::ChatMessage;
use super::requests::ChatRequest;
use crate::config::ChatConfig;

/// Separates queued messages that one turn receives together.
const MESSAGE_SEPARATOR: &str = "\n\n";

/// Describes work that the chat runtime performs for the conversation state machine.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChatEffect {
    Acknowledge {
        conversation_id: String,
        message_id: String,
    },
    Interrupt {
        turn_id: u64,
    },
    Reply {
        conversation_id: String,
        text: String,
    },
    StartTurn(TurnRequest),
}

/// Describes one agent turn for the runtime to start.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TurnRequest {
    pub conversation_id: String,
    pub model: String,
    /// Selects the configured resume command because an earlier turn created the agent session.
    pub resume: bool,
    pub session_id: String,
    pub text: String,
    pub turn_id: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TurnOutcome {
    Completed(String),
    Failed(String),
    Interrupted,
}

impl TurnOutcome {
    /// Names the outcome for logs without including agent output.
    pub fn category(&self) -> &'static str {
        match self {
            Self::Completed(_) => "completed",
            Self::Failed(_) => "failed",
            Self::Interrupted => "interrupted",
        }
    }
}

struct ActiveTurn {
    id: u64,
    started_at: Instant,
}

struct Conversation {
    active_turn: Option<ActiveTurn>,
    model: String,
    queued: Vec<String>,
    session_id: Option<String>,
    /// Records that a turn completed or was interrupted, so the agent saved a session that later turns resume.
    session_started: bool,
    steering: Vec<String>,
}

impl Conversation {
    fn pending_messages(&self) -> usize {
        self.steering.len() + self.queued.len()
    }

    fn summary(&self) -> String {
        let state = self.active_turn.as_ref().map_or_else(
            || "idle".to_owned(),
            |turn| format!("running for {}", format_elapsed(turn.started_at.elapsed())),
        );
        format!(
            "{state} on {} with {}",
            self.model,
            count(self.pending_messages(), "queued message", "queued messages")
        )
    }
}

/// Tracks chat conversations in memory and decides which turns, interrupts, and replies each event causes.
///
/// Conversations are keyed by the ID of the message that started them, which is also the ID of their thread.
#[derive(Default)]
pub struct Conversations {
    conversations: BTreeMap<String, Conversation>,
    default_model: Option<String>,
    next_turn_id: u64,
}

impl Conversations {
    /// Applies the outcome of a finished turn and starts the next turn when messages are waiting.
    pub fn finish_turn(
        &mut self,
        conversation_id: &str,
        turn_id: u64,
        outcome: TurnOutcome,
    ) -> Vec<ChatEffect> {
        let Some(conversation) = self.conversations.get_mut(conversation_id) else {
            return Vec::new();
        };
        if conversation.active_turn.as_ref().map(|turn| turn.id) != Some(turn_id) {
            return Vec::new();
        }
        conversation.active_turn = None;

        let mut effects = Vec::new();
        match outcome {
            TurnOutcome::Completed(text) => {
                conversation.session_started = true;
                let text = if text.is_empty() {
                    "The turn finished without output.".to_owned()
                } else {
                    text
                };
                effects.push(reply(conversation_id, text));
            }
            TurnOutcome::Failed(detail) => effects.push(reply(conversation_id, detail)),
            TurnOutcome::Interrupted => conversation.session_started = true,
        }
        effects.extend(self.start_next_turn(conversation_id));
        effects
    }

    /// Routes one incoming message by its thread and leading command.
    pub fn receive(&mut self, message: ChatMessage, config: &ChatConfig) -> Vec<ChatEffect> {
        if message
            .text
            .trim_start()
            .starts_with(config.reply.prefix.trim())
        {
            return Vec::new();
        }
        let request = ChatRequest::parse(&message.text, &config.commands);
        match message.thread {
            None => self.receive_root(message.id, request, config),
            Some(thread) if self.conversations.contains_key(&thread) => {
                self.receive_in_conversation(thread, message.id, request, config)
            }
            Some(thread) => self.receive_in_unknown_thread(thread, message.id, request, config),
        }
    }

    fn default_model<'a>(&'a self, config: &'a ChatConfig) -> &'a str {
        // A model selected earlier stays the default only while the configuration still offers it.
        self.default_model
            .as_deref()
            .filter(|model| {
                config
                    .models
                    .available
                    .iter()
                    .any(|available| available == model)
            })
            .unwrap_or(&config.models.default)
    }

    fn queue(
        &mut self,
        conversation_id: String,
        message_id: String,
        text: String,
    ) -> Vec<ChatEffect> {
        let Some(conversation) = self.conversations.get_mut(&conversation_id) else {
            return Vec::new();
        };
        conversation.queued.push(text);
        let mut effects = vec![acknowledge(&conversation_id, message_id)];
        effects.extend(self.start_next_turn(&conversation_id));
        effects
    }

    fn receive_in_conversation(
        &mut self,
        conversation_id: String,
        message_id: String,
        request: ChatRequest,
        config: &ChatConfig,
    ) -> Vec<ChatEffect> {
        let conversation = self
            .conversations
            .get_mut(&conversation_id)
            .expect("the caller checked that the conversation exists");
        match request {
            ChatRequest::Help => vec![reply(&conversation_id, help(config))],
            ChatRequest::Model(None) => vec![reply(
                &conversation_id,
                format!(
                    "This conversation uses {}. {}",
                    conversation.model,
                    available_models(config)
                ),
            )],
            ChatRequest::Model(Some(model)) => {
                let text = match unavailable_model(&model, config) {
                    Some(error) => error,
                    None => {
                        let text =
                            format!("This conversation uses {model} starting with its next turn.");
                        conversation.model = model;
                        text
                    }
                };
                vec![reply(&conversation_id, text)]
            }
            ChatRequest::Queue(text) if text.is_empty() => Vec::new(),
            ChatRequest::Queue(text) => self.queue(conversation_id, message_id, text),
            ChatRequest::Start(text) if text.is_empty() => {
                vec![reply(
                    &conversation_id,
                    missing_argument(&config.commands.start.name),
                )]
            }
            ChatRequest::Start(text) => self.queue(conversation_id, message_id, text),
            ChatRequest::Status => vec![reply(
                &conversation_id,
                format!("This conversation is {}.", conversation.summary()),
            )],
            ChatRequest::Steer(text) if text.is_empty() => {
                vec![reply(
                    &conversation_id,
                    missing_argument(&config.commands.steer.name),
                )]
            }
            ChatRequest::Steer(text) => {
                conversation.steering.push(text);
                let mut effects = vec![acknowledge(&conversation_id, message_id)];
                match &conversation.active_turn {
                    Some(turn) => effects.push(ChatEffect::Interrupt { turn_id: turn.id }),
                    None => effects.extend(self.start_next_turn(&conversation_id)),
                }
                effects
            }
            ChatRequest::Stop => {
                conversation.queued.clear();
                conversation.steering.clear();
                match &conversation.active_turn {
                    Some(turn) => vec![
                        ChatEffect::Interrupt { turn_id: turn.id },
                        reply(
                            &conversation_id,
                            "Stopped the running turn and cleared the queue.",
                        ),
                    ],
                    None => vec![reply(&conversation_id, "Nothing is running.")],
                }
            }
        }
    }

    fn receive_in_unknown_thread(
        &mut self,
        thread: String,
        message_id: String,
        request: ChatRequest,
        config: &ChatConfig,
    ) -> Vec<ChatEffect> {
        match request {
            ChatRequest::Help => vec![reply(&thread, help(config))],
            // Threads that never started a conversation are ordinary notes.
            ChatRequest::Queue(_) => Vec::new(),
            ChatRequest::Start(text) => self.start_conversation(thread, message_id, text, config),
            ChatRequest::Model(_)
            | ChatRequest::Status
            | ChatRequest::Steer(_)
            | ChatRequest::Stop => {
                vec![reply(
                    &thread,
                    format!(
                        "No conversation uses this thread. Start one here with {} and a prompt.",
                        config.commands.start.name
                    ),
                )]
            }
        }
    }

    fn receive_root(
        &mut self,
        message_id: String,
        request: ChatRequest,
        config: &ChatConfig,
    ) -> Vec<ChatEffect> {
        match request {
            ChatRequest::Help => vec![reply(&message_id, help(config))],
            ChatRequest::Model(None) => vec![reply(
                &message_id,
                format!(
                    "New conversations use {}. {}",
                    self.default_model(config),
                    available_models(config)
                ),
            )],
            ChatRequest::Model(Some(model)) => {
                let text = unavailable_model(&model, config).unwrap_or_else(|| {
                    let text = format!("New conversations use {model}.");
                    self.default_model = Some(model);
                    text
                });
                vec![reply(&message_id, text)]
            }
            // Messages without a command are ordinary notes.
            ChatRequest::Queue(_) => Vec::new(),
            ChatRequest::Start(text) => {
                self.start_conversation(message_id.clone(), message_id, text, config)
            }
            ChatRequest::Status => vec![reply(&message_id, self.status(config))],
            ChatRequest::Steer(_) => vec![reply(
                &message_id,
                format!(
                    "Reply with {} in the thread of the conversation to steer.",
                    config.commands.steer.name
                ),
            )],
            ChatRequest::Stop => self.stop_all(&message_id),
        }
    }

    fn start_conversation(
        &mut self,
        conversation_id: String,
        message_id: String,
        text: String,
        config: &ChatConfig,
    ) -> Vec<ChatEffect> {
        if text.is_empty() {
            return vec![reply(
                &conversation_id,
                missing_argument(&config.commands.start.name),
            )];
        }
        let model = self.default_model(config).to_owned();
        self.conversations.insert(
            conversation_id.clone(),
            Conversation {
                active_turn: None,
                model,
                queued: Vec::new(),
                session_id: None,
                session_started: false,
                steering: Vec::new(),
            },
        );
        self.queue(conversation_id, message_id, text)
    }

    /// Starts one turn with every waiting message, steering messages first.
    fn start_next_turn(&mut self, conversation_id: &str) -> Option<ChatEffect> {
        let conversation = self.conversations.get_mut(conversation_id)?;
        if conversation.active_turn.is_some() || conversation.pending_messages() == 0 {
            return None;
        }
        // Every attempt to start a session gets a new ID because a failed attempt may have claimed the previous one.
        if !conversation.session_started || conversation.session_id.is_none() {
            match new_session_id() {
                Ok(session_id) => conversation.session_id = Some(session_id),
                Err(_) => {
                    return Some(reply(
                        conversation_id,
                        "The dashboard could not create a session ID. Send another message to retry.",
                    ));
                }
            }
        }
        let text = conversation
            .steering
            .drain(..)
            .chain(conversation.queued.drain(..))
            .collect::<Vec<_>>()
            .join(MESSAGE_SEPARATOR);
        self.next_turn_id += 1;
        conversation.active_turn = Some(ActiveTurn {
            id: self.next_turn_id,
            started_at: Instant::now(),
        });
        Some(ChatEffect::StartTurn(TurnRequest {
            conversation_id: conversation_id.to_owned(),
            model: conversation.model.clone(),
            resume: conversation.session_started,
            session_id: conversation
                .session_id
                .clone()
                .expect("a session ID was assigned above"),
            text,
            turn_id: self.next_turn_id,
        }))
    }

    fn status(&self, config: &ChatConfig) -> String {
        let mut lines = vec![format!(
            "New conversations use {}.",
            self.default_model(config)
        )];
        let (busy, idle): (Vec<_>, Vec<_>) =
            self.conversations.iter().partition(|(_, conversation)| {
                conversation.active_turn.is_some() || conversation.pending_messages() > 0
            });
        lines.extend(
            busy.iter()
                .map(|(id, conversation)| format!("{id} is {}.", conversation.summary())),
        );
        if busy.is_empty() {
            lines.push("Nothing is running.".to_owned());
        }
        if !idle.is_empty() {
            lines.push(format!(
                "{} {} idle.",
                count(idle.len(), "conversation", "conversations"),
                if idle.len() == 1 { "is" } else { "are" }
            ));
        }
        lines.join("\n")
    }

    fn stop_all(&mut self, message_id: &str) -> Vec<ChatEffect> {
        let mut effects = Vec::new();
        for conversation in self.conversations.values_mut() {
            conversation.queued.clear();
            conversation.steering.clear();
            if let Some(turn) = &conversation.active_turn {
                effects.push(ChatEffect::Interrupt { turn_id: turn.id });
            }
        }
        let text = if effects.is_empty() {
            "Nothing is running.".to_owned()
        } else {
            format!(
                "Stopped {} and cleared every queue.",
                count(effects.len(), "running turn", "running turns")
            )
        };
        effects.push(reply(message_id, text));
        effects
    }
}

fn acknowledge(conversation_id: &str, message_id: String) -> ChatEffect {
    ChatEffect::Acknowledge {
        conversation_id: conversation_id.to_owned(),
        message_id,
    }
}

fn available_models(config: &ChatConfig) -> String {
    format!("Available models: {}.", config.models.available.join(", "))
}

fn count(value: usize, singular: &str, plural: &str) -> String {
    format!("{value} {}", if value == 1 { singular } else { plural })
}

fn format_elapsed(elapsed: Duration) -> String {
    let seconds = elapsed.as_secs();
    match (seconds / 3_600, seconds % 3_600 / 60, seconds % 60) {
        (0, 0, seconds) => format!("{seconds}s"),
        (0, minutes, seconds) => format!("{minutes}m {seconds}s"),
        (hours, minutes, _) => format!("{hours}h {minutes}m"),
    }
}

fn help(config: &ChatConfig) -> String {
    config
        .commands
        .entries()
        .iter()
        .map(|(_, command)| format!("{}: {}", command.name, command.description))
        .collect::<Vec<_>>()
        .join("\n")
}

fn missing_argument(command: &str) -> String {
    format!("Add a message after {command}.")
}

/// Formats random bytes as a version 4 UUID because coding agents commonly require UUID session IDs.
fn new_session_id() -> Result<String, getrandom::Error> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    ))
}

fn reply(conversation_id: &str, text: impl Into<String>) -> ChatEffect {
    ChatEffect::Reply {
        conversation_id: conversation_id.to_owned(),
        text: text.into(),
    }
}

fn unavailable_model(model: &str, config: &ChatConfig) -> Option<String> {
    (!config
        .models
        .available
        .iter()
        .any(|available| available == model))
    .then(|| format!("{model} is not available. {}", available_models(config)))
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;

    use super::{ChatEffect, Conversations, TurnOutcome, TurnRequest, new_session_id};
    use crate::chat::listener::ChatMessage;
    use crate::chat::tests::chat_config;

    #[test]
    fn starts_a_conversation_and_resumes_its_session_for_queued_replies() {
        let config = chat_config();
        let mut conversations = Conversations::default();

        let effects = conversations.receive(root("c1", "/ai fix the build"), &config);
        assert_eq!(effects[0], acknowledgement("c1", "c1"));
        let first = turn(&effects[1]);
        assert_eq!(first.conversation_id, "c1");
        assert_eq!(first.model, "opus");
        assert!(!first.resume);
        assert_eq!(first.text, "fix the build");

        assert_eq!(
            conversations.receive(in_thread("m2", "c1", "also run the tests"), &config),
            vec![acknowledgement("c1", "m2")]
        );
        assert_eq!(
            conversations.receive(in_thread("m3", "c1", "/review it"), &config),
            vec![acknowledgement("c1", "m3")]
        );

        let effects = conversations.finish_turn(
            "c1",
            first.turn_id,
            TurnOutcome::Completed("Fixed it.".to_owned()),
        );
        assert_eq!(effects[0], reply("c1", "Fixed it."));
        let second = turn(&effects[1]);
        assert!(second.resume);
        assert_eq!(second.session_id, first.session_id);
        assert_eq!(second.text, "also run the tests\n\n/review it");
    }

    #[test]
    fn steering_interrupts_the_running_turn_and_runs_first() {
        let config = chat_config();
        let mut conversations = Conversations::default();
        let first = turn(&conversations.receive(root("c1", "/ai build it"), &config)[1]);
        conversations.receive(in_thread("m2", "c1", "queued note"), &config);

        let effects = conversations.receive(in_thread("m3", "c1", "/steer use Rust"), &config);
        assert_eq!(
            effects,
            vec![
                acknowledgement("c1", "m3"),
                ChatEffect::Interrupt {
                    turn_id: first.turn_id
                },
            ]
        );

        let effects = conversations.finish_turn("c1", first.turn_id, TurnOutcome::Interrupted);
        let second = turn(&effects[0]);
        assert_eq!(effects.len(), 1);
        assert!(second.resume);
        assert_eq!(second.text, "use Rust\n\nqueued note");
    }

    #[test]
    fn stopping_interrupts_and_clears_queued_messages() {
        let config = chat_config();
        let mut conversations = Conversations::default();
        let first = turn(&conversations.receive(root("c1", "/ai build it"), &config)[1]);
        conversations.receive(in_thread("m2", "c1", "queued note"), &config);

        assert_eq!(
            conversations.receive(in_thread("m3", "c1", "/stop"), &config),
            vec![
                ChatEffect::Interrupt {
                    turn_id: first.turn_id
                },
                reply("c1", "Stopped the running turn and cleared the queue."),
            ]
        );
        assert!(
            conversations
                .finish_turn("c1", first.turn_id, TurnOutcome::Interrupted)
                .is_empty()
        );
        assert_eq!(
            conversations.receive(in_thread("m4", "c1", "/stop"), &config),
            vec![reply("c1", "Nothing is running.")]
        );
    }

    #[test]
    fn retries_a_failed_first_turn_with_a_new_session() {
        let config = chat_config();
        let mut conversations = Conversations::default();
        let first = turn(&conversations.receive(root("c1", "/ai build it"), &config)[1]);

        assert_eq!(
            conversations.finish_turn(
                "c1",
                first.turn_id,
                TurnOutcome::Failed("The agent command exited with status 1.".to_owned())
            ),
            vec![reply("c1", "The agent command exited with status 1.")]
        );

        let effects = conversations.receive(in_thread("m2", "c1", "try again"), &config);
        let second = turn(&effects[1]);
        assert!(!second.resume);
        assert_ne!(second.session_id, first.session_id);
    }

    #[test]
    fn selects_models_for_new_conversations_and_existing_threads() {
        let config = chat_config();
        let mut conversations = Conversations::default();

        assert_eq!(
            conversations.receive(root("r1", "/model"), &config),
            vec![reply(
                "r1",
                "New conversations use opus. Available models: haiku, opus, sonnet."
            )]
        );
        assert_eq!(
            conversations.receive(root("r2", "/model gpt"), &config),
            vec![reply(
                "r2",
                "gpt is not available. Available models: haiku, opus, sonnet."
            )]
        );
        assert_eq!(
            conversations.receive(root("r3", "/model sonnet"), &config),
            vec![reply("r3", "New conversations use sonnet.")]
        );
        let first = turn(&conversations.receive(root("c1", "/ai build it"), &config)[1]);
        assert_eq!(first.model, "sonnet");

        assert_eq!(
            conversations.receive(in_thread("m2", "c1", "/model haiku"), &config),
            vec![reply(
                "c1",
                "This conversation uses haiku starting with its next turn."
            )]
        );
        conversations.receive(in_thread("m3", "c1", "next"), &config);
        let effects = conversations.finish_turn("c1", first.turn_id, TurnOutcome::Interrupted);
        assert_eq!(turn(&effects[0]).model, "haiku");
    }

    #[test]
    fn answers_help_status_and_usage_without_starting_turns() {
        let config = chat_config();
        let mut conversations = Conversations::default();

        assert_eq!(
            conversations.receive(root("r1", "/help"), &config),
            vec![reply(
                "r1",
                "/help: Show these commands.\n/model: Show or select the model.\n/ai: Start a conversation.\n/status: Show what is running.\n/steer: Interrupt the turn and send this message next.\n/stop: Stop the turn and clear the queue."
            )]
        );
        assert_eq!(
            conversations.receive(root("r2", "/ai"), &config),
            vec![reply("r2", "Add a message after /ai.")]
        );
        assert_eq!(
            conversations.receive(root("r3", "/status"), &config),
            vec![reply(
                "r3",
                "New conversations use opus.\nNothing is running."
            )]
        );

        conversations.receive(root("c1", "/ai build it"), &config);
        conversations.receive(in_thread("m2", "c1", "queued note"), &config);
        assert_eq!(
            conversations.receive(in_thread("m3", "c1", "/status"), &config),
            vec![reply(
                "c1",
                "This conversation is running for 0s on opus with 1 queued message."
            )]
        );
        assert_eq!(
            conversations.receive(root("r4", "/status"), &config),
            vec![reply(
                "r4",
                "New conversations use opus.\nc1 is running for 0s on opus with 1 queued message."
            )]
        );
    }

    #[test]
    fn ignores_notes_replies_and_threads_without_conversations() {
        let config = chat_config();
        let mut conversations = Conversations::default();

        assert!(
            conversations
                .receive(root("r1", "remember the milk"), &config)
                .is_empty()
        );
        assert!(
            conversations
                .receive(in_thread("m1", "r1", "and eggs"), &config)
                .is_empty()
        );
        assert_eq!(
            conversations.receive(in_thread("m2", "r1", "/steer go"), &config),
            vec![reply(
                "r1",
                "No conversation uses this thread. Start one here with /ai and a prompt."
            )]
        );

        conversations.receive(root("c1", "/ai build it"), &config);
        assert!(
            conversations
                .receive(in_thread("m3", "c1", ":robot_face: Fixed it."), &config)
                .is_empty()
        );

        let effects = conversations.receive(in_thread("m4", "r1", "/ai pick this up"), &config);
        assert_eq!(effects[0], acknowledgement("r1", "m4"));
        assert_eq!(turn(&effects[1]).conversation_id, "r1");
    }

    #[test]
    fn ignores_outcomes_of_superseded_turns() {
        let config = chat_config();
        let mut conversations = Conversations::default();
        let first = turn(&conversations.receive(root("c1", "/ai build it"), &config)[1]);

        assert!(
            conversations
                .finish_turn("c1", first.turn_id + 1, TurnOutcome::Interrupted)
                .is_empty()
        );
        assert!(
            conversations
                .finish_turn("unknown", first.turn_id, TurnOutcome::Interrupted)
                .is_empty()
        );
    }

    #[test]
    fn creates_version_4_uuids() {
        let session_id = new_session_id().unwrap();

        let groups = session_id.split('-').map(str::len).collect::<Vec<_>>();
        assert_eq!(groups, [8, 4, 4, 4, 12]);
        assert_eq!(&session_id[14..15], "4");
        assert_matches!(&session_id[19..20], "8" | "9" | "a" | "b");
    }

    fn acknowledgement(conversation_id: &str, message_id: &str) -> ChatEffect {
        ChatEffect::Acknowledge {
            conversation_id: conversation_id.to_owned(),
            message_id: message_id.to_owned(),
        }
    }

    fn in_thread(id: &str, thread: &str, text: &str) -> ChatMessage {
        ChatMessage {
            id: id.to_owned(),
            text: text.to_owned(),
            thread: Some(thread.to_owned()),
        }
    }

    fn reply(conversation_id: &str, text: &str) -> ChatEffect {
        ChatEffect::Reply {
            conversation_id: conversation_id.to_owned(),
            text: text.to_owned(),
        }
    }

    fn root(id: &str, text: &str) -> ChatMessage {
        ChatMessage {
            id: id.to_owned(),
            text: text.to_owned(),
            thread: None,
        }
    }

    fn turn(effect: &ChatEffect) -> TurnRequest {
        match effect {
            ChatEffect::StartTurn(request) => request.clone(),
            effect => panic!("expected a turn but found {effect:?}"),
        }
    }
}
