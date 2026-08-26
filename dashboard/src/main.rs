use personal_dashboard::config::{ConfigService, RuntimeSchema};
use personal_dashboard::server::LaunchOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let launch_options = LaunchOptions::from_arguments(std::env::args().skip(1))?;
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "personal_dashboard=info".into()),
        )
        .init();

    let runtime_schema = RuntimeSchema::materialize()?;
    let (config_service, reload_service) = ConfigService::new(runtime_schema);
    tokio::spawn(reload_service.run());
    personal_dashboard::server::serve(launch_options, config_service).await
}
