use crate::Value;
use std::collections::HashMap;
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

impl Time {
    pub fn into_value(self) -> Value {
        let mut map = HashMap::new();
        map.insert(String::from("start"), Value::Integer(self.start.as_nanos() as i64));
        map.insert(String::from("end"), Value::Integer(self.end.as_nanos() as i64));
        map.insert(String::from("total"), Value::Integer(self.total.as_nanos() as i64));
        map.insert(String::from("resolve"), Value::Integer(self.resolve.as_nanos() as i64));
        map.insert(String::from("connect"), Value::Integer(self.connect.as_nanos() as i64));
        map.insert(String::from("write"), Value::Integer(self.write.as_nanos() as i64));
        map.insert(String::from("delay"), Value::Integer(self.delay.as_nanos() as i64));
        map.insert(String::from("read"), Value::Integer(self.read.as_nanos() as i64));
        Value::Map(map)
    }
}
