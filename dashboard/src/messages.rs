use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::config::ItemKind;

pub const PROTOCOL_VERSION: u16 = 9;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ActiveConfiguration {
    pub autocomplete: AutocompleteSettings,
    #[ts(type = "number")]
    pub revision: u64,
    pub setup: OptifySetup,
    pub theme: Theme,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AutocompleteSettings {
    #[ts(type = "number")]
    pub debounce_milliseconds: u64,
    pub minimum_characters: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AutocompleteSnapshot {
    pub autocomplete_id: String,
    pub editor_id: String,
    pub error: Option<String>,
    pub status: AutocompleteStatus,
    pub suggestion: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AutocompleteStatus {
    Completed,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct BootstrapResponse {
    pub protocol_version: u16,
    pub token: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    tag = "type"
)]
pub enum ClientMessage {
    Authenticate {
        connection_id: Option<String>,
        #[ts(type = "number | null")]
        last_event_sequence: Option<u64>,
        protocol_version: u16,
        token: String,
    },
    Request {
        request: ClientRequest,
        request_id: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    tag = "type"
)]
pub enum ClientRequest {
    ApplyOptifySetup {
        setup: OptifySetup,
    },
    CancelAutocomplete {
        editor_id: String,
    },
    CancelRun {
        run_id: String,
    },
    PreviewButton {
        button_index: usize,
        button_list: ButtonList,
        #[ts(type = "number")]
        configuration_revision: u64,
        item: ItemReference,
        prompt: Option<String>,
        section_id: String,
    },
    RefreshSection {
        #[ts(type = "number")]
        configuration_revision: u64,
        section_id: String,
    },
    RequestAutocomplete {
        autocomplete_id: String,
        button_index: usize,
        button_list: ButtonList,
        #[ts(type = "number")]
        configuration_revision: u64,
        draft: String,
        editor_id: String,
        item: ItemReference,
        section_id: String,
        selection_end: usize,
        selection_start: usize,
    },
    RunButton {
        button_index: usize,
        button_list: ButtonList,
        #[ts(type = "number")]
        configuration_revision: u64,
        item: ItemReference,
        prompt: Option<String>,
        section_id: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DashboardActor {
    pub login: String,
    pub url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DashboardItem {
    pub advanced_buttons: Vec<DashboardButton>,
    pub approved_by: Vec<DashboardActor>,
    pub assignees: Vec<String>,
    pub always_buttons: Vec<DashboardButton>,
    pub author: Option<String>,
    pub is_draft: Option<bool>,
    pub item_kind: ItemKind,
    pub labels: Vec<DashboardLabel>,
    #[ts(type = "number")]
    pub number: u64,
    pub repository: String,
    pub source: Option<String>,
    pub state: String,
    pub title: String,
    pub updated_at: String,
    pub url: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ButtonList {
    Advanced,
    Always,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DashboardButton {
    pub disabled: bool,
    pub index: usize,
    pub label: String,
    pub prompt: Option<PromptPresentation>,
    pub title: String,
    pub url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DashboardLabel {
    pub color: Option<String>,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DashboardSnapshot {
    #[ts(type = "number")]
    pub configuration_revision: u64,
    pub sections: Vec<SectionSnapshot>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    AuthenticationFailed,
    ConfigurationChanged,
    Internal,
    InvalidAutocomplete,
    InvalidButton,
    InvalidItem,
    InvalidMessage,
    InvalidRun,
    InvalidSection,
    InvalidSetup,
    ProtocolMismatch,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ItemReference {
    #[ts(type = "number")]
    pub number: u64,
    pub repository: String,
    pub source: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct OptifySetup {
    pub config_directories: Vec<String>,
    pub features: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    tag = "type"
)]
pub enum ServerEvent {
    AutocompleteUpdated { autocomplete: AutocompleteSnapshot },
    ConfigurationReloaded { configuration: ActiveConfiguration },
    DashboardUpdated { dashboard: DashboardSnapshot },
    RunUpdated { run: RunSnapshot },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    tag = "type"
)]
pub enum ServerMessage {
    ConnectionReady {
        active_configuration: Option<ActiveConfiguration>,
        connection_id: String,
        dashboard: Option<DashboardSnapshot>,
        #[ts(type = "number")]
        event_sequence: u64,
        protocol_version: u16,
        run: Option<RunSnapshot>,
        runs: Vec<RunSnapshot>,
        setup_status: SetupStatus,
    },
    Error {
        code: ErrorCode,
        field: Option<String>,
        message: String,
        request_id: Option<String>,
        retryable: bool,
    },
    Event {
        event: ServerEvent,
        event_id: String,
        #[ts(type = "number")]
        sequence: u64,
    },
    Response {
        request_id: String,
        response: ServerResponse,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    tag = "type"
)]
pub enum ServerResponse {
    AutocompleteCancellationAccepted {
        editor_id: String,
    },
    AutocompleteRequestAccepted {
        autocomplete_id: String,
        editor_id: String,
    },
    ButtonPreviewed {
        preview: String,
    },
    ButtonRunAccepted {
        run: RunSnapshot,
    },
    OptifySetupApplied {
        configuration: ActiveConfiguration,
    },
    RunCancellationAccepted {
        run_id: String,
    },
    SectionRefreshAccepted {
        refresh: SectionRefresh,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct PromptPresentation {
    pub default: Option<String>,
    pub label: String,
    pub placeholder: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct RunSnapshot {
    #[ts(type = "number")]
    pub created_at: u64,
    #[ts(type = "number | null")]
    pub exit_code: Option<i32>,
    pub id: String,
    pub label: String,
    pub output: String,
    pub preview: String,
    pub status: RunStatus,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Cancelled,
    Completed,
    Failed,
    Queued,
    Running,
    Started,
    TimedOut,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SectionRefreshStatus {
    Idle,
    Queued,
    Refreshing,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SectionRefresh {
    pub coalesced: bool,
    pub section_id: String,
    pub status: SectionRefreshStatus,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SectionSnapshot {
    pub collapsed: bool,
    pub error: Option<String>,
    pub id: String,
    pub items: Vec<DashboardItem>,
    pub items_per_page: usize,
    #[ts(type = "number | null")]
    pub last_successful_refresh: Option<u64>,
    #[ts(type = "number")]
    pub refresh_seconds: u64,
    pub stale: bool,
    pub status: SectionRefreshStatus,
    pub title: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SetupStatus {
    Configured,
    Required,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum Theme {
    Dark,
    Light,
    System,
}
