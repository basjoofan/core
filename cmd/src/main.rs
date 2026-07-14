mod command;
use clap::{Parser, Subcommand};
use lib::Source;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = env!("CARGO_BIN_NAME"), version)]
#[command(about = "Continuous test", long_about = None)]
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
        /// Test name, or @tag to run tests carrying that tag
        #[command()]
        target: Option<String>,
        /// Selected environment
        #[arg(long)]
        env: Option<String>,
        /// Path
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() {
    let interface = Interface::parse();
    match interface.command {
        Some(Commands::Eval { text }) => {
            let _ = command::eval(text, &mut Source::new(), HashMap::new()).await;
        }
        Some(Commands::Test { target, env, path }) => {
            if !command::test(target, env, path).await {
                std::process::exit(1);
            }
        }
        None => {
            command::repl().await;
        }
    }
}
