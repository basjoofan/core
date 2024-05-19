use super::value::Value;

pub fn len(objects: Vec<Value>) -> Value {
    if objects.len() != 1 {
        Value::Error(format!("wrong number of arguments. got={}, want=1", objects.len()))
    } else {
        if let Some(object) = objects.get(0) {
            match object {
                Value::String(string) => Value::Integer(string.len() as i64),
                Value::Array(elements) => Value::Integer(elements.len() as i64),
                Value::Map(pairs) => Value::Integer(pairs.len() as i64),
                _ => Value::Error(format!("function len not supported type {}", object.kind())),
            }
        } else {
            Value::Error(format!("function len need a parameter"))
        }
    }
}
