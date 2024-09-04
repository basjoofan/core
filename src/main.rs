use am::command;
use clap::{Parser, Subcommand};
use std::{path::PathBuf, time::Duration};

#[derive(Parser)]
#[command(name = command::NAME, version, about, long_about = None)]
struct Interface {
    /// Use verbose output
    #[arg(short, long)]
    verbose: bool,
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
    /// Interpret and run
    Run {
        /// Source path
        #[command()]
        path: Option<PathBuf>,
    },
    /// Run the tests
    Test {
        /// Test name
        #[command()]
        name: Option<String>,
        /// Concurrency
        #[arg(short, long, default_value_t = 1)]
        concurrency: u32,
        /// Duration
        #[arg(short, long, value_parser = humantime::parse_duration)]
        duration: Option<Duration>,
        /// Iterations
        #[arg(short, long, default_value_t = 1)]
        iterations: u32,
        /// File
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
}

fn main() {
    let interface = Interface::parse();
    if interface.verbose {
        println!("Use verbose output...");
    }
    match interface.command {
        Some(Commands::Eval { text }) => {
            command::eval(text);
        }
        Some(Commands::Run { path }) => {
            command::run(path);
        }
        Some(Commands::Test {
            name,
            concurrency,
            duration,
            iterations,
            file,
        }) => {
            let (duration, iterations) = match duration {
                Some(duration) => (duration, u32::MAX),
                None => (Duration::MAX, iterations),
            };
            command::test(name, concurrency, duration, iterations, file);
        }
        None => {
            command::repl();
        }
    }
}
