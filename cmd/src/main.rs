mod command;
use clap::{Parser, Subcommand};
use std::{path::PathBuf, time::Duration};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Interface {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Evaluate codes
    Eval {
        /// Source code
        #[command()]
        text: String,
    },
    /// Run tests
    Test {
        /// Test name
        #[command()]
        name: Option<String>,
        /// Threads
        #[arg(short, long, default_value_t = 1)]
        threads: u32,
        /// Duration
        #[arg(short, long, value_parser = parse_duration)]
        duration: Option<Duration>,
        /// Number
        #[arg(short, long, default_value_t = 1)]
        number: u32,
        /// Path
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// Record
        #[arg(short, long)]
        record: Option<PathBuf>,
        /// Stat
        #[arg(short, long)]
        stat: bool,
    },
}

#[tokio::main]
async fn main() {
    let interface = Interface::parse();
    match interface.command {
        Some(Commands::Eval { text }) => {
            command::eval(text, None).await;
        }
        Some(Commands::Test {
            name,
            threads,
            duration,
            number,
            path,
            record,
            stat,
        }) => {
            let (duration, number) = match duration {
                Some(duration) => (duration, u32::MAX),
                None => (Duration::MAX, number),
            };
            command::test(name, threads, duration, number, path, record, stat).await;
        }
        None => {
            command::repl().await;
        }
    }
}

fn parse_duration(s: &str) -> Result<Duration, String> {
    let mut chars = s.chars();
    let last = chars.next_back();
    let value = match chars.as_str().parse::<u64>() {
        Ok(value) => value,
        Err(_) => return Err(format!("Invalid number: {}", chars.as_str())),
    };
    let duration = match last {
        Some('s') => Duration::from_secs(value),
        Some('m') => Duration::from_secs(value * 60),
        Some('h') => Duration::from_secs(value * 3600),
        Some(c) => return Err(format!("Unknown time unit: {}, supported units: s, m, h", c)),
        None => return Err("Time unit needed, for example 1s or 2m".to_string()),
    };
    Ok(duration)
}
