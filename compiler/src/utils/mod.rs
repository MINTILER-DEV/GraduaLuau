use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

impl Display for LogLevel {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Trace => formatter.write_str("trace"),
            Self::Debug => formatter.write_str("debug"),
            Self::Info => formatter.write_str("info"),
            Self::Warning => formatter.write_str("warning"),
            Self::Error => formatter.write_str("error"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Logger {
    minimum_level: LogLevel,
}

impl Logger {
    pub fn new(minimum_level: LogLevel) -> Self {
        Self { minimum_level }
    }

    pub fn log(&self, level: LogLevel, message: impl AsRef<str>) {
        if level >= self.minimum_level {
            eprintln!("[{level}] {}", message.as_ref());
        }
    }
}
