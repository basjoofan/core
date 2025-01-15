use crate::eval_block;
use crate::Context;
use crate::Parser;
use crate::Source;
use crate::Stats;
use crate::Writer;
use std::fs::File;
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
            match eval_block(&exprs, &mut context) {
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

pub fn test(
    name: Option<String>,
    threads: u32,
    duration: Duration,
    number: u32,
    path: Option<PathBuf>,
    record: Option<PathBuf>,
    stat: bool,
) {
    let text = read_to_string(path.unwrap_or(std::env::current_dir().unwrap()));
    let mut context = Context::new();
    let mut tests = match Parser::new(&text).parse() {
        Ok(Source { exprs, requests, tests }) => {
            context.extend(requests);
            match eval_block(&exprs, &mut context) {
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
                        let mut writer = writer(record.as_ref(), thread);
                        let mut context = context.to_owned();
                        handles.push(std::thread::spawn(move || {
                            let mut number = u32::default();
                            while continuous.load(Ordering::Relaxed) && number < maximun {
                                match eval_block(test.as_ref(), &mut context) {
                                    Ok(_) => {}
                                    Err(error) => {
                                        println!("{}", error);
                                        break;
                                    }
                                }
                                let records = context.records();
                                records.iter().for_each(|record| println!("{}", record));
                                if let Some(ref mut writer) = writer {
                                    writer.write(&records, &name, thread, number)
                                }
                                stat.then(|| sender.send(records));
                                number += 1;
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
            for (thread, (name, test)) in tests.into_iter().enumerate() {
                let mut writer = writer(record.as_ref(), thread as u32);
                let mut context = context.to_owned();
                handles.push(std::thread::spawn(move || {
                    match eval_block(test.as_ref(), &mut context) {
                        Ok(_) => {}
                        Err(error) => {
                            println!("{}", error);
                        }
                    }
                    let records = context.records();
                    records.iter().for_each(|record| println!("{}", record));
                    if let Some(ref mut writer) = writer {
                        writer.write(&records, &name, thread as u32, u32::default())
                    }
                }));
            }
        }
    }
    handles.push(std::thread::spawn(move || {
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

fn writer(path: Option<&PathBuf>, thread: u32) -> Option<Writer<File>> {
    path.map(|path| {
        let file = path.join(format!("{}{:06}", std::env::var("POD").unwrap_or_default(), thread));
        let display = file.display();
        let file = match File::create(&file) {
            Err(error) => panic!("Create file {} error: {:?}", display, error),
            Ok(file) => file,
        };
        Writer::new(file)
    })
}

fn handle_interrupt(continuous: Arc<AtomicBool>) {
    let _ = ctrlc::set_handler(move || {
        continuous.store(false, Ordering::Relaxed);
    });
}
