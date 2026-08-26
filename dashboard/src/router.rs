use std::sync::Arc;

use crate::config::{ConfigService, ConfigServiceError};
use crate::messages::{ClientRequest, ErrorCode, OptifySetup, ServerResponse};

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
}

impl MessageRouter {
    pub fn new(config_service: Arc<ConfigService>) -> Arc<Self> {
        Arc::new(Self {
            apply_optify_setup: ApplyOptifySetupHandler::new(config_service),
        })
    }

    pub async fn route(&self, request: ClientRequest) -> Result<ServerResponse, RequestError> {
        match request {
            ClientRequest::ApplyOptifySetup { setup } => {
                self.apply_optify_setup.handle(setup).await
            }
        }
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::MessageRouter;
    use crate::config::{ConfigService, RuntimeSchema};
    use crate::messages::{ClientRequest, OptifySetup, ServerResponse};

    #[tokio::test]
    async fn routes_apply_setup_to_the_configuration_service() {
        let (config_service, _reload_service) =
            ConfigService::new(RuntimeSchema::materialize().unwrap());
        let router = MessageRouter::new(config_service);
        let response = router
            .route(ClientRequest::ApplyOptifySetup {
                setup: OptifySetup {
                    config_directories: vec![
                        Path::new(env!("CARGO_MANIFEST_DIR"))
                            .join("configs")
                            .display()
                            .to_string(),
                    ],
                    features: vec!["dashboard".to_owned()],
                },
            })
            .await
            .unwrap();

        let ServerResponse::OptifySetupApplied { configuration } = response;
        assert_eq!(configuration.revision, 1);
    }
}
