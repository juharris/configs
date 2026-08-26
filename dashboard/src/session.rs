use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};

use crate::config::ConfigService;
use crate::connections::{BootstrapTokenStore, ConnectionHub};
use crate::messages::{ClientMessage, ErrorCode, PROTOCOL_VERSION, ServerMessage, SetupStatus};
use crate::router::MessageRouter;
use crate::state::DashboardService;

const AUTHENTICATION_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct SessionServices {
    pub config_service: Arc<ConfigService>,
    pub connections: Arc<ConnectionHub>,
    pub dashboard_service: Arc<DashboardService>,
    pub router: Arc<MessageRouter>,
    pub tokens: Arc<BootstrapTokenStore>,
}

/// Authenticates one browser tab and owns its socket reader and bounded writer.
pub async fn run(mut socket: WebSocket, services: SessionServices) {
    let Some(last_event_sequence) = authenticate(&mut socket, &services.tokens).await else {
        return;
    };
    let Ok(registered) = services.connections.register() else {
        send_and_close(
            &mut socket,
            error_message(
                ErrorCode::Internal,
                "The dashboard service could not create the connection.",
            ),
        )
        .await;
        return;
    };
    let connection_id = registered.id;
    let active_configuration = match services.config_service.snapshot() {
        Ok(snapshot) => snapshot.map(|snapshot| snapshot.transport()),
        Err(error) => {
            tracing::error!(%error, "could not read active configuration for a connection");
            services.connections.unregister(connection_id);
            send_and_close(
                &mut socket,
                error_message(
                    ErrorCode::Internal,
                    "The dashboard service could not synchronize the connection.",
                ),
            )
            .await;
            return;
        }
    };
    let setup_status = if active_configuration.is_some() {
        SetupStatus::Configured
    } else {
        SetupStatus::Required
    };
    let dashboard = match services.dashboard_service.snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            tracing::error!(%error, "could not read dashboard state for a connection");
            services.connections.unregister(connection_id);
            send_and_close(
                &mut socket,
                error_message(
                    ErrorCode::Internal,
                    "The dashboard service could not synchronize the connection.",
                ),
            )
            .await;
            return;
        }
    };
    let ready = ServerMessage::ConnectionReady {
        active_configuration,
        connection_id: format!("connection-{connection_id}"),
        dashboard,
        event_sequence: services.connections.current_event_sequence(),
        protocol_version: PROTOCOL_VERSION,
        setup_status,
    };
    if send_message(&mut socket, &ready).await.is_err() {
        services.connections.unregister(connection_id);
        return;
    }
    if let Ok(events) = services.connections.replay_after(last_event_sequence) {
        for event in events {
            if send_message(&mut socket, &event).await.is_err() {
                services.connections.unregister(connection_id);
                return;
            }
        }
    }

    let (mut socket_sender, mut socket_receiver) = socket.split();
    let mut outbound = registered.receiver;
    let writer = tokio::spawn(async move {
        while let Some(message) = outbound.recv().await {
            let Ok(payload) = serde_json::to_string(&message) else {
                continue;
            };
            if socket_sender
                .send(Message::Text(payload.into()))
                .await
                .is_err()
            {
                return;
            }
        }
    });

    while let Some(message) = socket_receiver.next().await {
        let Ok(message) = message else {
            break;
        };
        let Message::Text(payload) = message else {
            if matches!(message, Message::Close(_)) {
                break;
            }
            services.connections.send(
                connection_id,
                error_message(ErrorCode::InvalidMessage, "Send JSON text messages only."),
            );
            continue;
        };
        let client_message = match serde_json::from_str::<ClientMessage>(payload.as_str()) {
            Ok(message) => message,
            Err(_) => {
                services.connections.send(
                    connection_id,
                    error_message(
                        ErrorCode::InvalidMessage,
                        "The dashboard request was not valid.",
                    ),
                );
                continue;
            }
        };
        let ClientMessage::Request {
            request,
            request_id,
        } = client_message
        else {
            services.connections.send(
                connection_id,
                error_message(
                    ErrorCode::InvalidMessage,
                    "The connection is already authenticated.",
                ),
            );
            continue;
        };
        if request_id.trim().is_empty() {
            services.connections.send(
                connection_id,
                error_message(ErrorCode::InvalidMessage, "The request ID cannot be blank."),
            );
            continue;
        }
        let response = match services.router.route(connection_id, request).await {
            Ok(response) => ServerMessage::Response {
                request_id,
                response,
            },
            Err(error) => ServerMessage::Error {
                code: error.code,
                field: error.field,
                message: error.message,
                request_id: Some(request_id),
                retryable: error.retryable,
            },
        };
        services.connections.send(connection_id, response);
    }

    services.connections.unregister(connection_id);
    writer.abort();
}

async fn authenticate(socket: &mut WebSocket, tokens: &BootstrapTokenStore) -> Option<Option<u64>> {
    let message = tokio::time::timeout(AUTHENTICATION_TIMEOUT, socket.recv())
        .await
        .ok()
        .flatten()
        .and_then(Result::ok);
    let Some(Message::Text(payload)) = message else {
        send_and_close(
            socket,
            error_message(
                ErrorCode::AuthenticationFailed,
                "Authenticate before sending dashboard requests.",
            ),
        )
        .await;
        return None;
    };
    let Ok(ClientMessage::Authenticate {
        last_event_sequence,
        protocol_version,
        token,
    }) = serde_json::from_str::<ClientMessage>(payload.as_str())
    else {
        send_and_close(
            socket,
            error_message(
                ErrorCode::AuthenticationFailed,
                "The authentication message was not valid.",
            ),
        )
        .await;
        return None;
    };
    if protocol_version != PROTOCOL_VERSION {
        send_and_close(
            socket,
            error_message(
                ErrorCode::ProtocolMismatch,
                "The browser and dashboard service protocol versions do not match.",
            ),
        )
        .await;
        return None;
    }
    if !tokens.consume(&token).unwrap_or(false) {
        send_and_close(
            socket,
            error_message(
                ErrorCode::AuthenticationFailed,
                "The dashboard connection token is invalid or expired.",
            ),
        )
        .await;
        return None;
    }
    Some(last_event_sequence)
}

fn error_message(code: ErrorCode, message: &str) -> ServerMessage {
    ServerMessage::Error {
        code,
        field: None,
        message: message.to_owned(),
        request_id: None,
        retryable: false,
    }
}

async fn send_and_close(socket: &mut WebSocket, message: ServerMessage) {
    let _ = send_message(socket, &message).await;
    let _ = socket.send(Message::Close(None)).await;
}

async fn send_message(socket: &mut WebSocket, message: &ServerMessage) -> Result<(), axum::Error> {
    let payload = serde_json::to_string(message).expect("server messages are serializable");
    socket.send(Message::Text(payload.into())).await
}
