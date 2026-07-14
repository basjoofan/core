use lib::{Mech, Parser, Request, Source, Trans, Value};
use std::collections::HashMap;
use std::env::current_dir;
use std::ffi::OsStr;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use tokio::fs::{read, read_dir};
use tokio::io::{AsyncBufReadExt, BufReader, stdin};

pub async fn repl() {
    let mut lines = BufReader::new(stdin()).lines();
    let mut source = Source::new();
    let mut values = HashMap::new();
    let mut buffer = Vec::new();
    while let Ok(Some(line)) = lines.next_line().await {
        if line == "exit" {
            break;
        }
        buffer.push(line);
        let text = buffer.join("\n");
        match source_state(&text) {
            State::Complete | State::Invalid => {
                values = eval(text, &mut source, values).await;
                buffer.clear();
            }
            State::Incomplete => {}
        }
    }
}

pub async fn eval(
    text: String,
    source: &mut Source,
    values: HashMap<String, Value>,
) -> HashMap<String, Value> {
    match Parser::new(&text).parse() {
        Ok(parsed) => {
            let index = source.extend(parsed);
            let expressions = source.exprs[index..].to_vec();
            let mut mech = Mech::interactive(source).with_values(values);
            let mut trans = HttpTrans;
            match mech.run(&expressions, &mut trans).await {
                Ok(value) => println!("{value}"),
                Err(error) => println!("{error}"),
            }
            mech.into_values()
        }
        Err(error) => {
            println!("{error}");
            values
        }
    }
}

pub async fn test(target: Option<String>, env: Option<String>, path: Option<PathBuf>) -> bool {
    let (name, tag) = match target {
        Some(target) => match target.strip_prefix('@') {
            Some("") => {
                println!("Tag name is required after '@'");
                return false;
            }
            Some(tag) => (None, Some(tag.to_owned())),
            None => (Some(target), None),
        },
        None => (None, None),
    };
    let path = path.unwrap_or_else(|| current_dir().unwrap());
    let text = match read_sources(path).await {
        Ok(text) => text,
        Err(error) => {
            println!("{error}");
            return false;
        }
    };
    let source = match Parser::new(&text).parse() {
        Ok(source) => source,
        Err(error) => {
            println!("{error}");
            return false;
        }
    };
    let names: Vec<_> = match name {
        Some(name) => {
            if source.test(&name).is_none() {
                println!("Test not found: {name}");
                return false;
            }
            vec![name]
        }
        None => source
            .tests
            .keys()
            .filter(|name| {
                tag.as_ref().is_none_or(|tag| {
                    source
                        .test(name)
                        .is_some_and(|test| test.tags.contains(tag))
                })
            })
            .cloned()
            .collect(),
    };
    let mut trans = HttpTrans;
    let mut passed = true;
    for name in names {
        let result = match Mech::new(&source, env.as_deref()) {
            Ok(mut mech) => mech.run_test(&name, &mut trans).await,
            Err(error) => Err(error),
        };
        match result {
            Ok(outcome) => println!("PASS  {} ({} expects)", outcome.name, outcome.excepts),
            Err(error) => {
                println!("FAIL  {name}: {error}");
                passed = false;
            }
        }
    }
    passed
}

struct HttpTrans;
impl Trans for HttpTrans {
    fn send<'a>(
        &'a mut self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = lib::Result> + Send + 'a>> {
        Box::pin(http::send(request))
    }
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
            '"' | '`' => quote = Some(character),
            '{' | '[' | '(' => stack.push(character),
            '}' if stack.pop() != Some('{') => return State::Invalid,
            ']' if stack.pop() != Some('[') => return State::Invalid,
            ')' if stack.pop() != Some('(') => return State::Invalid,
            _ => {}
        }
    }
    if quote.is_none() && stack.is_empty() {
        State::Complete
    } else {
        State::Incomplete
    }
}

async fn read_sources(path: PathBuf) -> Result<String, String> {
    let mut bytes = Vec::new();
    read_source_bytes(path, &mut bytes).await?;
    String::from_utf8(bytes).map_err(|error| error.to_string())
}

async fn read_source_bytes(path: PathBuf, bytes: &mut Vec<u8>) -> Result<(), String> {
    if path.is_dir() {
        let mut entries = read_dir(&path).await.map_err(|error| error.to_string())?;
        let mut paths = Vec::new();
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|error| error.to_string())?
        {
            paths.push(entry.path());
        }
        paths.sort();
        for path in paths {
            Box::pin(read_source_bytes(path, bytes)).await?;
        }
    } else if path.extension().and_then(OsStr::to_str) == Some("fan") {
        bytes.extend(read(&path).await.map_err(|error| error.to_string())?);
        bytes.push(b'\n');
    }
    Ok(())
}
