use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    pub confirm: bool,
    pub label: String,
    pub prompt: Option<PromptConfig>,
    pub url: Option<String>,
    pub working_directory: Option<String>,
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

/// Describes the dashboard-specific portion of an Optify feature file.
///
/// The standard Optify envelope is composed into the generated schema.
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
pub struct DashboardFeatureFile {
    pub options: Option<PartialRootConfig>,
}

#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "snake_case")]
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
    pub repositories: Option<BTreeMap<String, RepositoryConfig>>,
    pub sections: Option<Vec<SectionConfig>>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PromptConfig {
    pub label: String,
    pub placeholder: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryConfig {
    pub path: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootConfig {
    pub appearance: AppearanceConfig,
    pub application: ApplicationConfig,
    pub autocomplete: AutocompleteConfig,
    pub buttons: ButtonsConfig,
    pub repositories: BTreeMap<String, RepositoryConfig>,
    pub sections: Vec<SectionConfig>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SectionConfig {
    pub command: String,
    pub id: String,
    pub item_kind: ItemKind,
    pub refresh_seconds: Option<u64>,
    pub title: String,
}
