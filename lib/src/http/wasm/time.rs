pub struct Time {
    pub start: f64,
    pub end: f64,
    pub total: f64,
    pub resolve: f64,
    pub connect: f64,
    pub write: f64,
    pub delay: f64,
    pub read: f64,
}

impl Default for Time {
    fn default() -> Self {
        Self {
            start: 0.0,
            end: 0.0,
            total: 0.0,
            resolve: 0.0,
            connect: 0.0,
            write: 0.0,
            delay: 0.0,
            read: 0.0,
        }
    }
}
