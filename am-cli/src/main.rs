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
    /// Blow a test
    Blow {
        /// Test name
        #[command()]
        name: String,
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
    /// Run the tests
    Test {
        /// Annotation tag
        #[command()]
        #[arg(default_value = "test")]
        tag: String,
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
        Some(Commands::Blow {
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
            command::blow(name, concurrency, duration, iterations, file);
        }
        Some(Commands::Test { tag, file }) => {
            command::test(tag, file);
        }
        None => {
            command::repl();
        }
    }
}
