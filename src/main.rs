use am::command;
use clap::{Parser, Subcommand};
use std::{path::PathBuf, time::Duration};

#[derive(Parser)]
#[command(name = command::NAME, version, about, long_about = None)]
struct Interface {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Evaluate source code
    Eval {
        /// Source code
        #[command()]
        text: String,
    },
    /// Run the tests
    Test {
        /// Test name
        #[command()]
        name: Option<String>,
        /// Threads
        #[arg(short, long, default_value_t = 1)]
        threads: u32,
        /// Duration
        #[arg(short, long, value_parser = humantime::parse_duration)]
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
    },
}

fn main() {
    let interface = Interface::parse();
    match interface.command {
        Some(Commands::Eval { text }) => {
            command::eval(text, None);
        }
        Some(Commands::Test {
            name,
            threads,
            duration,
            number,
            path,
            record,
        }) => {
            let (duration, number) = match duration {
                Some(duration) => (duration, u32::MAX),
                None => (Duration::MAX, number),
            };
            command::test(name, threads, duration, number, path, record);
        }
        None => {
            command::repl();
        }
    }
}
