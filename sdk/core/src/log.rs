pub use crate::abi::LogLevel as Level;
pub use crate::print;

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
