pub use crate::internal::print;
pub use borderless_abi::LogLevel as Level;

/// Internal type to represent the log-level
///
/// As the abi only allows integer types (and integers would look bad in e.g. json),
/// we wrap the type into this representation.
#[derive(serde::Serialize, serde::Deserialize)]
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LogLine {
    /// Timestamp
    ///
    /// Counted in nanoseconds since unix-epoch
    timestamp: u128,

    /// Log-Level
    level: LogLevel,

    /// Log-Message
    msg: String,
}

impl LogLine {
    /// Constructs a new [`LogLine`] from its raw components
    pub fn new(timestamp: u128, level: u32, msg: String) -> Self {
        let level = match level {
            0 => LogLevel::Trace,
            1 => LogLevel::Debug,
            2 => LogLevel::Info,
            3 => LogLevel::Warn,
            4 => LogLevel::Error,
            _ => unreachable!("log level should be constructed by borderless_abi::LogLevel"),
        };
        Self {
            timestamp,
            level,
            msg,
        }
    }
}

#[macro_export]
macro_rules! log {
    ($lvl:expr, $($arg:tt)+) => {
        {
            let buf = ::std::format!($($arg)+);
            $crate::log::print($lvl, buf);
        }
    };
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)+) => {
        {
            let buf = ::std::format!($($arg)+);
            $crate::log::print(Level::Info, buf);
        }
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)+) => {
        $crate::log!($crate::log::Level::Error, $($arg)+)
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)+) => {
        $crate::log!($crate::log::Level::Warn, $($arg)+)
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)+) => {
        $crate::log!($crate::log::Level::Info, $($arg)+)
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)+) => {
        $crate::log!($crate::log::Level::Debug, $($arg)+)
    };
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)+) => {
        $crate::log!($crate::log::Level::Trace, $($arg)+)
    };
}
