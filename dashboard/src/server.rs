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

use crate::chat::ChatRuntime;
use crate::config::ConfigService;
use crate::connections::{BootstrapTokenStore, ConnectionHub};
use crate::messages::{BootstrapResponse, OptifySetup, PROTOCOL_VERSION};
use crate::processes::ProcessService;
use crate::router::MessageRouter;
use crate::session::{self, SessionServices};
use crate::state::DashboardService;

pub const DEVELOPMENT_BACKEND_PORT: u16 = 3000;
pub const PUBLIC_PORT: u16 = 5173;
pub const PUBLIC_ORIGIN: &str = "http://127.0.0.1:5173";

const MAX_WEBSOCKET_MESSAGE_BYTES: usize = 64 * 1024;
const PUBLIC_HOST: &str = "127.0.0.1:5173";

#[derive(Clone)]
struct ApplicationState {
    sessions: SessionServices,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LaunchOptions {
    pub open_browser: bool,
    pub setup: Option<OptifySetup>,
    pub show_help: bool,
}

impl LaunchOptions {
    pub const USAGE: &str = concat!(
        "Usage: personal-dashboard [--no-open] [--config-dir <absolute-directory> ... --feature <name> ...]\n",
        "\n",
        "  --config-dir <absolute-directory>  Add an ordered Optify configuration directory.\n",
        "  --feature <name>                   Add an ordered Optify root feature.\n",
        "  --no-open                          Start without opening the browser.\n",
        "  --help                             Show this help.\n",
        "\n",
        "Supply both --config-dir and --feature for startup validation.\n",
        "Without them, apply configuration through the browser Options page.\n",
        "The server keeps setup in memory and never saves or restores it from disk.",
    );

    pub fn from_arguments(arguments: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments.into_iter();
        let mut config_directories = Vec::new();
        let mut features = Vec::new();
        let mut options = Self {
            open_browser: true,
            setup: None,
            show_help: false,
        };
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--config-dir" => {
                    config_directories.push(Self::argument_value(&argument, &mut arguments)?);
                }
                "--feature" => {
                    features.push(Self::argument_value(&argument, &mut arguments)?);
                }
                "--help" => options.show_help = true,
                "--no-open" => options.open_browser = false,
                _ => return Err(format!("unsupported argument: {argument}")),
            }
        }
        if config_directories.is_empty() != features.is_empty() {
            return Err("--config-dir and --feature must be supplied together".to_owned());
        }
        if !config_directories.is_empty() {
            options.setup = Some(OptifySetup {
                config_directories,
                features,
            });
        }
        Ok(options)
    }

    fn argument_value(
        flag: &str,
        arguments: &mut impl Iterator<Item = String>,
    ) -> Result<String, String> {
        arguments
            .next()
            .filter(|value| !value.trim().is_empty() && !value.starts_with('-'))
            .ok_or_else(|| format!("{flag} requires a value"))
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
    let mut chat = ChatRuntime::new(&config_service).start().await?;
    let (dashboard_service, dashboard_runtime) = DashboardService::new(config_service.clone());
    tokio::spawn(dashboard_runtime.run());
    let application = application(config_service.clone(), dashboard_service);
    let url = format!("http://{address}");
    if config_service.snapshot()?.is_some() {
        tracing::info!(%url, "Personal Dashboard is ready");
    } else {
        tracing::info!(%url, "Personal Dashboard is awaiting Optify setup");
    }

    #[cfg(not(debug_assertions))]
    if options.open_browser {
        webbrowser::open(&url)
            .map_err(|error| format!("could not open the default browser at {url}: {error}"))?;
    }

    tokio::select! {
        result = axum::serve(listener, application).into_future() => result?,
        result = chat.wait() => {
            result?;
            return Err("chat runtime stopped unexpectedly".into());
        }
    }
    Ok(())
}

fn application(
    config_service: Arc<ConfigService>,
    dashboard_service: Arc<DashboardService>,
) -> Router {
    let connections = ConnectionHub::new();
    let process_service = ProcessService::new(connections.clone());
    tokio::spawn(
        connections
            .clone()
            .publish_config_events(config_service.subscribe()),
    );
    tokio::spawn(
        connections
            .clone()
            .publish_dashboard_events(dashboard_service.subscribe()),
    );
    application_with_state(ApplicationState {
        sessions: SessionServices {
            config_service: config_service.clone(),
            connections,
            dashboard_service: dashboard_service.clone(),
            process_service: process_service.clone(),
            router: MessageRouter::new(config_service, dashboard_service, process_service),
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
            LaunchOptions {
                open_browser: true,
                setup: None,
                show_help: false,
            }
        );
        assert_eq!(
            LaunchOptions::from_arguments(["--no-open".to_owned()]).unwrap(),
            LaunchOptions {
                open_browser: false,
                setup: None,
                show_help: false,
            }
        );
        assert!(LaunchOptions::from_arguments(["--unknown".to_owned()]).is_err());
        assert!(
            LaunchOptions::from_arguments(["--help".to_owned()])
                .unwrap()
                .show_help
        );
    }

    #[test]
    fn parses_explicit_ordered_startup_configuration_and_rejects_incomplete_arguments() {
        let explicit = LaunchOptions::from_arguments(
            [
                "--config-dir",
                "/personal/configs",
                "--feature",
                "dashboard",
                "--config-dir",
                "/work/configs",
                "--feature",
                "work-dashboard",
                "--no-open",
            ]
            .map(str::to_owned),
        )
        .unwrap();
        let setup = explicit.setup.unwrap();
        assert_eq!(
            setup.config_directories,
            ["/personal/configs", "/work/configs"]
        );
        assert_eq!(setup.features, ["dashboard", "work-dashboard"]);
        assert!(!explicit.open_browser);
        for arguments in [
            vec!["--config-dir"],
            vec!["--feature"],
            vec!["--config-dir", "/personal/configs"],
            vec!["--feature", "dashboard"],
            vec!["--config-dir", "--feature", "dashboard"],
            vec!["--feature", " "],
            vec!["--setup-file", "/tmp/setup.json"],
        ] {
            assert!(
                LaunchOptions::from_arguments(arguments.into_iter().map(str::to_owned)).is_err()
            );
        }
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
