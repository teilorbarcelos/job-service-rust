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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_configuration() {
        let err = AppError::Configuration("missing key".to_string());
        assert_eq!(err.to_string(), "Configuration error: missing key");
    }

    #[test]
    fn test_display_connection() {
        let err = AppError::Connection("refused".to_string());
        assert_eq!(err.to_string(), "Connection error: refused");
    }

    #[test]
    fn test_display_job() {
        let err = AppError::Job("failed".to_string());
        assert_eq!(err.to_string(), "Job error: failed");
    }

    #[test]
    fn test_display_validation() {
        let err = AppError::Validation("bad value".to_string());
        assert_eq!(err.to_string(), "Validation error: bad value");
    }

    #[test]
    fn test_implements_error() {
        fn check<T: std::error::Error>() {}
        check::<AppError>();
    }

    #[test]
    fn test_from_string() {
        let err: AppError = "oops".to_string().into();
        assert_eq!(err.to_string(), "Job error: oops");
    }
}
