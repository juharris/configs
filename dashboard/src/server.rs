use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use axum::{Json, Router};
use tower_http::trace::TraceLayer;

use crate::config::ConfigService;
use crate::connections::{BootstrapTokenStore, ConnectionHub};
use crate::messages::{BootstrapResponse, PROTOCOL_VERSION};
use crate::router::MessageRouter;
use crate::session::{self, SessionServices};

pub const DEVELOPMENT_BACKEND_PORT: u16 = 3000;
pub const PUBLIC_PORT: u16 = 5173;
pub const PUBLIC_ORIGIN: &str = "http://127.0.0.1:5173";

const MAX_WEBSOCKET_MESSAGE_BYTES: usize = 64 * 1024;
const PUBLIC_HOST: &str = "127.0.0.1:5173";

#[derive(Clone)]
struct ApplicationState {
    sessions: SessionServices,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LaunchOptions {
    pub open_browser: bool,
}

impl LaunchOptions {
    pub fn from_arguments(arguments: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments.into_iter();
        let options = match arguments.next().as_deref() {
            None => Self { open_browser: true },
            Some("--no-open") => Self {
                open_browser: false,
            },
            Some(argument) => return Err(format!("unsupported argument: {argument}")),
        };
        if let Some(argument) = arguments.next() {
            return Err(format!("unsupported argument: {argument}"));
        }
        Ok(options)
    }
}

pub async fn serve(
    options: LaunchOptions,
    config_service: Arc<ConfigService>,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    let _ = options;
    let port = if cfg!(debug_assertions) {
        DEVELOPMENT_BACKEND_PORT
    } else {
        PUBLIC_PORT
    };
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    let listener = tokio::net::TcpListener::bind(address).await.map_err(|error| {
        format!(
            "could not bind Personal Dashboard to http://{address}; stop the process using port {port} and try again: {error}"
        )
    })?;
    let application = application(config_service);
    let url = format!("http://{address}");
    tracing::info!(%url, "Personal Dashboard is ready");

    #[cfg(not(debug_assertions))]
    if options.open_browser {
        webbrowser::open(&url)
            .map_err(|error| format!("could not open the default browser at {url}: {error}"))?;
    }

    axum::serve(listener, application).await?;
    Ok(())
}

fn application(config_service: Arc<ConfigService>) -> Router {
    let connections = ConnectionHub::new();
    tokio::spawn(
        connections
            .clone()
            .publish_config_events(config_service.subscribe()),
    );
    application_with_state(ApplicationState {
        sessions: SessionServices {
            config_service: config_service.clone(),
            connections,
            router: MessageRouter::new(config_service),
            tokens: Arc::new(BootstrapTokenStore::new()),
        },
    })
}

fn application_with_state(state: ApplicationState) -> Router {
    Router::new()
        .route("/bootstrap", get(bootstrap))
        .route("/ws", any(websocket))
        .fallback(frontend_asset)
        .with_state(state)
        .layer(middleware::from_fn(protect_loopback_origin))
        .layer(TraceLayer::new_for_http())
}

async fn bootstrap(State(state): State<ApplicationState>) -> Result<impl IntoResponse, StatusCode> {
    let token = state.sessions.tokens.issue().map_err(|error| {
        tracing::error!(%error, "could not issue dashboard bootstrap token");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok((
        [
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
            (header::PRAGMA, HeaderValue::from_static("no-cache")),
        ],
        Json(BootstrapResponse {
            protocol_version: PROTOCOL_VERSION,
            token,
        }),
    ))
}

async fn protect_loopback_origin(request: Request, next: Next) -> Response {
    if !header_matches(request.headers(), header::HOST, PUBLIC_HOST) {
        return StatusCode::FORBIDDEN.into_response();
    }

    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; connect-src 'self' ws://127.0.0.1:5173; img-src 'self' data:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
        ),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    response
}

async fn websocket(
    State(state): State<ApplicationState>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Response {
    if !header_matches(&headers, header::ORIGIN, PUBLIC_ORIGIN) {
        return StatusCode::FORBIDDEN.into_response();
    }
    upgrade
        .max_frame_size(MAX_WEBSOCKET_MESSAGE_BYTES)
        .max_message_size(MAX_WEBSOCKET_MESSAGE_BYTES)
        .on_upgrade(move |socket| session::run(socket, state.sessions))
}

fn header_matches(headers: &HeaderMap, name: header::HeaderName, expected: &str) -> bool {
    headers.get(name).and_then(|value| value.to_str().ok()) == Some(expected)
}

#[cfg(debug_assertions)]
async fn frontend_asset() -> axum::http::StatusCode {
    axum::http::StatusCode::NOT_FOUND
}

#[cfg(not(debug_assertions))]
async fn frontend_asset(uri: axum::http::Uri) -> axum::response::Response {
    use axum::body::Body;
    use rust_embed::RustEmbed;

    #[derive(RustEmbed)]
    #[folder = "dist/"]
    struct FrontendAssets;

    let requested_path = uri.path().trim_start_matches('/');
    let asset = FrontendAssets::get(requested_path)
        .map(|asset| (requested_path, asset))
        .or_else(|| FrontendAssets::get("index.html").map(|asset| ("index.html", asset)));
    let Some((asset_path, asset)) = asset else {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    };
    let content_type = mime_guess::from_path(asset_path).first_or_octet_stream();
    axum::response::Response::builder()
        .header(axum::http::header::CONTENT_TYPE, content_type.as_ref())
        .body(Body::from(asset.data))
        .expect("static asset response is valid")
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue, header};

    use super::{LaunchOptions, PUBLIC_HOST, PUBLIC_ORIGIN, header_matches};

    #[test]
    fn opens_the_browser_by_default_and_supports_headless_verification() {
        assert_eq!(
            LaunchOptions::from_arguments(Vec::new()).unwrap(),
            LaunchOptions { open_browser: true }
        );
        assert_eq!(
            LaunchOptions::from_arguments(["--no-open".to_owned()]).unwrap(),
            LaunchOptions {
                open_browser: false
            }
        );
        assert!(LaunchOptions::from_arguments(["--unknown".to_owned()]).is_err());
    }

    #[test]
    fn public_host_and_origin_are_stable_loopback_values() {
        assert_eq!(PUBLIC_HOST, "127.0.0.1:5173");
        assert_eq!(PUBLIC_ORIGIN, "http://127.0.0.1:5173");

        let mut headers = HeaderMap::new();
        assert!(!header_matches(&headers, header::HOST, PUBLIC_HOST));
        assert!(!header_matches(&headers, header::ORIGIN, PUBLIC_ORIGIN));

        headers.insert(header::HOST, HeaderValue::from_static(PUBLIC_HOST));
        headers.insert(header::ORIGIN, HeaderValue::from_static(PUBLIC_ORIGIN));
        assert!(header_matches(&headers, header::HOST, PUBLIC_HOST));
        assert!(header_matches(&headers, header::ORIGIN, PUBLIC_ORIGIN));
    }
}
