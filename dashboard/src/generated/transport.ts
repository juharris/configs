// This file is generated from Rust transport types.
// Run `pnpm run bindings:generate` instead of editing it.

export type ActiveConfiguration = { revision: number, setup: OptifySetup, theme: Theme, };

export type BootstrapResponse = { protocolVersion: number, token: string, };

export type ClientMessage = { "type": "authenticate", lastEventSequence: number | null, protocolVersion: number, token: string, } | { "type": "request", request: ClientRequest, requestId: string, };

export type ClientRequest = { "type": "apply_optify_setup", setup: OptifySetup, };

export type ErrorCode = "authentication_failed" | "internal" | "invalid_message" | "invalid_setup" | "protocol_mismatch";

export type OptifySetup = { configDirectories: Array<string>, features: Array<string>, };

export type ServerEvent = { "type": "configuration_reloaded", configuration: ActiveConfiguration, };

export type ServerMessage = { "type": "connection_ready", activeConfiguration: ActiveConfiguration | null, connectionId: string, eventSequence: number, protocolVersion: number, setupStatus: SetupStatus, } | { "type": "error", code: ErrorCode, field: string | null, message: string, requestId: string | null, retryable: boolean, } | { "type": "event", event: ServerEvent, eventId: string, sequence: number, } | { "type": "response", requestId: string, response: ServerResponse, };

export type ServerResponse = { "type": "optify_setup_applied", configuration: ActiveConfiguration, };

export type SetupStatus = "configured" | "required";

export type Theme = "dark" | "light" | "system";
