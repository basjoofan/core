use crate::writer::Writer;
use lib::Context;
use lib::ExecutionStep;
use lib::Expr;
use lib::Parser;
use lib::Source;
use lib::Stats;
use lib::Value;
use std::env::current_dir;
use std::env::var;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::fs::File;
use tokio::fs::read;
use tokio::fs::read_dir;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::io::stdin;
use tokio::signal;
use tokio::sync;
use tokio::task;
use tokio::time;

pub async fn repl() {
    let stdin = BufReader::new(stdin());
    let mut lines = stdin.lines();
    let mut context = Context::new();
    let mut source = Source::new();
    let mut buffer = Vec::new();
    loop {
        if let Ok(Some(text)) = lines.next_line().await {
            if text == "exit" {
                if !buffer.is_empty() {
                    let _ = eval(buffer.join("\n"), &mut source, Some(context)).await;
                }
                break;
            }
            let trimmed = text.trim();
            if trimmed.is_empty() && buffer.is_empty() {
                continue;
            }
            buffer.push(text);
            let input = buffer.join("\n");
            match source_state(&input) {
                State::Incomplete => {}
                State::Complete | State::Invalid => {
                    context = eval(input, &mut source, Some(context)).await;
                    buffer.clear();
                }
            }
        }
    }
}

pub async fn eval(text: String, source: &mut Source, context: Option<Context>) -> Context {
    let mut context = context.unwrap_or_default();
    match Parser::new(&text).parse() {
        Ok(s) => {
            if let Some(name) = s
                .clients
                .inner
                .keys()
                .find(|name| source.clients.inner.contains_key(*name))
            {
                println!("duplicate client '{name}'");
                return context;
            }
            let index = source.extend(s);
            let exprs = &source.exprs[index..];
            match execute(source, exprs, context.clone()).await {
                Ok((value, next)) => {
                    context = next;
                    println!("{value}");
                }
                Err(error) => println!("{error}"),
            }
        }
        Err(error) => println!("{error}"),
    }
    context
}

#[derive(Debug, PartialEq, Eq)]
enum State {
    Complete,
    Incomplete,
    Invalid,
}

fn source_state(text: &str) -> State {
    let mut stack = Vec::new();
    let mut quote = None;
    let mut escaped = false;
    for character in text.chars() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == active_quote {
                quote = None;
            }
            continue;
        }
        match character {
            '"' => quote = Some(character),
            '{' | '[' | '(' => stack.push(character),
            '}' => match stack.pop() {
                Some('{') => {}
                _ => return State::Invalid,
            },
            ']' => match stack.pop() {
                Some('[') => {}
                _ => return State::Invalid,
            },
            ')' => match stack.pop() {
                Some('(') => {}
                _ => return State::Invalid,
            },
            _ => {}
        }
    }
    if quote.is_none() && stack.is_empty() {
        State::Complete
    } else {
        State::Incomplete
    }
}

pub async fn test(
    name: Option<String>,
    task: u32,
    duration: Duration,
    number: u32,
    path: Option<PathBuf>,
    record: Option<PathBuf>,
    stat: bool,
) {
    let text = match read_sources(path.unwrap_or(current_dir().unwrap())).await {
        Ok(source) => source,
        Err(error) => {
            println!("{error}");
            return;
        }
    };
    let mut context = Context::new();
    let source = Arc::new(match Parser::new(&text).parse() {
        Ok(source) => match source.eval_block(&source.exprs, &mut context) {
            Ok(_) => source,
            Err(error) => {
                println!("{error}");
                return;
            }
        },
        Err(error) => {
            println!("{error}");
            return;
        }
    });
    let mut set = task::JoinSet::new();
    let (sender, mut receiver) = sync::mpsc::channel(task as usize);
    match name {
        Some(name) => {
            match source.test(&name) {
                Some(test) => {
                    let continuous = Arc::new(AtomicBool::new(true));
                    let maximun = number / task;
                    for task in 0..task {
                        let continuous = continuous.to_owned();
                        let sender = sender.to_owned();
                        let mut writer = writer(record.as_ref(), task).await;
                        let name = name.to_owned();
                        let test = test.to_owned();
                        let source = source.to_owned();
                        let mut context = context.to_owned();
                        set.spawn(async move {
                            let mut number = u32::default();
                            while continuous.load(Ordering::Relaxed) && number < maximun {
                                match execute(&source, &test, context.clone()).await {
                                    Ok((_, next)) => context = next,
                                    Err(error) => {
                                        println!("{error}");
                                        break;
                                    }
                                }
                                let records = context.records();
                                records.iter().for_each(|record| println!("{record}"));
                                if let Some(ref mut writer) = writer {
                                    writer.write(&records, &name, task, number).await;
                                }
                                if stat {
                                    let _ = sender.send(records).await;
                                }
                                number += 1;
                            }
                        });
                    }
                    // handle interrupt signal
                    task::spawn(register(continuous.to_owned()));
                    // completed after task sleep duration
                    task::spawn(async move {
                        time::sleep(duration).await;
                        continuous.store(false, Ordering::Relaxed)
                    });
                }
                None => {
                    println!("Test not found: {name}")
                }
            }
        }
        None => {
            for (task, (name, test)) in source.tests.iter().enumerate() {
                let mut writer = writer(record.as_ref(), task as u32).await;
                let name = name.to_owned();
                let test = test.to_owned();
                let source = source.to_owned();
                let mut context = context.to_owned();
                set.spawn(async move {
                    match execute(&source, test.as_ref(), context.clone()).await {
                        Ok((_, next)) => context = next,
                        Err(error) => {
                            println!("{error}");
                        }
                    }
                    let records = context.records();
                    records.iter().for_each(|record| println!("{record}"));
                    if let Some(ref mut writer) = writer {
                        writer
                            .write(&records, &name, task as u32, u32::default())
                            .await
                    }
                });
            }
        }
    }
    stat.then(|| {
        set.spawn(async move {
            let mut stats = Stats::default();
            while let Some(records) = receiver.recv().await {
                for record in records.iter() {
                    stats.add(&record.name, record.time.total.as_millis());
                }
            }
            print!("{stats}");
        });
    });
    std::mem::drop(sender);
    while let Some(result) = set.join_next().await {
        if let Err(error) = result {
            println!("Task error: {error}");
        }
    }
}

async fn execute(
    source: &Source,
    exprs: &[Expr],
    context: Context,
) -> Result<(Value, Context), String> {
    let mut execution = source.start(exprs, context);
    let mut step = execution.run();
    loop {
        match step {
            ExecutionStep::Pending(pending) => {
                let id = pending.id;
                let result = http::send(pending.request).await;
                step = execution.resume(id, result);
            }
            ExecutionStep::Complete(value) => {
                return Ok((value, execution.into_context()));
            }
            ExecutionStep::Error(error) => return Err(error),
        }
    }
}

async fn read_sources(path: PathBuf) -> Result<String, String> {
    let mut text = Vec::new();
    read_source_bytes(path, &mut text).await?;
    let text = String::from_utf8(text)
        .map_err(|error| format!("Could not decode source file: {error}"))?;
    Ok(text)
}

async fn read_source_bytes(path: PathBuf, text: &mut Vec<u8>) -> Result<(), String> {
    if path.is_dir() {
        let mut entries = read_dir(&path)
            .await
            .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|error| format!("Could not read {}: {error}", path.display()))?
        {
            Box::pin(read_source_bytes(entry.path(), text)).await?;
        }
    } else if path.is_file() {
        match path.extension().and_then(OsStr::to_str) {
            Some("fan") => {
                text.append(
                    &mut read(&path)
                        .await
                        .map_err(|error| format!("Could not read {}: {error}", path.display()))?,
                );
                text.push(b'\n');
            }
            _ => {}
        }
    }
    Ok(())
}

async fn writer(path: Option<&PathBuf>, task: u32) -> Option<Writer<File>> {
    match path {
        Some(path) => {
            let file = path.join(format!("{}{:06}", var("POD").unwrap_or_default(), task));
            let display = file.display();
            let file = match File::create(&file).await {
                Err(error) => panic!("Create file {display} error: {error:?}"),
                Ok(file) => file,
            };
            Some(Writer::new(file))
        }
        None => None,
    }
}

async fn register(continuous: Arc<AtomicBool>) {
    match signal::ctrl_c().await {
        Err(error) => panic!("Failed to listen for interrupt: {error:?}"),
        Ok(result) => result,
    };
    continuous.store(false, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_complete_multiline_source() {
        assert_eq!(source_state("client user {"), State::Incomplete);
        assert_eq!(
            source_state("client user { scheme: https, host: \"example.com\", requests: {} }"),
            State::Complete
        );
    }

    #[test]
    fn ignores_delimiters_inside_strings() {
        assert_eq!(source_state(r#"println("{")"#), State::Complete);
    }

    #[test]
    fn detects_mismatched_delimiters() {
        assert_eq!(source_state("let x = (1]"), State::Invalid);
    }
}
