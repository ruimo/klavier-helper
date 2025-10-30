use std::{cell::RefCell, collections::{vec_deque, VecDeque}, rc::{Rc, Weak}, time::SystemTime};

/// Represents the severity level of a log entry
#[derive(PartialEq, Debug, Clone)]
pub enum Severity {
    /// Informational message, lowest severity
    Info,
    /// Warning message, medium severity
    Warn,
    /// Error message, highest severity
    Err,
}

/// A log entry with timestamp, severity level, and message text
#[derive(PartialEq, Debug, Clone)]
pub struct Entry {
    /// When the log entry was created
    pub time: SystemTime,
    /// The severity level of the log entry
    pub severity: Severity,
    /// The message text
    pub text: String,
}

impl Entry {
    /// Creates a new info-level log entry with the current timestamp
    pub fn info<S: Into<String>>(text: S) -> Self {
        Self::new(SystemTime::now(), Severity::Info, text)
    }

    /// Creates a new warning-level log entry with the current timestamp
    pub fn warn<S: Into<String>>(text: S) -> Self {
        Self::new(SystemTime::now(), Severity::Warn, text)
    }

    /// Creates a new error-level log entry with the current timestamp
    pub fn err<S: Into<String>>(text: S) -> Self {
        Self::new(SystemTime::now(), Severity::Err, text)
    }

    /// Creates a new log entry with the specified timestamp, severity, and text
    pub fn new<S: Into<String>>(time: SystemTime, severity: Severity, text: S) -> Self {
        Self {
            time,
            severity,
            text: text.into(),
        }
    }
}

/// Observer interface for log entries
///
/// Implement this trait to receive notifications when new log entries are added
pub trait Observer {
    /// Called when a new log entry is added
    fn notify(&mut self, entry: &Entry);
}

/// A log storage with a fixed maximum size and observer pattern support
pub struct Logs {
    size: usize,
    logs: VecDeque<Entry>,
    observers: Vec<Weak<RefCell<dyn Observer>>>,
}

impl Logs {
    /// Creates a new log storage with the specified maximum size
    pub fn new(size: usize) -> Self {
        Self {
            size,
            logs: VecDeque::with_capacity(size),
            observers: vec![],
        }
    }
    
    /// Adds an observer that will be notified when new log entries are added
    ///
    /// The observer is stored as a weak reference, so it will be automatically
    /// removed when it's no longer referenced elsewhere
    pub fn add_observer(&mut self, observer: Rc<RefCell<dyn Observer>>) {
        self.observers.push(Rc::downgrade(&observer));
    }

    #[inline]
    fn trim(&mut self) {
        if self.size <= self.logs.len() {
            self.logs.remove(0);
        }
    }
    
    fn append_log(&mut self, e: Entry) {
        self.trim();
        
        // Simultaneously clean up dropped observers and notify active ones
        self.observers.retain(|observer| {
            if let Some(obs) = observer.upgrade() {
                obs.borrow_mut().notify(&e);
                true
            } else {
                false
            }
        });
        
        self.logs.push_back(e);
    }

    /// Adds an informational log entry with the given text
    pub fn info<S: Into<String>>(&mut self, text: S) {
        self.append_log(Entry::info(text));
    }

    /// Adds a warning log entry with the given text
    pub fn warn<S: Into<String>>(&mut self, text: S) {
        self.append_log(Entry::warn(text));
    }

    /// Adds an error log entry with the given text
    pub fn err<S: Into<String>>(&mut self, text: S) {
        self.append_log(Entry::err(text));
    }

    /// Returns an iterator over the log entries in oldest-first order
    ///
    /// You can call rev() on the returned iterator to get newest-first order.
    pub fn logs(&self) -> vec_deque::Iter<'_, Entry> {
        self.logs.iter()
    }
}

#[macro_export]
macro_rules! info {
    ($this:expr, $e:expr) => { $this.info($e.to_string()) };
    ($this:expr, $fmt:expr, $($arg:tt)*) => { $this.info(format!($fmt, $($arg)*)) };
}

#[macro_export]
macro_rules! warn {
    ($this:expr, $e:expr) => { $this.warn($e.to_string()) };
    ($this:expr, $fmt:expr, $($arg:tt)*) => { $this.warn(format!($fmt, $($arg)*)) };
}

#[macro_export]
macro_rules! err {
    ($this:expr, $e:expr) => { $this.err($e.to_string()) };
    ($this:expr, $fmt:expr, $($arg:tt)*) => { $this.err(format!($fmt, $($arg)*)) };
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::Severity;
    use super::{Entry, Observer};
    use super::Logs;

    #[test]
    fn empty() {
        let logs = Logs::new(5);
        assert_eq!(logs.logs().next(), None);
    }

    #[test]
    fn can_iter() {
        let mut logs = Logs::new(5);
        info!(logs, "Hello");
        err!(logs, "World {}", 1);
        let mut iter = logs.logs();
        assert_eq!(iter.next().map(|e| e.text.clone()), Some("Hello".to_owned()));
        assert_eq!(iter.next().map(|e| e.text.clone()), Some("World 1".to_owned()));
        assert_eq!(iter.next(), None);

        let mut riter = logs.logs().rev();
        assert_eq!(riter.next().map(|e| e.text.clone()), Some("World 1".to_owned()));
        assert_eq!(riter.next().map(|e| e.text.clone()), Some("Hello".to_owned()));
        assert_eq!(riter.next(), None);
    }
    
    #[test]
    fn can_observe() {
        let mut logs = Logs::new(5);
        struct MyObserver {
            e: Vec<Entry>,
        }

        impl Observer for MyObserver {
            fn notify(&mut self, entry: &Entry) {
                self.e.push(entry.clone());
            }
        }
        
        let obs = Rc::new(RefCell::new(MyObserver { e: vec![] }));
        logs.add_observer(obs.clone());

        info!(logs, "Hello");
        err!(logs, "World");
        
        assert_eq!(obs.borrow().e[0].text, "Hello");
        assert_eq!(obs.borrow().e[0].severity, Severity::Info);

        assert_eq!(obs.borrow().e[1].text, "World");
        assert_eq!(obs.borrow().e[1].severity, Severity::Err);
    }
}