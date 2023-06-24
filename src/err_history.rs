use std::{time::{Duration, SystemTime}, alloc::System};

pub enum Severity {
    Info,
    Warn,
    Err,
}

pub struct Entry {
    pub time: SystemTime,
    pub severity: Severity,
    pub text: String,
}

impl Entry {
    pub fn info<S: Into<String>>(text: S) -> Self {
        Self::new(SystemTime::now(), Severity::Info, text)
    }

    pub fn warn<S: Into<String>>(text: S) -> Self {
        Self::new(SystemTime::now(), Severity::Warn, text)
    }

    pub fn err<S: Into<String>>(text: S) -> Self {
        Self::new(SystemTime::now(), Severity::Err, text)
    }

    pub fn new<S: Into<String>>(time: SystemTime, severity: Severity, text: S) -> Self {
        Self {
            time,
            severity,
            text: text.into();
        }
    }
}

pub struct Logs {
    size: usize,
    logs: Vec<Entry>,
}

impl Logs {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            logs: Vec::with_capacity(size),
        }
    }

    pub fn last_update

}