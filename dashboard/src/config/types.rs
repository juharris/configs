use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::messages::Theme;

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AppearanceConfig {
    pub theme: Theme,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationConfig {
    pub command_timeout_seconds: u64,
    pub default_refresh_seconds: u64,
    pub max_concurrent_commands: usize,
    pub max_output_bytes_per_run: usize,
    pub shell: String,
    pub working_directories: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AutocompleteConfig {
    pub command: String,
    pub debounce_milliseconds: u64,
    pub instruction: String,
    pub minimum_characters: usize,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ButtonConfig {
    pub command: Option<String>,
    #[serde(default)]
    pub detached: bool,
    pub label: String,
    pub prompt: Option<PromptConfig>,
    pub url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ButtonListsConfig {
    pub advanced: Vec<ButtonConfig>,
    pub always: Vec<ButtonConfig>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ButtonsConfig {
    pub issues: ButtonListsConfig,
    pub pull_requests: ButtonListsConfig,
}

/// Defines the commands that run one turn of a chat conversation.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChatAgentConfig {
    /// Runs every later turn of a conversation and receives that turn's message text.
    pub resume: String,
    /// Runs the first turn of a conversation and receives that turn's message text.
    pub start: String,
    /// Limits how long one turn may run before it is interrupted.
    pub timeout_seconds: u64,
}

/// Names one chat command and describes it in the help reply.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChatCommandConfig {
    pub description: String,
    /// Is the first word of a message that runs this command, such as `/ai`.
    pub name: String,
}

/// Names the commands that control chat conversations.
///
/// A message that does not start with one of these names is queued for the conversation's next turn.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChatCommandsConfig {
    pub help: ChatCommandConfig,
    pub model: ChatCommandConfig,
    pub start: ChatCommandConfig,
    pub status: ChatCommandConfig,
    pub steer: ChatCommandConfig,
    pub stop: ChatCommandConfig,
}

impl ChatCommandsConfig {
    /// Lists every command with its configuration key in alphabetical order.
    pub fn entries(&self) -> [(&'static str, &ChatCommandConfig); 6] {
        [
            ("help", &self.help),
            ("model", &self.model),
            ("start", &self.start),
            ("status", &self.status),
            ("steer", &self.steer),
            ("stop", &self.stop),
        ]
    }
}

/// Connects coding-agent conversations to an external chat through configured commands.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChatConfig {
    /// Runs after the dashboard accepts a message, for example to add a reaction.
    pub acknowledgement: Option<String>,
    pub agent: ChatAgentConfig,
    pub commands: ChatCommandsConfig,
    pub listener: ChatListenerConfig,
    pub models: ChatModelsConfig,
    pub reply: ChatReplyConfig,
    /// Is the current working directory for every chat command and must be one of `application.working_directories`.
    pub working_directory: String,
}

/// Defines the long-running command that prints incoming chat messages.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChatListenerConfig {
    /// Prints a ready event after connecting, then incoming messages as JSON lines.
    pub command: String,
    /// Repairs listener dependencies or authentication before another connection attempt.
    pub recovery: Option<String>,
    /// Bounds each recovery command, including any interactive authentication.
    pub recovery_timeout_seconds: u64,
    /// Limits restarts before a listener failure stops the dashboard.
    pub restart_attempts: usize,
    /// Is the delay before the dashboard restarts a listener that exited.
    pub restart_seconds: u64,
    /// Bounds the wait for an explicit ready event from each connection attempt.
    pub startup_timeout_seconds: u64,
}

/// Lists the models that chat conversations can select.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChatModelsConfig {
    pub available: Vec<String>,
    /// Is the model for new conversations and must be one of `available`.
    pub default: String,
}

/// Defines how the dashboard posts replies into a conversation.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChatReplyConfig {
    pub command: String,
    /// Starts every reply so the dashboard can ignore its own replies when the listener prints them.
    pub prefix: String,
}

/// Describes the dashboard-specific portion of an Optify feature file.
///
/// The standard Optify envelope is composed into the generated schema.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
pub struct DashboardFeatureFile {
    pub options: Option<PartialRootConfig>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ItemKind {
    Issue,
    PullRequest,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PartialRootConfig {
    pub appearance: Option<AppearanceConfig>,
    pub application: Option<ApplicationConfig>,
    pub autocomplete: Option<AutocompleteConfig>,
    pub buttons: Option<ButtonsConfig>,
    pub chat: Option<ChatConfig>,
    pub sections: Option<Vec<SectionConfig>>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PromptConfig {
    pub default: Option<String>,
    pub label: String,
    pub placeholder: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootConfig {
    pub appearance: AppearanceConfig,
    pub application: ApplicationConfig,
    pub autocomplete: AutocompleteConfig,
    pub buttons: ButtonsConfig,
    pub chat: Option<ChatConfig>,
    pub sections: Vec<SectionConfig>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SectionConfig {
    pub cache_ttl_seconds: u64,
    #[serde(default)]
    pub collapsed: bool,
    pub command: String,
    pub id: String,
    pub item_kind: ItemKind,
    pub items_per_page: usize,
    pub refresh_seconds: Option<u64>,
    pub title: String,
}
