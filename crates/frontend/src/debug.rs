/// Centralized debug logging for the frontend
///
/// Provides a consistent debug output format without emojis or verbose formatting.
/// These macros only output in debug builds (when `debug_assertions` is enabled).
///
/// Log a debug message (only in debug builds)
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        eprintln!("[DEBUG] {}", format!($($arg)*))
    };
}

/// Log an error message (only in debug builds)
#[macro_export]
macro_rules! debug_error {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        eprintln!("[ERROR] {}", format!($($arg)*))
    };
}

/// Log an info message (only in debug builds)
#[macro_export]
macro_rules! debug_info {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        eprintln!("[INFO] {}", format!($($arg)*))
    };
}
