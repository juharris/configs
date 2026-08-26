use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ActiveConfiguration {
    #[ts(type = "number")]
    pub revision: u64,
    pub setup: OptifySetup,
    pub theme: Theme,
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
    ApplyOptifySetup { setup: OptifySetup },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    AuthenticationFailed,
    Internal,
    InvalidMessage,
    InvalidSetup,
    ProtocolMismatch,
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
    ConfigurationReloaded { configuration: ActiveConfiguration },
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
        #[ts(type = "number")]
        event_sequence: u64,
        protocol_version: u16,
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
    OptifySetupApplied { configuration: ActiveConfiguration },
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
