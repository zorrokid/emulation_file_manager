use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

/// Initializes logging for the application.
///
/// Sets up two logging outputs:
/// - Console: Human-readable format for development/terminal use
/// - File: JSON format in `~/.local/share/efm-relm4-ui/logs/` for bug reports
///
/// Log files are rotated daily to prevent unbounded growth.
///
/// Default log level is "info" for production, but can be overridden with RUST_LOG:
/// - RUST_LOG=debug efm-relm4-ui
/// - RUST_LOG=service=trace,database=debug efm-relm4-ui
///
/// Returns a guard that must be kept alive for the duration of the program.
/// Dropping this guard will cause file logging to stop.
pub fn init_logging() -> tracing_appender::non_blocking::WorkerGuard {
    // Store logs in XDG-compliant location
    // Linux: ~/.local/share/efm-relm4-ui/logs/
    // macOS: ~/Library/Application Support/efm-relm4-ui/logs/
    // Windows: C:\Users\<User>\AppData\Local\efm-relm4-ui\logs\
    let log_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("efm-relm4-ui")
        .join("logs");
    
    // Create directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        eprintln!("Warning: Failed to create log directory at {}: {}", log_dir.display(), e);
        eprintln!("Logs will only be written to console.");
    }

    // Console output (human-readable, for when running from terminal)
    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(false)
        .with_level(true)
        .compact();

    // File output (JSON for bug reports, daily rotation)
    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        log_dir.clone(),
        "app.log"
    );
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .with_file(true)
        .with_line_number(true);

    // Default to info level for production
    // service and database at debug for more detailed troubleshooting
    // Allow RUST_LOG environment variable to override for debugging
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::new("info,service=debug,database=info,file_import=info,file_export=info")
        });

    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    println!("Application logs are being written to: {}", log_dir.display());
    println!("Log level: info (set RUST_LOG environment variable to change)");

    guard
}
