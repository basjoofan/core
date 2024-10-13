use crate::Opcode;
use crate::Value;

pub struct Vm {
    constants: Vec<Value>,
    instructions: Vec<Opcode>,
    stack: Vec<Value>,
    sp: usize,
}

impl Vm {
    pub fn new(constants: Vec<Value>, instructions: Vec<Opcode>) -> Self {
        Self {
            constants,
            instructions,
            stack: Vec::new(),
            sp: usize::MIN,
        }
    }

    pub fn run(&mut self) {
        for i in 0..self.instructions.len() {
            let opcode = self.instructions[i];
            match opcode {
                Opcode::Constant(index) => {
                    let value = self.constants[index].clone();
                    self.push(value);
                }
                Opcode::Pop => {
                    self.pop();
                }
                Opcode::Add => {
                    let right = self.pop();
                    let left = self.pop();
                    match (left, right) {
                        (Value::Integer(left), Value::Integer(right)) => self.push(Value::Integer(left + right)),
                        (_, _) => panic!("Stack underflow!"),
                    }
                }
            }
        }
    }

    pub fn push(&mut self, value: Value) {
        self.stack.insert(self.sp, value);
        self.sp += 1;
    }

    pub fn pop(&mut self) -> Value {
        self.sp -= 1;
        self.stack[self.sp].clone()
    }

    pub fn past(&self) -> &Value {
        &self.stack[self.sp]
    }
}

#[cfg(test)]
mod tests {
    use crate::Compiler;
    use crate::Value;
    use crate::Vm;

    fn run_vm_tests(tests: Vec<(&str, Value)>) {
        for (text, value) in tests {
            let source = crate::parser::Parser::new(text).parse().unwrap();
            let mut compiler = Compiler::new();
            let result = compiler.compile(&source);
            assert!(result.is_ok(), "compile error: {}", result.unwrap_err());
            let mut vm = Vm::new(compiler.constants, compiler.instructions);
            vm.run();
            println!("{} = {}", vm.past(), value);
            assert_eq!(vm.past(), &value);
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
