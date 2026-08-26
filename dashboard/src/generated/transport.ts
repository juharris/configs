// This file is generated from Rust transport types.
// Run `pnpm run bindings:generate` instead of editing it.

export type ActiveConfiguration = { revision: number, setup: OptifySetup, theme: Theme, };

export type BootstrapResponse = { protocolVersion: number, token: string, };

export type ButtonList = "advanced" | "always";

export type ClientMessage = { "type": "authenticate", lastEventSequence: number | null, protocolVersion: number, token: string, } | { "type": "request", request: ClientRequest, requestId: string, };

export type ClientRequest = { "type": "apply_optify_setup", setup: OptifySetup, } | { "type": "cancel_run", runId: string, } | { "type": "preview_button", buttonIndex: number, buttonList: ButtonList, configurationRevision: number, item: ItemReference, prompt: string | null, sectionId: string, } | { "type": "refresh_section", configurationRevision: number, sectionId: string, } | { "type": "run_button", buttonIndex: number, buttonList: ButtonList, configurationRevision: number, item: ItemReference, prompt: string | null, sectionId: string, };

export type DashboardButton = { confirm: boolean, disabled: boolean, index: number, label: string, prompt: PromptPresentation | null, title: string, url: string | null, };

export type DashboardItem = { advancedButtons: Array<DashboardButton>, assignees: Array<string>, alwaysButtons: Array<DashboardButton>, author: string | null, isDraft: boolean | null, itemKind: ItemKind, labels: Array<DashboardLabel>, number: number, repository: string, source: string | null, state: string, title: string, updatedAt: string, url: string, };

export type DashboardLabel = { color: string | null, name: string, };

export type DashboardSnapshot = { configurationRevision: number, sections: Array<SectionSnapshot>, };

export type ErrorCode = "authentication_failed" | "configuration_changed" | "internal" | "invalid_button" | "invalid_item" | "invalid_message" | "invalid_run" | "invalid_section" | "invalid_setup" | "protocol_mismatch";

export type ItemReference = { number: number, repository: string, source: string | null, };

export type ItemKind = "issue" | "pull_request";

export type OptifySetup = { configDirectories: Array<string>, features: Array<string>, };

export type PromptPresentation = { label: string, placeholder: string, };

export type RunSnapshot = { exitCode: number | null, id: string, label: string, output: string, preview: string, status: RunStatus, };

export type RunStatus = "cancelled" | "completed" | "failed" | "queued" | "running" | "timed_out";

export type SectionRefresh = { coalesced: boolean, sectionId: string, status: SectionRefreshStatus, };

export type SectionRefreshStatus = "idle" | "queued" | "refreshing";

export type SectionSnapshot = { error: string | null, id: string, items: Array<DashboardItem>, itemsPerPage: number, lastSuccessfulRefresh: number | null, stale: boolean, status: SectionRefreshStatus, title: string, };

export type ServerEvent = { "type": "configuration_reloaded", configuration: ActiveConfiguration, } | { "type": "dashboard_updated", dashboard: DashboardSnapshot, } | { "type": "run_updated", run: RunSnapshot, };

export type ServerMessage = { "type": "connection_ready", activeConfiguration: ActiveConfiguration | null, connectionId: string, dashboard: DashboardSnapshot | null, eventSequence: number, protocolVersion: number, setupStatus: SetupStatus, } | { "type": "error", code: ErrorCode, field: string | null, message: string, requestId: string | null, retryable: boolean, } | { "type": "event", event: ServerEvent, eventId: string, sequence: number, } | { "type": "response", requestId: string, response: ServerResponse, };

export type ServerResponse = { "type": "button_previewed", preview: string, } | { "type": "button_run_accepted", run: RunSnapshot, } | { "type": "optify_setup_applied", configuration: ActiveConfiguration, } | { "type": "run_cancellation_accepted", runId: string, } | { "type": "section_refresh_accepted", refresh: SectionRefresh, };

export type SetupStatus = "configured" | "required";

export type Theme = "dark" | "light" | "system";
