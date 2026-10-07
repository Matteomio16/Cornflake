#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use log;
use env_logger;

/// Release builds have no console, so logs go to %APPDATA%\app.cornflake\logs\cornflake.log
/// (rotated at 5 MB). Transcript text is only logged at debug level and never reaches this file.
fn log_target() -> env_logger::Target {
    if cfg!(debug_assertions) {
        return env_logger::Target::Stderr;
    }
    let dir = dirs::data_dir().unwrap_or_default().join("app.cornflake").join("logs");
    if std::fs::create_dir_all(&dir).is_err() {
        return env_logger::Target::Stderr;
    }
    let path = dir.join("cornflake.log");
    if std::fs::metadata(&path).map_or(false, |m| m.len() > 5 * 1024 * 1024) {
        let _ = std::fs::rename(&path, dir.join("cornflake.old.log"));
    }
    match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(f) => env_logger::Target::Pipe(Box::new(f)),
        Err(_) => env_logger::Target::Stderr,
    }
}

fn main() {
    std::env::set_var("RUST_LOG", "info");
    env_logger::Builder::from_default_env().target(log_target()).init();

    log::info!("Starting application...");
    app_lib::run();
}
