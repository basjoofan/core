use crate::http;
use crate::Value;
use std::collections::HashMap;
use std::time::Duration;

pub fn println(objects: Vec<Value>) -> Value {
    match format(objects) {
        error @ Value::Error(_) => error,
        value => {
            println!("{}", value);
            Value::None
        }
    }
}

pub fn print(objects: Vec<Value>) -> Value {
    match format(objects) {
        error @ Value::Error(_) => error,
        value => {
            print!("{}", value);
            Value::None
        }
    }
}

pub fn format(mut objects: Vec<Value>) -> Value {
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
                Value::Error(format!("wrong number of arguments. got={}, want={}", variables.len(), ranges.len()))
            } else {
                for (range, variable) in ranges.into_iter().zip(variables) {
                    string.replace_range(range, &variable.to_string());
                }
                Value::String(string)
            }
        }
        None => Value::Error("function length need a parameter".to_string()),
        _ => Value::Error("first parameter must be a string".to_string()),
    }
}

pub fn length(objects: Vec<Value>) -> Value {
    if objects.len() != 1 {
        Value::Error(format!("wrong number of arguments. got={}, want=1", objects.len()))
    } else if let Some(object) = objects.first() {
        match object {
            Value::String(string) => Value::Integer(string.len() as i64),
            Value::Array(elements) => Value::Integer(elements.len() as i64),
            Value::Map(pairs) => Value::Integer(pairs.len() as i64),
            _ => Value::Error(format!("function length not supported type {:?}", object)),
        }
    } else {
        Value::Error("function length need a parameter".to_string())
    }
}

pub fn append(mut objects: Vec<Value>) -> Value {
    objects.reverse();
    match objects.pop() {
        Some(Value::Array(mut array)) => {
            while let Some(object) = objects.pop() {
                array.push(object);
            }
            Value::Array(array)
        }
        None => Value::Error("function length need a parameter".to_string()),
        _ => Value::Error("first parameter must be a array".to_string()),
    }
}

pub fn http(objects: Vec<Value>) -> Value {
    if objects.len() != 1 {
        Value::Error(format!("wrong number of arguments. got={}, want=1", objects.len()))
    } else if let Some(object) = objects.first() {
        match object {
            Value::String(message) => {
                let client = http::Client::default();
                let (request, response, time, error) = client.send(message);
                let mut record = HashMap::new();
                record.insert(String::from("request"), request.into_value());
                record.insert(String::from("response"), response.into_value());
                record.insert(String::from("time"), time.into_value());
                record.insert(String::from("error"), Value::String(error));
                Value::Map(record)
            }
            _ => Value::Error(format!("function send not supported type {:?}", object)),
        }
    } else {
        Value::Error("function send need a parameter".to_string())
    }
}

pub fn track(objects: Vec<Value>) -> Value {
    if objects.len() != 1 {
        Value::Error(format!("wrong number of arguments. got={}, want=1", objects.len()))
    } else if let Some(object) = objects.first() {
        match object {
            Value::Map(record) => {
                let name = record.get("name").unwrap_or(&Value::None);
                println!("=== TEST  {}", name);
                let mut flag = true;
                if let Some(Value::Array(asserts)) = record.get("asserts") {
                    for assert in asserts {
                        if let Value::Map(assert) = assert {
                            let result = !matches!(assert.get("result"), Some(Value::Boolean(false)));
                            println!(
                                "{} => {} {} {} => {}",
                                assert.get("expr").unwrap_or(&Value::None),
                                assert.get("left").unwrap_or(&Value::None),
                                assert.get("compare").unwrap_or(&Value::None),
                                assert.get("right").unwrap_or(&Value::None),
                                result
                            );
                            flag &= result;
                        }
                    }
                }
                println!(
                    "--- {}  {} ({:?})",
                    match flag {
                        true => "PASS",
                        false => "FAIL",
                    },
                    name,
                    match record.get("time") {
                        Some(Value::Map(time)) => {
                            match time.get("total") {
                                Some(Value::Integer(total)) => Duration::from_nanos(*total as u64),
                                _ => Duration::ZERO,
                            }
                        }
                        _ => Duration::ZERO,
                    }
                );
                Value::None
            }
            _ => Value::Error(format!("function send not supported type {:?}", object)),
        }
    } else {
        Value::Error("function send need a parameter".to_string())
    }
}

#[test]
fn test_format() {
    let tests = vec![
        (
            vec![Value::String(String::from("Hello, {name}!")), Value::String(String::from("World"))],
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
        assert_eq!(actual, expected);
    }
}

#[test]
fn test_append() {
    let tests = vec![
        (
            vec![Value::Array(vec![Value::Integer(1)]), Value::Integer(2)],
            Value::Array(vec![Value::Integer(1), Value::Integer(2)]),
        ),
        (
            vec![Value::Array(vec![Value::Integer(1)]), Value::String(String::from("string"))],
            Value::Array(vec![Value::Integer(1), Value::String(String::from("string"))]),
        ),
        (
            vec![Value::Array(vec![Value::Integer(1)]), Value::Boolean(true)],
            Value::Array(vec![Value::Integer(1), Value::Boolean(true)]),
        ),
    ];
    for (test, expected) in tests {
        let actual = append(test);
        println!("{}=={}", actual, expected);
        assert_eq!(actual, expected);
    }
}
