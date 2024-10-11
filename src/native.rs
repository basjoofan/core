use crate::Value;

pub fn println(objects: Vec<Value>) -> Value {
    if let Some(object) = objects.first() {
        println!("{}", object)
    }
    Value::None
}

pub fn print(objects: Vec<Value>) -> Value {
    if let Some(object) = objects.first() {
        print!("{}", object)
    }
    Value::None
}

pub fn length(objects: Vec<Value>) -> Value {
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
