use personal_dashboard::config::{ConfigService, RuntimeSchema};
use personal_dashboard::server::LaunchOptions;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Personal Dashboard failed: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let launch_options = LaunchOptions::from_arguments(std::env::args().skip(1))
        .map_err(|error| format!("{error}\n\n{}", LaunchOptions::USAGE))?;
    if launch_options.show_help {
        println!("{}", LaunchOptions::USAGE);
        return Ok(());
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "personal_dashboard=info".into()),
        )
        .init();

    let runtime_schema = RuntimeSchema::materialize()?;
    let (config_service, reload_service) = ConfigService::new(runtime_schema);
    if let Some(setup) = &launch_options.setup {
        config_service.apply_setup(setup.clone())?;
    }
    tokio::spawn(reload_service.run());
    personal_dashboard::server::serve(launch_options, config_service).await
}
