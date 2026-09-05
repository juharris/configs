use std::sync::Arc;

use crate::autocomplete::{AutocompleteError, resolve_autocomplete, validate_identifier};
use crate::buttons::{ButtonError, ResolvedCommand, resolve_command};
use crate::config::{ConfigService, ConfigServiceError, ConfigurationSnapshot};
use crate::messages::{
    ButtonList, ClientRequest, DashboardItem, ErrorCode, ItemReference, OptifySetup, ServerResponse,
};
use crate::processes::{ProcessError, ProcessService};
use crate::state::{DashboardService, DashboardServiceError};

/// Applies Optify setup through the configuration service without blocking the socket task.
pub struct ApplyOptifySetupHandler {
    config_service: Arc<ConfigService>,
}

impl ApplyOptifySetupHandler {
    pub fn new(config_service: Arc<ConfigService>) -> Self {
        Self { config_service }
    }

    pub async fn handle(&self, setup: OptifySetup) -> Result<ServerResponse, RequestError> {
        let config_service = self.config_service.clone();
        let snapshot = tokio::task::spawn_blocking(move || config_service.apply_setup(setup))
            .await
            .map_err(|error| RequestError::internal(error.to_string()))?
            .map_err(RequestError::from)?;
        Ok(ServerResponse::OptifySetupApplied {
            configuration: snapshot.transport(),
        })
    }
}

/// Routes each typed request to its focused handler.
pub struct MessageRouter {
    apply_optify_setup: ApplyOptifySetupHandler,
    cancel_autocomplete: CancelAutocompleteHandler,
    cancel_run: CancelRunHandler,
    preview_button: PreviewButtonHandler,
    refresh_section: RefreshSectionHandler,
    request_autocomplete: RequestAutocompleteHandler,
    run_button: RunButtonHandler,
}

impl MessageRouter {
    pub fn new(
        config_service: Arc<ConfigService>,
        dashboard_service: Arc<DashboardService>,
        process_service: Arc<ProcessService>,
    ) -> Arc<Self> {
        let button_resolver = Arc::new(ButtonCommandResolver::new(
            config_service.clone(),
            dashboard_service.clone(),
        ));
        Arc::new(Self {
            apply_optify_setup: ApplyOptifySetupHandler::new(config_service.clone()),
            cancel_autocomplete: CancelAutocompleteHandler::new(process_service.clone()),
            cancel_run: CancelRunHandler::new(process_service.clone()),
            preview_button: PreviewButtonHandler::new(button_resolver.clone()),
            refresh_section: RefreshSectionHandler::new(dashboard_service.clone()),
            request_autocomplete: RequestAutocompleteHandler::new(
                button_resolver.clone(),
                process_service.clone(),
            ),
            run_button: RunButtonHandler::new(button_resolver, process_service),
        })
    }

    pub async fn route(
        &self,
        socket_id: u64,
        connection_id: &str,
        request: ClientRequest,
    ) -> Result<ServerResponse, RequestError> {
        match request {
            ClientRequest::ApplyOptifySetup { setup } => {
                self.apply_optify_setup.handle(setup).await
            }
            ClientRequest::CancelAutocomplete { editor_id } => {
                self.cancel_autocomplete.handle(socket_id, editor_id).await
            }
            ClientRequest::CancelRun { run_id } => {
                self.cancel_run.handle(connection_id, run_id).await
            }
            ClientRequest::PreviewButton {
                button_index,
                button_list,
                configuration_revision,
                item,
                prompt,
                section_id,
                working_directory,
            } => {
                self.preview_button
                    .handle(
                        configuration_revision,
                        section_id,
                        item,
                        button_list,
                        button_index,
                        prompt,
                        working_directory,
                    )
                    .await
            }
            ClientRequest::RefreshSection {
                configuration_revision,
                section_id,
            } => {
                self.refresh_section
                    .handle(configuration_revision, section_id)
                    .await
            }
            ClientRequest::RequestAutocomplete {
                autocomplete_id,
                button_index,
                button_list,
                configuration_revision,
                draft,
                editor_id,
                item,
                section_id,
                selection_end,
                selection_start,
            } => {
                self.request_autocomplete
                    .handle(
                        socket_id,
                        configuration_revision,
                        section_id,
                        item,
                        button_list,
                        button_index,
                        draft,
                        selection_end,
                        selection_start,
                        editor_id,
                        autocomplete_id,
                    )
                    .await
            }
            ClientRequest::RunButton {
                button_index,
                button_list,
                configuration_revision,
                item,
                prompt,
                section_id,
                working_directory,
            } => {
                self.run_button
                    .handle(
                        connection_id,
                        configuration_revision,
                        section_id,
                        item,
                        button_list,
                        button_index,
                        prompt,
                        working_directory,
                    )
                    .await
            }
        }
    }
}

pub struct CancelAutocompleteHandler {
    process_service: Arc<ProcessService>,
}

impl CancelAutocompleteHandler {
    pub fn new(process_service: Arc<ProcessService>) -> Self {
        Self { process_service }
    }

    pub async fn handle(
        &self,
        connection_id: u64,
        editor_id: String,
    ) -> Result<ServerResponse, RequestError> {
        self.process_service
            .cancel_autocomplete(connection_id, &editor_id)
            .map_err(RequestError::from)?;
        Ok(ServerResponse::AutocompleteCancellationAccepted { editor_id })
    }
}

pub struct CancelRunHandler {
    process_service: Arc<ProcessService>,
}

impl CancelRunHandler {
    pub fn new(process_service: Arc<ProcessService>) -> Self {
        Self { process_service }
    }

    pub async fn handle(
        &self,
        connection_id: &str,
        run_id: String,
    ) -> Result<ServerResponse, RequestError> {
        self.process_service
            .cancel(connection_id, &run_id)
            .map_err(RequestError::from)?;
        Ok(ServerResponse::RunCancellationAccepted { run_id })
    }
}

struct ButtonCommandResolver {
    config_service: Arc<ConfigService>,
    dashboard_service: Arc<DashboardService>,
}

impl ButtonCommandResolver {
    fn new(config_service: Arc<ConfigService>, dashboard_service: Arc<DashboardService>) -> Self {
        Self {
            config_service,
            dashboard_service,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn context(
        &self,
        configuration_revision: u64,
        section_id: &str,
        item: &ItemReference,
    ) -> Result<(ConfigurationSnapshot, DashboardItem), RequestError> {
        let item = self
            .dashboard_service
            .item(configuration_revision, section_id, item)
            .map_err(RequestError::from)?;
        let configuration = self
            .config_service
            .snapshot()
            .map_err(RequestError::from)?
            .ok_or_else(|| RequestError::internal("configuration is unavailable".to_owned()))?;
        if configuration.revision != configuration_revision {
            return Err(RequestError::from(
                DashboardServiceError::ConfigurationChanged {
                    active: configuration.revision,
                    requested: configuration_revision,
                },
            ));
        }
        Ok((configuration, item))
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve(
        &self,
        configuration_revision: u64,
        section_id: &str,
        item: &ItemReference,
        button_list: ButtonList,
        button_index: usize,
        prompt: Option<&str>,
        working_directory: &str,
    ) -> Result<ResolvedCommand, RequestError> {
        let (configuration, item) = self.context(configuration_revision, section_id, item)?;
        resolve_command(
            &configuration,
            &item,
            button_list,
            button_index,
            prompt,
            working_directory,
        )
        .map_err(RequestError::from)
    }
}

pub struct RequestAutocompleteHandler {
    process_service: Arc<ProcessService>,
    resolver: Arc<ButtonCommandResolver>,
}

impl RequestAutocompleteHandler {
    fn new(resolver: Arc<ButtonCommandResolver>, process_service: Arc<ProcessService>) -> Self {
        Self {
            process_service,
            resolver,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn handle(
        &self,
        connection_id: u64,
        configuration_revision: u64,
        section_id: String,
        item: ItemReference,
        button_list: ButtonList,
        button_index: usize,
        draft: String,
        selection_end: usize,
        selection_start: usize,
        editor_id: String,
        autocomplete_id: String,
    ) -> Result<ServerResponse, RequestError> {
        validate_identifier(&autocomplete_id)?;
        validate_identifier(&editor_id)?;
        let (configuration, item) =
            self.resolver
                .context(configuration_revision, &section_id, &item)?;
        let invocation = resolve_autocomplete(
            &configuration,
            &item,
            button_list,
            button_index,
            &draft,
            selection_end,
            selection_start,
        )?;
        self.process_service.start_autocomplete(
            connection_id,
            editor_id.clone(),
            autocomplete_id.clone(),
            invocation,
        )?;
        Ok(ServerResponse::AutocompleteRequestAccepted {
            autocomplete_id,
            editor_id,
        })
    }
}

pub struct PreviewButtonHandler {
    resolver: Arc<ButtonCommandResolver>,
}

impl PreviewButtonHandler {
    fn new(resolver: Arc<ButtonCommandResolver>) -> Self {
        Self { resolver }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn handle(
        &self,
        configuration_revision: u64,
        section_id: String,
        item: ItemReference,
        button_list: ButtonList,
        button_index: usize,
        prompt: Option<String>,
        working_directory: String,
    ) -> Result<ServerResponse, RequestError> {
        let command = self.resolver.resolve(
            configuration_revision,
            &section_id,
            &item,
            button_list,
            button_index,
            prompt.as_deref(),
            &working_directory,
        )?;
        Ok(ServerResponse::ButtonPreviewed {
            preview: command.preview,
        })
    }
}

/// Queues refreshes through the dashboard state service without resolving commands in the router.
pub struct RefreshSectionHandler {
    dashboard_service: Arc<DashboardService>,
}

impl RefreshSectionHandler {
    pub fn new(dashboard_service: Arc<DashboardService>) -> Self {
        Self { dashboard_service }
    }

    pub async fn handle(
        &self,
        configuration_revision: u64,
        section_id: String,
    ) -> Result<ServerResponse, RequestError> {
        let refresh = self
            .dashboard_service
            .refresh_section(configuration_revision, section_id)
            .await
            .map_err(RequestError::from)?;
        Ok(ServerResponse::SectionRefreshAccepted { refresh })
    }
}

pub struct RunButtonHandler {
    process_service: Arc<ProcessService>,
    resolver: Arc<ButtonCommandResolver>,
}

impl RunButtonHandler {
    fn new(resolver: Arc<ButtonCommandResolver>, process_service: Arc<ProcessService>) -> Self {
        Self {
            process_service,
            resolver,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn handle(
        &self,
        connection_id: &str,
        configuration_revision: u64,
        section_id: String,
        item: ItemReference,
        button_list: ButtonList,
        button_index: usize,
        prompt: Option<String>,
        working_directory: String,
    ) -> Result<ServerResponse, RequestError> {
        let command = self.resolver.resolve(
            configuration_revision,
            &section_id,
            &item,
            button_list,
            button_index,
            prompt.as_deref(),
            &working_directory,
        )?;
        let run = self.process_service.start(connection_id, command);
        Ok(ServerResponse::ButtonRunAccepted { run })
    }
}

#[derive(Debug)]
pub struct RequestError {
    pub code: ErrorCode,
    pub field: Option<String>,
    pub message: String,
    pub retryable: bool,
}

impl RequestError {
    fn internal(detail: String) -> Self {
        tracing::error!(%detail, "dashboard request failed internally");
        Self {
            code: ErrorCode::Internal,
            field: None,
            message: "The dashboard service could not complete the request.".to_owned(),
            retryable: true,
        }
    }
}

impl From<ConfigServiceError> for RequestError {
    fn from(error: ConfigServiceError) -> Self {
        match error {
            ConfigServiceError::Lock => Self::internal(error.to_string()),
            ConfigServiceError::Setup { field, message } => Self {
                code: ErrorCode::InvalidSetup,
                field: Some(field),
                message,
                retryable: false,
            },
            error => Self {
                code: ErrorCode::InvalidSetup,
                field: None,
                message: error.to_string(),
                retryable: false,
            },
        }
    }
}

impl From<AutocompleteError> for RequestError {
    fn from(error: AutocompleteError) -> Self {
        Self {
            code: ErrorCode::InvalidAutocomplete,
            field: None,
            message: error.to_string(),
            retryable: false,
        }
    }
}

impl From<DashboardServiceError> for RequestError {
    fn from(error: DashboardServiceError) -> Self {
        match error {
            DashboardServiceError::ConfigurationChanged { .. } => Self {
                code: ErrorCode::ConfigurationChanged,
                field: Some("configurationRevision".to_owned()),
                message: "The configuration changed. Refresh the dashboard and try again."
                    .to_owned(),
                retryable: true,
            },
            DashboardServiceError::InvalidSection(_) => Self {
                code: ErrorCode::InvalidSection,
                field: Some("sectionId".to_owned()),
                message: "The requested dashboard section is not configured.".to_owned(),
                retryable: false,
            },
            DashboardServiceError::InvalidItem => Self {
                code: ErrorCode::InvalidItem,
                field: Some("item".to_owned()),
                message: "The requested dashboard item is no longer available.".to_owned(),
                retryable: false,
            },
            DashboardServiceError::Lock | DashboardServiceError::Unavailable => {
                Self::internal(error.to_string())
            }
        }
    }
}

impl From<ButtonError> for RequestError {
    fn from(error: ButtonError) -> Self {
        let field = match &error {
            ButtonError::InvalidWorkingDirectory => "workingDirectory",
            _ => "buttonIndex",
        };
        Self {
            code: ErrorCode::InvalidButton,
            field: Some(field.to_owned()),
            message: error.to_string(),
            retryable: false,
        }
    }
}

impl From<ProcessError> for RequestError {
    fn from(error: ProcessError) -> Self {
        match error {
            ProcessError::InvalidRun => Self {
                code: ErrorCode::InvalidRun,
                field: Some("runId".to_owned()),
                message: error.to_string(),
                retryable: false,
            },
            ProcessError::Lock => Self::internal(error.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::MessageRouter;
    use crate::config::{ConfigService, RuntimeSchema};
    use crate::messages::{ClientRequest, OptifySetup, ServerResponse};
    use crate::processes::ProcessService;
    use crate::state::DashboardService;

    #[tokio::test]
    async fn routes_apply_setup_to_the_configuration_service() {
        let (config_service, _reload_service) =
            ConfigService::new(RuntimeSchema::materialize().unwrap());
        let (dashboard_service, _dashboard_runtime) = DashboardService::new(config_service.clone());
        let connections = crate::connections::ConnectionHub::new();
        let process_service = ProcessService::new(connections);
        let router = MessageRouter::new(config_service, dashboard_service, process_service);
        let response = router
            .route(
                1,
                "connection-test",
                ClientRequest::ApplyOptifySetup {
                    setup: OptifySetup {
                        config_directories: vec![
                            Path::new(env!("CARGO_MANIFEST_DIR"))
                                .join("configs")
                                .display()
                                .to_string(),
                        ],
                        features: vec!["dashboard".to_owned()],
                    },
                },
            )
            .await
            .unwrap();

        match response {
            ServerResponse::OptifySetupApplied { configuration } => {
                assert_eq!(configuration.revision, 1);
            }
            ServerResponse::AutocompleteCancellationAccepted { .. }
            | ServerResponse::AutocompleteRequestAccepted { .. }
            | ServerResponse::ButtonPreviewed { .. }
            | ServerResponse::ButtonRunAccepted { .. }
            | ServerResponse::RunCancellationAccepted { .. }
            | ServerResponse::SectionRefreshAccepted { .. } => {
                panic!("apply setup returned a refresh response")
            }
        }
    }
}
