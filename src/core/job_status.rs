use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Success,
    Failed,
    Cancelled,
    Timeout,
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobStatus::Success => write!(f, "success"),
            JobStatus::Failed => write!(f, "failed"),
            JobStatus::Cancelled => write!(f, "cancelled"),
            JobStatus::Timeout => write!(f, "timeout"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_success() {
        assert_eq!(JobStatus::Success.to_string(), "success");
    }

    #[test]
    fn test_display_failed() {
        assert_eq!(JobStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_display_cancelled() {
        assert_eq!(JobStatus::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn test_display_timeout() {
        assert_eq!(JobStatus::Timeout.to_string(), "timeout");
    }

    #[test]
    fn test_debug() {
        assert_eq!(format!("{:?}", JobStatus::Success), "Success");
    }

    #[test]
    fn test_clone() {
        let a = JobStatus::Success;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn test_copy() {
        let a = JobStatus::Failed;
        let _b = a;
        let _c = a;
    }
}
