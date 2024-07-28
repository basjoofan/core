
use std::time::Duration;
use std::time::Instant;

#[derive(Debug)]
pub struct Time {
    pub start: Instant,
    pub end: Instant,
    pub total: Duration,
    pub resolve: Duration,
    pub connect: Duration,
    pub write: Duration,
    pub read: Duration,
    pub delay: Duration,
}

impl Time {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start: now,
            end: now,
            total: Duration::ZERO,
            resolve: Duration::ZERO,
            connect: Duration::ZERO,
            write: Duration::ZERO,
            read: Duration::ZERO,
            delay: Duration::ZERO,
        }
    }
}
