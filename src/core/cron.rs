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
