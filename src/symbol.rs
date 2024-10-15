use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq)]
pub struct Symbol {
    scope: Scope,
    pub index: usize,
}

#[derive(Debug, PartialEq, Eq)]
enum Scope {
    Global,
}

pub struct Table {
    inner: HashMap<String, Symbol>,
    index: usize,
}

impl Table {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
            index: usize::MIN,
        }
    }

    pub fn define(&mut self, name: &str) -> usize {
        let index = self.index;
        let symbol = Symbol {
            scope: Scope::Global,
            index: index,
        };
        self.index += 1;
        self.inner.insert(name.to_string(), symbol);
        index
    }

    pub fn resolve(&self, name: &str) -> Option<&Symbol> {
        self.inner.get(name)
    }
}

#[test]
fn test_global_symbol_table() {
    let mut table = Table::new();
    table.define("a");
    table.define("b");
    table.define("c");
    assert_eq!(table.inner.len(), 3);
    let expects = vec![
        (
            "a",
            Symbol {
                scope: Scope::Global,
                index: 0,
            },
        ),
        (
            "b",
            Symbol {
                scope: Scope::Global,
                index: 1,
            },
        ),
        (
            "c",
            Symbol {
                scope: Scope::Global,
                index: 2,
            },
        ),
    ];
    for (name, symbol) in expects {
        assert_eq!(table.resolve(name), Some(&symbol));
    }
}
