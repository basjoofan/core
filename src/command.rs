use crate::Compiler;
use crate::Expr;
use crate::Machine;
use crate::Parser;
use std::collections::HashSet;
use std::io::stdin;
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

pub const NAME: &str = env!("CARGO_PKG_NAME");

pub fn repl() {
    let mut lines = stdin().lock().lines();
    let mut globals = Vec::new();
    let mut compiler = Compiler::new();
    loop {
        if let Some(Ok(text)) = lines.next() {
            if text == "exit" {
                break;
            }
            if text.trim().is_empty() {
                continue;
            }
            match Parser::new(&text).parse() {
                Ok(source) => match compiler.compile(source) {
                    Ok(opcodes) => {
                        let mut machine = Machine::new(compiler.consts(), &mut globals, opcodes);
                        machine.run();
                        println!("{}", machine.past());
                    }
                    Err(message) => println!("{}", message),
                },
                Err(message) => println!("{}", message),
            };
        }
    }
}

pub fn eval(text: String) {
    let mut globals = Vec::new();
    let mut compiler = Compiler::new();
    match Parser::new(&text).parse() {
        Ok(source) => match compiler.compile(source) {
            Ok(opcodes) => {
                let mut machine = Machine::new(compiler.consts(), &mut globals, opcodes);
                machine.run();
                println!("{}", machine.past());
            }
            Err(message) => println!("{}", message),
        },
        Err(message) => println!("{}", message),
    };
}

pub fn run(path: Option<PathBuf>) {
    let text = read_to_string(path.unwrap_or(std::env::current_dir().unwrap()));
    eval(text);
}

pub fn test(name: Option<String>, concurrency: u32, duration: Duration, iterations: u32, path: Option<PathBuf>) {
    let text = read_to_string(path.unwrap_or(std::env::current_dir().unwrap()));
    let mut globals = Vec::new();
    let mut compiler = Compiler::new();
    let source = match Parser::new(&text).parse() {
        Ok(source) => source,
        Err(message) => {
            println!("{}", message);
            return;
        }
    };
    let mut tests = HashSet::new();
    for expr in source.iter() {
        if let Expr::Test(name, _) = expr {
            tests.insert(name.to_owned());
        }
    }
    match compiler.compile(source) {
        Ok(opcodes) => {
            Machine::new(compiler.consts(), &mut globals, opcodes).run();
        }
        Err(message) => {
            println!("{}", message);
            return;
        }
    };
    let mut handles = Vec::new();
    match name {
        Some(name) => {
            if tests.contains(&name) {
                match compiler.compile(vec![Expr::Call(Box::new(Expr::Ident(name)), Vec::new())]) {
                    Ok(opcodes) => {
                        let continuous = Arc::new(AtomicBool::new(true));
                        let iterations = iterations / concurrency;
                        for _ in 0..concurrency {
                            let continuous = continuous.clone();
                            let consts = compiler.consts().clone();
                            let mut globals = globals.clone();
                            let opcodes = opcodes.clone();
                            handles.push(std::thread::spawn(move || {
                                let mut i = iterations;
                                while continuous.load(Ordering::Relaxed) && i > 0 {
                                    Machine::new(&consts, &mut globals, opcodes.clone()).run();
                                    i -= 1;
                                }
                            }));
                        }
                        // handle interrupt signal
                        handle_interrupt(continuous.clone());
                        // completed after thread sleep duration
                        std::thread::spawn(move || {
                            std::thread::sleep(duration);
                            continuous.store(false, Ordering::Relaxed)
                        });
                    }
                    Err(message) => println!("{}", message),
                }
            } else {
                println!("Test not found: {}", name);
            }
        }
        None => {
            for name in tests {
                let mut globals = globals.clone();
                let mut compiler = compiler.clone();
                handles.push(std::thread::spawn(move || {
                    match compiler.compile(vec![Expr::Call(Box::new(Expr::Ident(name)), Vec::new())]) {
                        Ok(opcodes) => {
                            Machine::new(compiler.consts(), &mut globals, opcodes).run();
                        }
                        Err(message) => println!("{}", message),
                    }
                }));
            }
        }
    }
    for handle in handles {
        let _ = handle.join();
    }
}

fn read_to_string(path: PathBuf) -> String {
    let mut text = String::new();
    read(path, &mut text).expect("Could not read source file");
    text
}

fn read(path: PathBuf, text: &mut String) -> std::io::Result<()> {
    if path.is_dir() {
        let mut entries = std::fs::read_dir(path)?
            .map(|r| r.map(|e| e.path()))
            .collect::<Result<Vec<PathBuf>, std::io::Error>>()?;
        entries.sort();
        for entry in entries {
            read(entry, text)?;
        }
    } else if path.is_file() && path.extension() == Some(std::ffi::OsStr::new(NAME)) {
        text.push_str(&std::fs::read_to_string(path)?)
    }
    Ok(())
}

fn handle_interrupt(continuous: Arc<AtomicBool>) {
    let _ = ctrlc::set_handler(move || {
        continuous.store(false, Ordering::Relaxed);
    });
}
