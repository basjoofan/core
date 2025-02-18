use std::collections::HashMap;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

struct Stat {
    count: u128,
    sum: u128,
    avg: u128,
    max: u128,
    min: u128,
    failed: u128,
}

impl Stat {
    pub fn new(millis: u128) -> Self {
        let (count, failed) = if millis > 0 { (1, 0) } else { (0, 1) };
        Stat {
            count,
            sum: millis,
            avg: millis,
            max: millis,
            min: millis,
            failed,
        }
    }
    pub fn add(&mut self, millis: u128) {
        if millis == 0 {
            self.failed += 1;
        } else {
            self.count += 1;
            self.sum += millis;
            self.avg = self.sum / self.count;
            if millis > self.max {
                self.max = millis;
            }
            if self.min == 0 || millis < self.min {
                self.min = millis;
            }
        }
    }
}

impl Display for Stat {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "count={}  sum={}  avg={}  max={}  min={}  failed={}",
            self.count, self.sum, self.avg, self.max, self.min, self.failed
        )
    }
}

#[derive(Default)]
pub struct Stats {
    stats: HashMap<String, Stat>,
}

impl Stats {
    pub fn add(&mut self, name: &String, millis: u128) {
        match self.stats.get_mut(name) {
            Some(stat) => {
                stat.add(millis);
            }
            None => {
                self.stats.insert(name.to_owned(), Stat::new(millis));
            }
        };
    }
}

impl Display for Stats {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for (name, stat) in self.stats.iter() {
            writeln!(f, "{} {}", name, stat)?
        }
        Ok(())
    }
}
