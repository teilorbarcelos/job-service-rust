use tracing_subscriber::EnvFilter;

pub fn setup_tracing(level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .with_target(true)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::util::SubscriberInitExt;

    #[test]
    fn test_env_filter_creation() {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("error"));
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .json()
            .with_target(true)
            .finish();
        // Try to init, may fail if already set
        let _ = subscriber.try_init();
    }

    #[test]
    fn test_setup_tracing_creates_filter_info() {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"));
        assert!(!filter.to_string().is_empty());
    }

    #[test]
    fn test_env_filter_debug() {
        let filter = EnvFilter::new("debug");
        assert_eq!(filter.to_string(), "debug");
    }

    #[test]
    fn test_env_filter_warn() {
        let filter = EnvFilter::new("warn");
        assert_eq!(filter.to_string(), "warn");
    }
}
