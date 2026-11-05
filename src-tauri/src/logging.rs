use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

pub fn init(log_dir: &Path) -> Result<WorkerGuard, String> {
    std::fs::create_dir_all(log_dir).map_err(|error| error.to_string())?;

    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("app")
        .filename_suffix("log")
        .max_log_files(3)
        .build(log_dir)
        .map_err(|error| error.to_string())?;

    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,better_notepad_lib=debug"));

    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_target(true)
        .with_writer(writer)
        .with_filter(filter.clone());

    let registry = tracing_subscriber::registry().with(file_layer);

    #[cfg(debug_assertions)]
    {
        let console_layer = tracing_subscriber::fmt::layer()
            .with_ansi(true)
            .with_target(false)
            .with_writer(std::io::stderr)
            .with_filter(filter);
        registry.with(console_layer).init();
    }

    #[cfg(not(debug_assertions))]
    {
        registry.init();
    }

    Ok(guard)
}
