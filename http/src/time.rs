use std::time::Duration;

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
        Self {
            start: Duration::ZERO,
            end: Duration::ZERO,
            total: Duration::ZERO,
            resolve: Duration::ZERO,
            connect: Duration::ZERO,
            write: Duration::ZERO,
            delay: Duration::ZERO,
            read: Duration::ZERO,
        }
    }
}
