use lib::Context;
use lib::Parser;
use lib::Source;
use lib::Stats;
use lib::Writer;
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
    loop {
        if let Ok(Some(text)) = lines.next_line().await {
            if text == "exit" {
                break;
            }
            if text.trim().is_empty() {
                continue;
            }
            context = eval(text, &mut source, Some(context)).await;
        }
    }
}

pub async fn eval(text: String, source: &mut Source, context: Option<Context>) -> Context {
    let mut context = context.unwrap_or_default();
    match Parser::new(&text).parse() {
        Ok(s) => {
            let index = source.extend(s);
            let exprs = &source.exprs[index..];
            match source.eval_block(exprs, &mut context).await {
                Ok(value) => {
                    println!("{value}");
                }
                Err(error) => println!("{error}"),
            }
        }
        Err(error) => println!("{error}"),
    }
    context
}

pub async fn test(
    name: Option<String>,
    tasks: u32,
    duration: Duration,
    number: u32,
    path: Option<PathBuf>,
    record: Option<PathBuf>,
    stat: bool,
) {
    let text = read_text(path.unwrap_or(current_dir().unwrap())).await;
    let mut context = Context::new();
    let source = Arc::new(match Parser::new(&text).parse() {
        Ok(source) => match source.eval_block(&source.exprs, &mut context).await {
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
    let (sender, mut receiver) = sync::mpsc::channel(tasks as usize);
    match name {
        Some(name) => {
            match source.test(&name) {
                Some(test) => {
                    let continuous = Arc::new(AtomicBool::new(true));
                    let maximun = number / tasks;
                    for task in 0..tasks {
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
                                match source.eval_block(&test, &mut context).await {
                                    Ok(_) => {}
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
                    match &source.eval_block(test.as_ref(), &mut context).await {
                        Ok(_) => {}
                        Err(error) => {
                            println!("{error}");
                        }
                    }
                    let records = context.records();
                    records.iter().for_each(|record| println!("{record}"));
                    if let Some(ref mut writer) = writer {
                        writer.write(&records, &name, task as u32, u32::default()).await
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

async fn read_text(path: PathBuf) -> String {
    let mut text = Vec::new();
    read_bytes(path, &mut text).await.expect("Could not read source file");
    String::from_utf8(text).expect("Could not decode source file")
}

async fn read_bytes(path: PathBuf, text: &mut Vec<u8>) -> std::io::Result<()> {
    if path.is_dir() {
        let mut entries = read_dir(path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            Box::pin(read_bytes(path, text)).await?;
        }
    } else if path.is_file() && path.extension() == Some(OsStr::new("fan")) {
        text.append(&mut read(path).await?);
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
