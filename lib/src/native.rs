use crate::Value;
use std::collections::HashMap;

pub type Function = fn(Vec<Value>) -> Result<Output, String>;

pub struct Output {
    pub value: Value,
    pub sensitive: bool,
}

#[derive(Clone)]
pub struct Registry {
    functions: HashMap<String, Function>,
}

impl Default for Registry {
    fn default() -> Self {
        let mut registry = Self {
            functions: HashMap::new(),
        };
        registry.register("secret", secret);
        registry.register("file", file);
        registry
    }
}

impl Registry {
    pub fn register(&mut self, name: impl Into<String>, function: Function) {
        self.functions.insert(name.into(), function);
    }

    pub fn call(&self, name: &str, arguments: Vec<Value>) -> Option<Result<Output, String>> {
        self.functions.get(name).map(|function| function(arguments))
    }
}

fn secret(arguments: Vec<Value>) -> Result<Output, String> {
    let name = one_string("secret", arguments)?;
    let value = std::env::var(&name).map_err(|_| format!("secret '{name}' is not set"))?;
    Ok(Output {
        value: Value::String(value),
        sensitive: true,
    })
}

fn file(arguments: Vec<Value>) -> Result<Output, String> {
    let path = one_string("file", arguments)?;
    Ok(Output {
        value: Value::String(format!("\0file:{path}")),
        sensitive: false,
    })
}

fn one_string(name: &str, arguments: Vec<Value>) -> Result<String, String> {
    if arguments.len() != 1 {
        return Err(format!("{name} requires one argument"));
    }
    match arguments.into_iter().next() {
        Some(Value::String(value)) => Ok(value),
        Some(value) => Err(format!("{name} requires a string, got {value:?}")),
        None => unreachable!(),
    }
}

pub(crate) fn file_path(value: &Value) -> Option<&str> {
    match value {
        Value::String(value) => value.strip_prefix("\0file:"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn upper(arguments: Vec<Value>) -> Result<Output, String> {
        let value = one_string("upper", arguments)?;
        Ok(Output {
            value: Value::String(value.to_uppercase()),
            sensitive: false,
        })
    }

    #[test]
    fn registry_accepts_future_native_functions() {
        let mut registry = Registry::default();
        registry.register("upper", upper);
        let output = registry
            .call("upper", vec![Value::String("fan".into())])
            .unwrap()
            .unwrap();
        assert_eq!(output.value, Value::String("FAN".into()));
    }
}
