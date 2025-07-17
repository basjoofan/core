use crate::Value;

pub fn println(values: Vec<Value>) -> Result<Value, String> {
    match format(values) {
        error @ Err(_) => error,
        Ok(value) => {
            println!("{value}");
            Ok(Value::Null)
        }
    }
}

pub fn print(values: Vec<Value>) -> Result<Value, String> {
    match format(values) {
        error @ Err(_) => error,
        Ok(value) => {
            print!("{value}");
            Ok(Value::Null)
        }
    }
}

pub fn format(mut values: Vec<Value>) -> Result<Value, String> {
    values.reverse();
    match values.pop() {
        Some(Value::String(mut string)) => {
            let regex = regex::Regex::new(r"\{\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*\}").unwrap();
            let matches = regex.find_iter(&string);
            let mut ranges = Vec::new();
            matches.for_each(|m| ranges.push(m.range()));
            ranges.reverse();
            let variables = values.iter();
            if variables.len() != ranges.len() {
                Err(format!("wrong number of arguments. got={}, want={}", variables.len(), ranges.len()))
            } else {
                for (range, variable) in ranges.into_iter().zip(variables) {
                    string.replace_range(range, &variable.to_string());
                }
                Ok(Value::String(string))
            }
        }
        None => Err("function length need a parameter".to_string()),
        _ => Err("first parameter must be a string".to_string()),
    }
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
        match format(test) {
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
