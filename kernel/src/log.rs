#![allow(dead_code)]

#[derive(PartialEq, PartialOrd)]
pub(crate) enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

pub(crate) static LOG_LEVEL: LogLevel = LogLevel::Info;

#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        if $level >= $crate::log::LOG_LEVEL {
            println!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        log!($crate::log::LogLevel::Debug, "[DEBUG] {}", format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        log!($crate::log::LogLevel::Info, "[INFO] {}", format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        log!($crate::log::LogLevel::Warn, "[WARN] {}", format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        log!($crate::log::LogLevel::Error, "[ERROR] {}", format_args!($($arg)*));
    };
}