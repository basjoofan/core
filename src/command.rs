use crate::Context;
use crate::Parser;
use crate::Records;
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
    loop {
        if let Some(Ok(text)) = lines.next() {
            if text == "exit" {
                break;
            }
            if text.trim().is_empty() {
                continue;
            }
            context = eval(text, Some(context));
        }
    }
}

pub fn eval(text: String, context: Option<Context>) -> Context {
    let mut context = context.unwrap_or_default();
    match Parser::new(&text).parse() {
        Ok(Source { exprs, requests, .. }) => {
            context.extend(requests);
            match context.eval(&exprs, &mut Records::new()) {
                Ok(value) => {
                    println!("{}", value);
                }
                Err(error) => println!("{}", error),
            }
        }
        Err(error) => println!("{}", error),
    }
    context
}

pub fn test(name: Option<String>, concurrency: u32, duration: Duration, iterations: u32, path: Option<PathBuf>) {
    let text = read_to_string(path.unwrap_or(std::env::current_dir().unwrap()));
    let mut context = Context::new();
    let mut tests = match Parser::new(&text).parse() {
        Ok(Source { exprs, requests, tests }) => {
            context.extend(requests);
            match context.eval(&exprs, &mut Records::new()) {
                Ok(_) => tests,
                Err(error) => {
                    println!("{}", error);
                    return;
                }
            }
        }
        Err(error) => {
            println!("{}", error);
            return;
        }
    };
    let mut handles = Vec::new();
    match name {
        Some(name) => {
            match tests.remove(&name) {
                Some(test) => {
                    let name = Arc::new(name);
                    let test = Arc::new(test);
                    let continuous = Arc::new(AtomicBool::new(true));
                    let iterations = iterations / concurrency;
                    for _ in 0..concurrency {
                        let continuous = continuous.to_owned();
                        let _ = name.to_owned();
                        let test = test.to_owned();
                        let mut context = context.to_owned();
                        handles.push(std::thread::spawn(move || {
                            let mut i = iterations;
                            while continuous.load(Ordering::Relaxed) && i > 0 {
                                let mut records = Records::new();
                                match context.eval(test.as_ref(), &mut records) {
                                    Ok(_) => {}
                                    Err(error) => {
                                        println!("{}", error);
                                        break;
                                    }
                                }
                                println!("{}", records);
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
                None => {
                    println!("Test not found: {}", name)
                }
            }
        }
        None => {
            for (_, test) in tests {
                let mut context = context.to_owned();
                handles.push(std::thread::spawn(move || {
                    let mut records = Records::new();
                    match context.eval(test.as_ref(), &mut records) {
                        Ok(_) => {}
                        Err(error) => {
                            println!("{}", error);
                        }
                    }
                    println!("{}", records);
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
