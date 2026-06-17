use chrono::{DateTime, Utc};

pub trait CronAdapter: Send + Sync {
    fn is_valid(&self, expr: &str) -> bool;
    fn next_run_date(&self, expr: &str, from: DateTime<Utc>) -> Option<DateTime<Utc>>;
}

pub struct CronExpressionAdapter;

impl CronAdapter for CronExpressionAdapter {
    fn is_valid(&self, expr: &str) -> bool {
        expr.parse::<cron::Schedule>().is_ok()
    }

    fn next_run_date(&self, expr: &str, _from: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let schedule: cron::Schedule = expr.parse().ok()?;
        schedule.upcoming(Utc).next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_is_valid_valid_expression() {
        let adapter = CronExpressionAdapter;
        assert!(adapter.is_valid("*/5 * * * * *"));
        assert!(adapter.is_valid("0 0 * * * *"));
    }

    #[test]
    fn test_is_valid_invalid_expression() {
        let adapter = CronExpressionAdapter;
        assert!(!adapter.is_valid("invalid"));
        assert!(!adapter.is_valid(""));
    }

    #[test]
    fn test_next_run_date_returns_some() {
        let adapter = CronExpressionAdapter;
        let from = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let next = adapter.next_run_date("*/5 * * * * *", from);
        assert!(next.is_some());
        assert!(next.unwrap() > from);
    }

    #[test]
    fn test_next_run_date_invalid_returns_none() {
        let adapter = CronExpressionAdapter;
        let from = Utc::now();
        let next = adapter.next_run_date("invalid", from);
        assert!(next.is_none());
    }
}
