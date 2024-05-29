use super::evaluator::eval;
use super::evaluator::eval_expression;
use super::parser::Parser;
use super::syntax::Expr;
use super::token::Kind;
use super::token::Token;
use super::value::Context;
use super::value::Value;
use std::io::stdin;
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::mpsc;

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
                let evaluated = eval(&script, &mut context);
                println!("{}", evaluated)
            }
        }
    }
}

pub fn run(path: Option<PathBuf>) {
    let input = read_to_string(path.unwrap_or(std::env::current_dir().unwrap()));
    let mut context = Context::default();
    let script = Parser::new(&input).parse();
    let evaluated = eval(&script, &mut context);
    println!("{}", evaluated)
}

pub fn call(name: String) {
    let (sender, receiver) = mpsc::channel();
    let input = read_to_string(std::env::current_dir().unwrap());
    let mut context = Context::default();
    context.set_sender(sender);
    let script = Parser::new(&input).parse();
    eval(&script, &mut context);
    std::thread::spawn(move || {
        let value = eval_callable(&name, &mut context);
        if value.is_error() {
            println!("{}", value)
        }
    });
    for record in receiver {
        println!("=== TEST  {}", record.name);
        let mut result = true;
        record.asserts.iter().for_each(|assert| {
            result &= assert.result;
            println!("{}", assert);
        });
        if result {
            println!(
                "--- PASS  {} ({:?})",
                record.name,
                record.duration
            );
        } else {
            println!(
                "--- FAIL  {} ({:?})",
                record.name,
                record.duration
            );
        }
    }
}

pub fn test(tag: String) {
    let input = read_to_string(std::env::current_dir().unwrap());
    let mut context = Context::default();
    let script = Parser::new(&input).parse();
    let mut evaluated = eval(&script, &mut context);
    for callable in script.requests.iter().chain(script.functions.iter()) {
        if let (Some(tags), Some(name)) = match callable {
            Expr::Function(_, Some(tags), Some(name), _, _) => (Some(tags), Some(name)),
            Expr::Request(_, Some(tags), name, _, _) => (Some(tags), Some(name)),
            _ => (None, None),
        } {
            if tags.contains(&tag) {
                evaluated = eval_callable(name, &mut context);
            }
        }
    }
    println!("{}", evaluated)
}

fn eval_callable(name: &String, context: &mut Context) -> Value {
    let token = Token {
        kind: Kind::Ident,
        literal: name.clone(),
    };
    let callable = Expr::Call(
        token.clone(),
        Some(Box::new(Expr::Ident(token, name.clone()))),
        Vec::new(),
    );
    eval_expression(&callable, context)
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
