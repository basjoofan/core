use am::command;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
    /// Interpret and run
    Run {
        /// Source path
        #[command()]
        path: Option<PathBuf>,
    },
    /// Run a request or a function
    Call {
        /// Call name
        #[command()]
        name: String,
    },
    /// Run the tests
    Test {
        /// Annotation tag
        #[command()]
        #[arg(default_value = "test")]
        tag: String,
    },
}

fn main() {
    let interface = Interface::parse();
    if interface.verbose {
        println!("Use verbose output...");
    }
    match interface.command {
        Some(Commands::Run { path }) => {
            command::run(path);
        }
        Some(Commands::Call { name }) => {
            command::call(name);
        }
        Some(Commands::Test { tag }) => {
            command::test(tag);
        }
        None => {
            command::start();
        }
    }
}
