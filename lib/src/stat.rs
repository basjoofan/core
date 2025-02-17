use crate::Record;
use std::collections::HashMap;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;
use tokio::sync::mpsc::Receiver;

struct Stat {
    count: u128,
    sum: u128,
    avg: u128,
    max: u128,
    min: u128,
}

impl Stat {
    pub fn new(millis: u128) -> Self {
        Stat {
            count: 1,
            sum: millis,
            avg: millis,
            max: millis,
            min: millis,
        }
    }
    pub fn add(&mut self, millis: u128) {
        self.count += 1;
        self.sum += millis;
        self.avg = self.sum / self.count;
        if millis > self.max {
            self.max = millis;
        }
        if millis < self.min {
            self.min = millis;
        }
    }
}

impl Display for Stat {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "count={}  sum={}  avg={}  max={}  min={}",
            self.count, self.sum, self.avg, self.max, self.min
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

pub async fn receive(mut receiver: Receiver<Vec<Record>>) {
    let mut stats = Stats::default();
    while let Some(records) = receiver.recv().await {
        for record in records.iter() {
            stats.add(&record.name, record.time.total.as_millis());
        }
    }
    print!("{}", stats);
}
