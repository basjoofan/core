use crate::code::{self, Opcode};
use crate::value::Value;

pub struct Vm {
    constants: Vec<Value>,
    instructions: Vec<u8>,
    stack: Vec<Value>,
    // sp: usize,
}

impl Vm {
    pub fn new(constants: Vec<Value>, instructions: Vec<u8>) -> Self {
        Self {
            constants,
            instructions,
            stack: Vec::new(),
            // sp: 0,
        }
    }

    pub fn run(&mut self) {
        let mut i = 0;
        while i < self.instructions.len() {
            let opcode = Opcode::from(self.instructions[i]);
            i += 1;
            match opcode {
                Opcode::Constant => {
                    let index = code::read(&self.instructions[i..i + opcode.width()]);
                    i += opcode.width();
                    if let Some(value) = self.constants.get(index as usize) {
                        self.push(value.clone());
                    }
                }
                Opcode::Add => {
                    let right = self.pop();
                    let left = self.pop();
                    match (left, right) {
                        (Some(Value::Integer(left)), Some(Value::Integer(right))) => {
                            self.stack.push(Value::Integer(left + right))
                        }
                        (_, _) => panic!("Stack underflow!"),
                    }
                }
            }
        }
    }

    pub fn top(&self) -> Option<&Value> {
        self.stack.last()
    }

    fn pop(&mut self) -> Option<Value> {
        self.stack.pop()
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value)
    }
}

#[cfg(test)]
mod tests {
    use crate::compiler::Compiler;
    use crate::value::Value;
    use crate::vm::Vm;

    fn run_vm_tests(tests: Vec<(&str, Value)>) {
        for (text, value) in tests {
            let source = crate::parser::Parser::new(text).parse();
            let mut compiler = Compiler::new();
            for expression in source.block.iter() {
                let result = compiler.compile(&expression);
                assert!(result.is_ok(), "{}", result.unwrap_err())
            }
            let mut vm = Vm::new(compiler.constants, compiler.instructions);
            vm.run();
            assert!(vm.top().unwrap().clone() == value);
        }
    }

    #[test]
    fn test_integer_arithmetic() {
        let tests = vec![
            ("1", Value::Integer(1)),
            ("2", Value::Integer(2)),
            ("1 + 2", Value::Integer(3)),
        ];
        run_vm_tests(tests);
    }
}
