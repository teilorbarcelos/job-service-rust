use std::fmt;

#[derive(Debug, Clone)]
pub struct JobInfo {
    pub name: String,
    pub schedule: String,
    pub enabled: bool,
    pub description: String,
}

impl JobInfo {
    pub fn new(name: String, schedule: String, enabled: bool, description: String) -> Self {
        Self {
            name,
            schedule,
            enabled,
            description,
        }
    }
}

impl fmt::Display for JobInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "JobInfo {{ name: {}, schedule: {}, enabled: {}, description: {} }}",
            self.name, self.schedule, self.enabled, self.description
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_info_new() {
        let info = JobInfo::new("test".into(), "*/5 * * * *".into(), true, "A test job".into());
        assert_eq!(info.name, "test");
        assert_eq!(info.schedule, "*/5 * * * *");
        assert!(info.enabled);
        assert_eq!(info.description, "A test job");
    }

    #[test]
    fn test_display() {
        let info = JobInfo::new("x".into(), "* * * * * *".into(), true, "desc".into());
        let s = info.to_string();
        assert!(s.contains("x"));
        assert!(s.contains("desc"));
    }

    #[test]
    fn test_job_info_disabled() {
        let info = JobInfo::new("a".into(), "* * * * *".into(), false, "".into());
        assert!(!info.enabled);
    }
}
