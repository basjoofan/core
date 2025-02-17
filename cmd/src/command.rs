use lib::eval_block;
use lib::Context;
use lib::Parser;
use lib::Source;
use lib::Stats;
use lib::Writer;
use std::env::current_dir;
use std::env::var;
use std::ffi::OsStr;
use std::io::stdin;
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::read;
use tokio::fs::read_dir;
use tokio::fs::File;
use tokio::signal;
use tokio::task;
use tokio::time;

pub async fn repl() {
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
            context = eval(text, Some(context)).await;
        }
    }
}

pub async fn eval(text: String, context: Option<Context>) -> Context {
    let mut context = context.unwrap_or_default();
    match Parser::new(&text).parse() {
        Ok(Source { exprs, requests, .. }) => {
            context.extend(requests);
            match eval_block(&exprs, &mut context).await {
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

pub async fn test(
    name: Option<String>,
    threads: u32,
    duration: Duration,
    number: u32,
    path: Option<PathBuf>,
    record: Option<PathBuf>,
    stat: bool,
) {
    let text = read_text(path.unwrap_or(current_dir().unwrap())).await;
    let mut context = Context::new();
    let mut tests = match Parser::new(&text).parse() {
        Ok(Source { exprs, requests, tests }) => {
            context.extend(requests);
            match eval_block(&exprs, &mut context).await {
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
    let (sender, receiver) = std::sync::mpsc::channel();
    match name {
        Some(name) => {
            match tests.remove(&name) {
                Some(test) => {
                    let name = Arc::new(name);
                    let test = Arc::new(test);
                    let continuous = Arc::new(AtomicBool::new(true));
                    let maximun = number / threads;
                    for thread in 0..threads {
                        let sender = sender.to_owned();
                        let continuous = continuous.to_owned();
                        let name = name.to_owned();
                        let test = test.to_owned();
                        let mut writer = writer(record.as_ref(), thread).await;
                        let mut context = context.to_owned();
                        handles.push(task::spawn(async move {
                            let mut number = u32::default();
                            while continuous.load(Ordering::Relaxed) && number < maximun {
                                match eval_block(test.as_ref(), &mut context).await {
                                    Ok(_) => {}
                                    Err(error) => {
                                        println!("{}", error);
                                        break;
                                    }
                                }
                                let records = context.records();
                                records.iter().for_each(|record| println!("{}", record));
                                if let Some(ref mut writer) = writer {
                                    writer.write(&records, &name, thread, number).await;
                                }
                                stat.then(|| sender.send(records));
                                number += 1;
                            }
                        }));
                    }
                    // handle interrupt signal
                    task::spawn(register(continuous.clone()));
                    // completed after thread sleep duration
                    task::spawn(async move {
                        time::sleep(duration).await;
                        continuous.store(false, Ordering::Relaxed)
                    });
                }
                None => {
                    println!("Test not found: {}", name)
                }
            }
        }
        None => {
            for (thread, (name, test)) in tests.into_iter().enumerate() {
                let mut writer = writer(record.as_ref(), thread as u32).await;
                let mut context = context.to_owned();
                handles.push(task::spawn(async move {
                    match eval_block(test.as_ref(), &mut context).await {
                        Ok(_) => {}
                        Err(error) => {
                            println!("{}", error);
                        }
                    }
                    let records = context.records();
                    records.iter().for_each(|record| println!("{}", record));
                    if let Some(ref mut writer) = writer {
                        writer.write(&records, &name, thread as u32, u32::default()).await
                    }
                }));
            }
        }
    }
    handles.push(task::spawn_blocking(move || {
        stat.then(|| {
            let mut stats = Stats::default();
            for records in receiver {
                for record in records.iter() {
                    stats.add(&record.name, record.time.total.as_millis());
                }
            }
            print!("{}", stats);
        });
    }));
    std::mem::drop(sender);
    for handle in handles {
        let _ = handle.await;
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

async fn writer(path: Option<&PathBuf>, thread: u32) -> Option<Writer<File>> {
    match path {
        Some(path) => {
            let file = path.join(format!("{}{:06}", var("POD").unwrap_or_default(), thread));
            let display = file.display();
            let file = match File::create(&file).await {
                Err(error) => panic!("Create file {} error: {:?}", display, error),
                Ok(file) => file,
            };
            Some(Writer::new(file).await)
        }
        None => None,
    }
}

async fn register(continuous: Arc<AtomicBool>) {
    match signal::ctrl_c().await {
        Err(error) => panic!("Failed to listen for interrupt: {:?}", error),
        Ok(result) => result,
    };
    continuous.store(false, Ordering::Relaxed);
}
