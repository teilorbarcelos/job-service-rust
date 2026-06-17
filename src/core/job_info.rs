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
