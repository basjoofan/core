use crate::Context;
use crate::Value;

pub fn println(values: Vec<Value>, context: &Context) -> Result<Value, String> {
    match format(values, context) {
        error @ Err(_) => error,
        Ok(value) => {
            println!("{value}");
            Ok(Value::Null)
        }
    }
}

pub fn print(values: Vec<Value>, context: &Context) -> Result<Value, String> {
    match format(values, context) {
        error @ Err(_) => error,
        Ok(value) => {
            print!("{value}");
            Ok(Value::Null)
        }
    }
}

pub fn format(values: Vec<Value>, context: &Context) -> Result<Value, String> {
    match values.first() {
        Some(Value::String(string)) => Ok(Value::String(format_template(string, context))),
        None => Err("function length need a parameter".to_string()),
        _ => Err("first parameter must be a string".to_string()),
    }
}

pub fn format_template(str: &str, context: &Context) -> String {
    let regex = regex::Regex::new(r"\{\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*\}").unwrap();
    let matches = regex.find_iter(str);
    let mut ranges = Vec::new();
    matches.for_each(|m| ranges.push((m.as_str()[1..m.as_str().len() - 1].trim(), m.range())));
    ranges.reverse();
    let mut string = str.to_string();
    for (variable, range) in ranges.into_iter() {
        if let Some(variable) = context.get(variable) {
            string.replace_range(range, variable.to_string().as_str())
        }
    }
    string
}

pub fn length(values: Vec<Value>) -> Result<Value, String> {
    if values.len() != 1 {
        Err(format!("wrong number of arguments. got={}, want=1", values.len()))
    } else if let Some(value) = values.first() {
        match value {
            Value::String(string) => Ok(Value::Integer(string.len() as i64)),
            Value::Array(items) => Ok(Value::Integer(items.len() as i64)),
            Value::Map(pairs) => Ok(Value::Integer(pairs.len() as i64)),
            _ => Err(format!("function length not supported type {value:?}")),
        }
    } else {
        Err("function length need a parameter".to_string())
    }
}

pub fn append(mut values: Vec<Value>) -> Result<Value, String> {
    values.reverse();
    match values.pop() {
        Some(Value::Array(mut array)) => {
            while let Some(value) = values.pop() {
                array.push(value);
            }
            Ok(Value::Array(array))
        }
        None => Err("function length need a parameter".to_string()),
        _ => Err("first parameter must be a array".to_string()),
    }
}

#[test]
fn test_format() {
    let tests = vec![
        (
            vec![Value::String(String::from("Hello, {name}!"))],
            vec![(String::from("name"), Value::String(String::from("World")))],
            Value::String(String::from("Hello, World!")),
        ),
        (
            vec![Value::String(String::from(r#"{ "name": "{name}" , age: 2 }"#))],
            vec![(String::from("name"), Value::String(String::from("Bob")))],
            Value::String(String::from(r#"{ "name": "Bob" , age: 2 }"#)),
        ),
        (
            vec![Value::String(String::from(r#"{ "name": "{name}" , age: {age} }"#))],
            vec![
                (String::from("name"), Value::String(String::from("Bob"))),
                (String::from("age"), Value::Integer(2)),
            ],
            Value::String(String::from(r#"{ "name": "Bob" , age: 2 }"#)),
        ),
        (
            vec![Value::String(String::from(r#"{ "name": "{name}" , age: {age} }"#))],
            vec![(String::from("name"), Value::String(String::from("Bob")))],
            Value::String(String::from(r#"{ "name": "Bob" , age: {age} }"#)),
        ),
    ];
    for (test, variables, expected) in tests {
        let context = Context::from(std::collections::HashMap::from_iter(variables));
        match format(test, &context) {
            Ok(actual) => {
                println!("{actual}=={expected}");
                assert_eq!(actual, expected);
            }
            Err(error) => {
                unreachable!("{}", error);
            }
        }
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
        match append(test) {
            Ok(actual) => {
                println!("{actual}=={expected}");
                assert_eq!(actual, expected);
            }
            Err(error) => {
                unreachable!("{}", error);
            }
        }
    }
}
