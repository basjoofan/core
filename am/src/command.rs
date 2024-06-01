use super::evaluator::eval;
use super::evaluator::eval_call_value;
use super::parser::Parser;
use super::record::Record;
use super::syntax::Expr;
use super::value::Context;
use super::value::Value;
use std::io::stdin;
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;

pub const NAME: &str = env!("CARGO_PKG_NAME");

pub fn start() {
    let mut lines = stdin().lock().lines();
    let mut context = Context::default();
    loop {
        if let Some(line) = lines.next() {
            if let Ok(input) = line {
                if input == "exit" {
                    break;
                }
                let script = Parser::new(&input).parse();
                print_error(eval(&script, &mut context));
            }
        }
    }
}

pub fn run(path: Option<PathBuf>) {
    let input = read_to_string(path.unwrap_or(std::env::current_dir().unwrap()));
    let mut context = Context::default();
    let script = Parser::new(&input).parse();
    print_error(eval(&script, &mut context));
}

pub fn call(name: String) {
    let input: String = read_to_string(std::env::current_dir().unwrap());
    let mut context = Context::default();
    let script = Parser::new(&input).parse();
    print_error(eval(&script, &mut context));
    let (sender, receiver) = mpsc::channel();
    context.set_sender(&sender);
    context.set_group(&name);
    std::thread::spawn(move || {
        print_error(eval_call(&name, &mut context));
    });
    std::mem::drop(sender);
    print_record(receiver);
}

pub fn test(tag: String) {
    let input = read_to_string(std::env::current_dir().unwrap());
    let mut context = Context::default();
    let script = Parser::new(&input).parse();
    print_error(eval(&script, &mut context));
    let (sender, receiver) = mpsc::channel();
    for call in script.requests.into_iter().chain(script.functions.into_iter()) {
        if let (Some(tags), Some(name)) = match call {
            Expr::Function(_, Some(tags), Some(name), _, _) => (Some(tags), Some(name)),
            Expr::Request(_, Some(tags), name, _, _) => (Some(tags), Some(name)),
            _ => (None, None),
        } {
            if tags.contains(&tag) {
                let mut context = context.clone();
                context.set_sender(&sender);
                context.set_group(&name);
                //let name = name.clone();
                std::thread::spawn(move || {
                    print_error(eval_call(&name, &mut context));
                });
            }
        }
    }
    std::mem::drop(sender);
    print_record(receiver);
}

fn eval_call(name: &String, context: &mut Context) -> Value {
    if let Some(value) = context.get(name) {
        eval_call_value(value, Vec::new(), context)
    } else {
        Value::Error(format!("call:{} not found", name))
    }
}

fn print_record(receiver: Receiver<Record>) {
    for record in receiver {
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
    }
}

fn print_error(value: Value) {
    if value.is_error() {
        println!("{}", value)
    }
}

fn read_to_string(path: PathBuf) -> String {
    let mut input = String::new();
    read(path, &mut input).expect("Could not read script file");
    input
}

fn read(path: PathBuf, input: &mut String) -> std::io::Result<()> {
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            if let Ok(entry) = entry {
                let path = entry.path();
                read(path, input)?;
            }
        }
    } else if path.is_file() && path.extension() == Some(std::ffi::OsStr::new(NAME)) {
        input.push_str(&std::fs::read_to_string(path)?)
    } else {
    }
    Ok(())
}
