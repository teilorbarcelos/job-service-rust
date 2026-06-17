use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Configuration(String),
    Connection(String),
    Job(String),
    Validation(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            AppError::Connection(msg) => write!(f, "Connection error: {}", msg),
            AppError::Job(msg) => write!(f, "Job error: {}", msg),
            AppError::Validation(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl From<String> for AppError {
    fn from(msg: String) -> Self {
        AppError::Job(msg)
    }
}
