use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub struct JobSignal {
    aborted: AtomicBool,
}

impl JobSignal {
    pub fn new() -> Self {
        Self {
            aborted: AtomicBool::new(false),
        }
    }

    pub fn abort(&self) {
        self.aborted.store(true, Ordering::SeqCst);
    }

    pub fn aborted(&self) -> bool {
        self.aborted.load(Ordering::SeqCst)
    }

    pub fn throw_if_aborted(&self) -> Result<(), super::errors::AppError> {
        if self.aborted() {
            return Err(super::errors::AppError::Job("Job was cancelled".to_string()));
        }
        Ok(())
    }
}

impl Default for JobSignal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_not_aborted() {
        let signal = JobSignal::new();
        assert!(!signal.aborted());
    }

    #[test]
    fn test_abort_sets_flag() {
        let signal = JobSignal::new();
        signal.abort();
        assert!(signal.aborted());
    }

    #[test]
    fn test_throw_if_aborted_ok() {
        let signal = JobSignal::new();
        assert!(signal.throw_if_aborted().is_ok());
    }

    #[test]
    fn test_throw_if_aborted_err() {
        let signal = JobSignal::new();
        signal.abort();
        assert!(signal.throw_if_aborted().is_err());
    }

    #[test]
    fn test_default() {
        let signal = JobSignal::default();
        assert!(!signal.aborted());
    }
}
