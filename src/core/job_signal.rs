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
