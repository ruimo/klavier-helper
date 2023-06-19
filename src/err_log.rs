use std::{time::{SystemTime}, collections::{VecDeque, vec_deque}};

#[derive(PartialEq, Debug)]
pub enum Severity {
    Info,
    Warn,
    Err,
}

#[derive(PartialEq, Debug)]
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
            text: text.into(),
        }
    }
}

pub struct Logs {
    size: usize,
    logs: VecDeque<Entry>,
}

impl Logs {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            logs: VecDeque::with_capacity(size),
        }
    }

    #[inline]
    fn trim(&mut self) {
        if self.size <= self.logs.len() {
            self.logs.remove(0);
        }
    }

    pub fn info<S: Into<String>>(&mut self, text: S) {
        self.trim();
        self.logs.push_back(Entry::info(text));
    }

    pub fn warn<S: Into<String>>(&mut self, text: S) {
        self.trim();
        self.logs.push_back(Entry::warn(text));
    }

    pub fn err<S: Into<String>>(&mut self, text: S) {
        self.trim();
        self.logs.push_back(Entry::err(text));
    }

    /// Oldest first order. You can call rev() to reverse the order.
    pub fn logs(&self) -> vec_deque::Iter<'_, Entry> {
        self.logs.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::Logs;

    #[test]
    fn empty() {
        let logs = Logs::new(5);
        assert_eq!(logs.logs().next(), None);
    }

    #[test]
    fn can_iter() {
        let mut logs = Logs::new(5);
        logs.info("Hello");
        logs.err("World");
        let mut iter = logs.logs();
        assert_eq!(iter.next().map(|e| e.text.clone()), Some("Hello".to_owned()));
        assert_eq!(iter.next().map(|e| e.text.clone()), Some("World".to_owned()));
        assert_eq!(iter.next(), None);

        let mut riter = logs.logs().rev();
        assert_eq!(riter.next().map(|e| e.text.clone()), Some("World".to_owned()));
        assert_eq!(riter.next().map(|e| e.text.clone()), Some("Hello".to_owned()));
        assert_eq!(riter.next(), None);
    }
}