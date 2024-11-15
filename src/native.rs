use crate::http;
use crate::Value;

pub const NATIVES: &[(&str, Value)] = &[
    ("print", Value::Native(print)),
    ("println", Value::Native(println)),
    ("length", Value::Native(length)),
    ("format", Value::Native(format)),
    ("http", Value::Native(http)),
];

fn println(objects: Vec<Value>) -> Value {
    match format(objects) {
        error @ Value::Error(_) => error,
        value => {
            println!("{}", value);
            Value::None
        }
    }
}

fn print(objects: Vec<Value>) -> Value {
    match format(objects) {
        error @ Value::Error(_) => error,
        value => {
            print!("{}", value);
            Value::None
        }
    }
}

fn length(objects: Vec<Value>) -> Value {
    if objects.len() != 1 {
        Value::Error(format!("wrong number of arguments. got={}, want=1", objects.len()))
    } else if let Some(object) = objects.first() {
        match object {
            Value::String(string) => Value::Integer(string.len() as i64),
            Value::Array(elements) => Value::Integer(elements.len() as i64),
            Value::Map(pairs) => Value::Integer(pairs.len() as i64),
            _ => Value::Error(format!("function length not supported type {}", object.kind())),
        }
    } else {
        Value::Error("function length need a parameter".to_string())
    }
}

fn format(mut objects: Vec<Value>) -> Value {
    objects.reverse();
    match objects.pop() {
        Some(Value::String(mut string)) => {
            let regex = regex::Regex::new(r"\{\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*\}").unwrap();
            let matches = regex.find_iter(&string);
            let mut ranges = Vec::new();
            matches.for_each(|m| ranges.push(m.range()));
            ranges.reverse();
            let variables = objects.iter();
            if variables.len() != ranges.len() {
                Value::Error(format!(
                    "wrong number of arguments. got={}, want={}",
                    variables.len(),
                    ranges.len()
                ))
            } else {
                for (range, variable) in ranges.into_iter().zip(variables) {
                    string.replace_range(range, &variable.to_string());
                }
                Value::String(string)
            }
        }
        None => Value::Error("function length need a parameter".to_string()),
        _ => return Value::Error("first parameter must be a string".to_string()),
    }
}

fn http(objects: Vec<Value>) -> Value {
    if objects.len() != 1 {
        Value::Error(format!("wrong number of arguments. got={}, want=1", objects.len()))
    } else if let Some(object) = objects.first() {
        match object {
            Value::String(message) => {
                let client = http::Client::default();
                let (_, response, _, _) = client.send(message);
                response.to_value()
            }
            _ => Value::Error(format!("function send not supported type {}", object.kind())),
        }
    } else {
        Value::Error("function send need a parameter".to_string())
    }
}

#[test]
fn test_format() {
    let tests = vec![
        (
            vec![
                Value::String(String::from("Hello, {name}!")),
                Value::String(String::from("World")),
            ],
            Value::String(String::from("Hello, World!")),
        ),
        (
            vec![
                Value::String(String::from(r#"{ "name": "{name}" , age: 2 }"#)),
                Value::String(String::from("Bob")),
            ],
            Value::String(String::from(r#"{ "name": "Bob" , age: 2 }"#)),
        ),
        (
            vec![
                Value::String(String::from(r#"{ "name": "{name}" , age: {age} }"#)),
                Value::String(String::from("Bob")),
                Value::Integer(2),
            ],
            Value::String(String::from(r#"{ "name": "Bob" , age: 2 }"#)),
        ),
    ];
    for (test, expected) in tests {
        let actual = format(test);
        println!("{}=={}", actual, expected);
        assert!(actual == expected);
    }
}
