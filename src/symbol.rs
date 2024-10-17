use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Symbol {
    Global(usize),
    Local(usize),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Symbols {
    outer: Option<Box<Symbols>>,
    inner: HashMap<String, Symbol>,
}

impl Symbols {
    pub fn new() -> Self {
        Self {
            outer: None,
            inner: HashMap::new(),
        }
    }

    pub fn wrap(self) -> Self {
        Self {
            outer: Some(Box::new(self)),
            inner: HashMap::new(),
        }
    }

    pub fn peel(self) -> Self {
        match self.outer {
            Some(outer) => *outer,
            None => self,
        }
    }

    pub fn length(&self) -> usize {
        self.inner.len()
    }

    pub fn define(&mut self, name: &str) -> &Symbol {
        let index = self.inner.len();
        let symbol = match self.outer {
            Some(_) => Symbol::Local(index),
            None => Symbol::Global(index),
        };
        self.inner.insert(name.to_string(), symbol);
        self.inner.get(name).unwrap()
    }

    pub fn resolve(&self, name: &str) -> Option<&Symbol> {
        match self.inner.get(name) {
            Some(symbol) => Some(symbol),
            None => match &self.outer {
                Some(outer) => outer.resolve(name),
                None => None,
            },
        }
    }
}

#[test]
fn test_symbol_scope() {
    let mut global = Symbols::new();
    global.define("a");
    global.define("b");
    global.define("c");
    let mut local = global.wrap();
    local.define("c");
    local.define("d");
    let mut last = local.wrap();
    last.define("e");
    last.define("f");
    let expects = vec![
        ("a", Symbol::Global(0)),
        ("b", Symbol::Global(1)),
        ("c", Symbol::Local(0)),
        ("d", Symbol::Local(1)),
        ("e", Symbol::Local(0)),
        ("f", Symbol::Local(1)),
    ];
    for (name, symbol) in expects {
        assert_eq!(last.resolve(name), Some(&symbol));
    }
    assert_eq!(last.resolve("g"), None);
    // assert_eq!(global.resolve("c"), Some(&Symbol::Global(2)));
    // assert_eq!(local.resolve("c"), Some(&Symbol::Local(0)));
}
