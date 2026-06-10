use lib::Context;
use lib::Parser;
use lib::Source;
use lib::Stats;
use lib::Writer;
use lib::client::Clients;
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
    let mut yaml_buffer = Vec::new();
    loop {
        if let Ok(Some(text)) = lines.next_line().await {
            if text == "exit" {
                flush_yaml_buffer(&mut yaml_buffer, &mut source);
                break;
            }
            if !yaml_buffer.is_empty() {
                if text.trim().is_empty() {
                    flush_yaml_buffer(&mut yaml_buffer, &mut source);
                } else {
                    yaml_buffer.push(text);
                }
                continue;
            }
            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }
            if looks_like_yaml_request_definition_start(trimmed) {
                yaml_buffer.push(text);
                continue;
            }
            context = eval(text, &mut source, Some(context)).await;
        }
    }
}

pub async fn eval(text: String, source: &mut Source, context: Option<Context>) -> Context {
    let mut context = context.unwrap_or_default();
    if looks_like_yaml_request_definition(&text) {
        match load_yaml_request_definition(&text, source) {
            Ok(names) => print_loaded_clients(&names),
            Err(error) => println!("{error}"),
        }
        return context;
    }
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

fn flush_yaml_buffer(buffer: &mut Vec<String>, source: &mut Source) {
    if buffer.is_empty() {
        return;
    }
    let text = buffer.join("\n");
    match load_yaml_request_definition(&text, source) {
        Ok(names) => print_loaded_clients(&names),
        Err(error) => println!("{error}"),
    }
    buffer.clear();
}

fn load_yaml_request_definition(text: &str, source: &mut Source) -> Result<Vec<String>, String> {
    if contains_mixed_yaml_and_fan(text) {
        return Err(
            "YAML request definitions and fan code must be submitted separately".to_string(),
        );
    }

    let clients = Clients::from_str(text).map_err(|error| error.to_string())?;
    let mut names = clients.inner.keys().cloned().collect::<Vec<_>>();
    names.sort();
    source
        .clients
        .try_extend(clients)
        .map_err(|error| error.to_string())?;
    Ok(names)
}

fn print_loaded_clients(names: &[String]) {
    for name in names {
        println!("loaded client: {name}");
    }
}

fn looks_like_yaml_request_definition(text: &str) -> bool {
    let mut has_name = false;
    let mut has_scheme = false;
    let mut has_host = false;
    let mut has_requests = false;

    for line in text.lines() {
        match top_level_yaml_key(line) {
            Some("name") => has_name = true,
            Some("scheme") => has_scheme = true,
            Some("host") => has_host = true,
            Some("requests") => has_requests = true,
            _ => {}
        }
    }

    has_name && has_scheme && has_host && has_requests
}

fn looks_like_yaml_request_definition_start(text: &str) -> bool {
    matches!(
        top_level_yaml_key(text),
        Some("name" | "scheme" | "host" | "requests")
    )
}

fn top_level_yaml_key(line: &str) -> Option<&str> {
    if line.is_empty() || line.starts_with(char::is_whitespace) {
        return None;
    }
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "---" || trimmed == "..." {
        return None;
    }
    let (key, _) = trimmed.split_once(':')?;
    if key.is_empty()
        || !key
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return None;
    }
    Some(key)
}

fn contains_mixed_yaml_and_fan(text: &str) -> bool {
    let top_level_keys = ["name", "scheme", "host", "port", "requests"];
    text.lines().any(|line| {
        let trimmed = line.trim();
        !trimmed.is_empty()
            && !trimmed.starts_with('#')
            && trimmed != "---"
            && trimmed != "..."
            && !trimmed.starts_with('-')
            && !trimmed.contains(':')
            && !line.starts_with(char::is_whitespace)
            && !top_level_keys.contains(&trimmed)
    })
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
    let (text, clients) = match read_sources(path.unwrap_or(current_dir().unwrap())).await {
        Ok(source) => source,
        Err(error) => {
            println!("{error}");
            return;
        }
    };
    let mut context = Context::new();
    let source = Arc::new(match Parser::new(&text).parse() {
        Ok(mut source) => {
            source.clients = clients;
            match source.eval_block(&source.exprs, &mut context).await {
                Ok(_) => source,
                Err(error) => {
                    println!("{error}");
                    return;
                }
            }
        }
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

async fn read_sources(path: PathBuf) -> Result<(String, Clients), String> {
    let mut text = Vec::new();
    let mut clients = Clients::default();
    read_source_bytes(path, &mut text, &mut clients).await?;
    let text = String::from_utf8(text)
        .map_err(|error| format!("Could not decode source file: {error}"))?;
    Ok((text, clients))
}

async fn read_source_bytes(
    path: PathBuf,
    text: &mut Vec<u8>,
    clients: &mut Clients,
) -> Result<(), String> {
    if path.is_dir() {
        let mut entries = read_dir(&path)
            .await
            .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|error| format!("Could not read {}: {error}", path.display()))?
        {
            Box::pin(read_source_bytes(entry.path(), text, clients)).await?;
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
            Some("yaml") | Some("yml") => {
                let input = String::from_utf8(
                    read(&path)
                        .await
                        .map_err(|error| format!("Could not read {}: {error}", path.display()))?,
                )
                .map_err(|error| format!("Could not decode {}: {error}", path.display()))?;
                let parsed = Clients::from_str(&input)
                    .map_err(|error| format!("{}: {error}", path.display()))?;
                clients
                    .try_extend(parsed)
                    .map_err(|error| format!("{}: {error}", path.display()))?;
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
    fn detects_yaml_request_definition_shape() {
        let input = r#"
name: user
scheme: http
host: example.com
requests:
  - get:
      path: /get
      method: GET
"#;

        assert!(looks_like_yaml_request_definition(input));
    }

    #[test]
    fn does_not_detect_fan_inputs_as_yaml() {
        let inputs = [
            r#""name: user""#,
            r#"let requests = "value";"#,
            r#"{"name": "user", "requests": []}"#,
            r#"println("requests: {requests}")"#,
        ];

        for input in inputs {
            assert!(!looks_like_yaml_request_definition(input), "{input}");
        }
    }
}
