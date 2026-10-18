//! Logs go to a daily-rotated file in the data folder (spec §7), keeping a week.

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};

use crate::platform::paths::Paths;

/// Starts logging. Keep the guard alive until exit so buffered lines get written.
/// If the log folder can't be used, logs go to stderr instead.
pub fn init(paths: &Paths) -> Option<WorkerGuard> {
    let file = std::fs::create_dir_all(&paths.logs).ok().and_then(|()| {
        RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix("tally")
            .filename_suffix("log")
            .max_log_files(7)
            .build(&paths.logs)
            .ok()
    });
    log_panics();
    let builder = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_ansi(false);
    match file {
        Some(file) => {
            let (writer, guard) = tracing_appender::non_blocking(file);
            builder.with_writer(writer).init();
            Some(guard)
        }
        None => {
            builder.with_writer(std::io::stderr).init();
            tracing::warn!(folder = %paths.logs.display(), "could not open the log folder");
            None
        }
    }
}

/// tally has no console, so a panic would otherwise vanish: write it to the log too.
fn log_panics() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("(no message)");
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        tracing::error!(%location, "panic: {message}");
        default_hook(info);
    }));
}
