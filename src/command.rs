use crate::Context;
use crate::Source;
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
    let mut context = Context::new();
    let mut source = Source::new();
    loop {
        if let Some(Ok(text)) = lines.next() {
            if text == "exit" {
                break;
            }
            if text.trim().is_empty() {
                continue;
            }
            match source.load(&text) {
                Ok(exprs) => match source.eval(&exprs, &mut context) {
                    Ok(value) => {
                        println!("{}", value);
                    }
                    Err(error) => println!("{}", error),
                },
                Err(error) => println!("{}", error),
            }
        }
    }
}

pub fn eval(text: String) {
    let mut context = Context::new();
    let mut source = Source::new();
    match source.load(&text) {
        Ok(exprs) => match source.eval(&exprs, &mut context) {
            Ok(value) => {
                println!("{}", value);
            }
            Err(error) => println!("{}", error),
        },
        Err(error) => println!("{}", error),
    }
}

pub fn run(path: Option<PathBuf>) {
    let text = read_to_string(path.unwrap_or(std::env::current_dir().unwrap()));
    eval(text);
}

pub fn test(name: Option<String>, concurrency: u32, duration: Duration, iterations: u32, path: Option<PathBuf>) {
    let text = read_to_string(path.unwrap_or(std::env::current_dir().unwrap()));
    let mut context = Context::new();
    let mut source = Source::new();
    match source.load(&text) {
        Ok(exprs) => match source.eval(&exprs, &mut context) {
            Ok(value) => {
                println!("{}", value);
            }
            Err(error) => {
                println!("{}", error);
                return;
            }
        },
        Err(error) => {
            println!("{}", error);
            return;
        }
    }
    let mut handles = Vec::new();
    let source = Arc::new(source);
    let tests = source.tests();
    match name {
        Some(name) => {
            if tests.contains(&name) {
                let name = Arc::new(name);
                let continuous = Arc::new(AtomicBool::new(true));
                let iterations = iterations / concurrency;
                for _ in 0..concurrency {
                    let continuous = continuous.to_owned();
                    let mut context = context.to_owned();
                    let source = source.to_owned();
                    let name = name.to_owned();
                    handles.push(std::thread::spawn(move || {
                        let mut i = iterations;
                        while continuous.load(Ordering::Relaxed) && i > 0 {
                            match source.test(name.as_str(), &mut context) {
                                Ok(_) => {}
                                Err(error) => {
                                    println!("{}", error);
                                    break;
                                }
                            }
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
            } else {
                println!("Test not found: {}", name)
            }
        }
        None => {
            for name in tests {
                let mut context = context.to_owned();
                let source = source.to_owned();
                handles.push(std::thread::spawn(move || match source.test(name.as_str(), &mut context) {
                    Ok(_) => {}
                    Err(error) => {
                        println!("{}", error);
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
