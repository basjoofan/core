use super::evaluator::eval_call_name;
use super::parser::Parser;
use super::record;
use super::record::Record;
use super::stat::Stats;
use super::syntax::Expr;
use super::value::Context;
use super::value::Value;
use std::io::stdin;
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Duration;

pub const NAME: &str = env!("CARGO_PKG_NAME");

pub fn repl() {
    let mut lines = stdin().lock().lines();
    let mut context = Context::default();
    loop {
        if let Some(line) = lines.next() {
            if let Ok(text) = line {
                if text == "exit" {
                    break;
                }
                let source = Parser::new(&text).parse();
                print_error(source.eval(&mut context));
            }
        }
    }
}

pub fn eval(text: String) {
    let mut context = Context::default();
    let source = Parser::new(&text).parse();
    print_error(source.eval(&mut context));
}

pub fn run(path: Option<PathBuf>) {
    let text = read_to_string(path.unwrap_or(std::env::current_dir().unwrap()));
    let mut context = Context::default();
    let source = Parser::new(&text).parse();
    print_error(source.eval(&mut context));
}

pub fn blow(name: String, concurrency: u32, duration: Duration, iterations: u32, file: Option<PathBuf>) {
    let text = read_to_string(std::env::current_dir().unwrap());
    let mut context = Context::default();
    let source = Parser::new(&text).parse();
    print_error(source.eval(&mut context));
    let (sender, receiver) = mpsc::channel();
    let continuous = Arc::new(AtomicBool::new(true));
    let iterations = iterations / concurrency;
    for _ in 0..concurrency {
        let continuous = continuous.clone();
        let mut context = context.clone();
        context.set_sender(&sender);
        context.set_group(&name);
        let name = name.clone();
        std::thread::spawn(move || {
            let mut i = iterations;
            while continuous.load(Ordering::Relaxed) && i > 0 {
                print_error(eval_call_name(&name, &mut context));
                i -= 1;
            }
        });
    }
    handle_ctrlc(continuous.clone());
    std::thread::spawn(move || {
        std::thread::sleep(duration);
        continuous.store(false, Ordering::Relaxed)
    });
    std::mem::drop(sender);
    process_record(receiver, file);
}

pub fn test(tag: String, file: Option<PathBuf>) {
    let text = read_to_string(std::env::current_dir().unwrap());
    let mut context = Context::default();
    let source = Parser::new(&text).parse();
    print_error(source.eval(&mut context));
    let (sender, receiver) = mpsc::channel();
    for call in source.requests.into_iter().chain(source.functions.into_iter()) {
        if let (Some(tags), Some(name)) = match call {
            Expr::Function(_, Some(tags), Some(name), _, _) => (Some(tags), Some(name)),
            Expr::Request(_, Some(tags), name, _, _) => (Some(tags), Some(name)),
            _ => (None, None),
        } {
            if tags.contains(&tag) {
                let mut context = context.clone();
                context.set_sender(&sender);
                context.set_group(&name);
                std::thread::spawn(move || {
                    print_error(eval_call_name(&name, &mut context));
                });
            }
        }
    }
    std::mem::drop(sender);
    process_record(receiver, file);
}

fn process_record(receiver: Receiver<Record>, file: Option<PathBuf>) {
    let schema = record::schema();
    let mut writer = record::writer(&schema, file);
    let mut stats = Stats::default();
    for record in receiver {
        // print record
        println!("=== TEST  {}/{}", record.group.name, record.request.name);
        let mut result = true;
        record.asserts.iter().for_each(|assert| {
            result &= assert.result;
            println!("{}", assert);
        });
        if result {
            println!(
                "--- PASS  {}/{} ({:?})",
                record.group.name, record.request.name, record.duration
            );
        } else {
            println!(
                "--- FAIL  {}/{} ({:?})",
                record.group.name, record.request.name, record.duration
            );
        }
        // store record
        if let Some(ref mut writer) = writer {
            let _ = writer.append(record.to_record(&schema));
        }
        // stat record
        stats.add(&record.request.name, record.duration.as_millis())
    }
    if let Some(ref mut writer) = writer {
        let _ = writer.flush();
    }
    print!("{}", stats);
}

fn print_error(value: Value) {
    if value.is_error() {
        println!("{}", value)
    }
}

fn read_to_string(path: PathBuf) -> String {
    let mut text = String::new();
    read(path, &mut text).expect("Could not read source file");
    text
}

fn read(path: PathBuf, text: &mut String) -> std::io::Result<()> {
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            if let Ok(entry) = entry {
                let path = entry.path();
                read(path, text)?;
            }
        }
    } else if path.is_file() && path.extension() == Some(std::ffi::OsStr::new(NAME)) {
        text.push_str(&std::fs::read_to_string(path)?)
    } else {
    }
    Ok(())
}

fn handle_ctrlc(continuous: Arc<AtomicBool>) {
    let _ = ctrlc::set_handler(move || {
        continuous.store(false, Ordering::Relaxed);
    });
}
