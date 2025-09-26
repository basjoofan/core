use std::time::Duration;
use std::time::SystemTime;

pub struct Time {
    pub start: Duration,
    pub end: Duration,
    pub total: Duration,
    pub resolve: Duration,
    pub connect: Duration,
    pub write: Duration,
    pub delay: Duration,
    pub read: Duration,
}

impl Default for Time {
    fn default() -> Self {
        let now = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        Self {
            start: now,
            end: now,
            total: Duration::ZERO,
            resolve: Duration::ZERO,
            connect: Duration::ZERO,
            write: Duration::ZERO,
            delay: Duration::ZERO,
            read: Duration::ZERO,
        }
    }
}
