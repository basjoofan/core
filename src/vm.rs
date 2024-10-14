use crate::Opcode;
use crate::Value;

pub struct Vm {
    consts: Vec<Value>,
    instructions: Vec<Opcode>,
    stack: Vec<Value>,
    sp: usize,
}

impl Vm {
    pub fn new(consts: Vec<Value>, instructions: Vec<Opcode>) -> Self {
        Self {
            consts,
            instructions,
            stack: Vec::new(),
            sp: usize::MIN,
        }
    }

    pub fn run(&mut self) {
        let mut ip = usize::MIN;
        while ip < self.instructions.len() {
            let opcode = self.instructions[ip];
            ip += 1;
            match opcode {
                Opcode::None => self.push(Value::None),
                Opcode::Const(index) => {
                    let value = self.consts[index].clone();
                    self.push(value);
                }
                Opcode::Pop => {
                    self.pop();
                }
                Opcode::Add
                | Opcode::Sub
                | Opcode::Mul
                | Opcode::Div
                | Opcode::Lt
                | Opcode::Gt
                | Opcode::Eq
                | Opcode::Ne => {
                    let right = self.pop();
                    let left = self.pop();
                    match (left, right, opcode) {
                        (Value::Integer(left), Value::Integer(right), Opcode::Add) => {
                            self.push(Value::Integer(left + right))
                        }
                        (Value::Integer(left), Value::Integer(right), Opcode::Sub) => {
                            self.push(Value::Integer(left - right))
                        }
                        (Value::Integer(left), Value::Integer(right), Opcode::Mul) => {
                            self.push(Value::Integer(left * right))
                        }
                        (Value::Integer(left), Value::Integer(right), Opcode::Div) => {
                            self.push(Value::Integer(left / right))
                        }
                        (Value::Integer(left), Value::Integer(right), Opcode::Lt) => {
                            self.push(Value::Boolean(left < right))
                        }
                        (Value::Integer(left), Value::Integer(right), Opcode::Gt) => {
                            self.push(Value::Boolean(left > right))
                        }
                        (Value::Integer(left), Value::Integer(right), Opcode::Eq) => {
                            self.push(Value::Boolean(left == right))
                        }
                        (Value::Integer(left), Value::Integer(right), Opcode::Ne) => {
                            self.push(Value::Boolean(left != right))
                        }
                        (Value::Boolean(left), Value::Boolean(right), Opcode::Eq) => {
                            self.push(Value::Boolean(left == right))
                        }
                        (Value::Boolean(left), Value::Boolean(right), Opcode::Ne) => {
                            self.push(Value::Boolean(left != right))
                        }
                        (left, right, opcode) => panic!(
                            "unsupported types for binary operation: {} {:?} {}",
                            left, opcode, right
                        ),
                    }
                }
                Opcode::True => self.push(Value::Boolean(true)),
                Opcode::False => self.push(Value::Boolean(false)),
                Opcode::Minus => {
                    let operand = self.pop();
                    match operand {
                        Value::Integer(value) => self.push(Value::Integer(-value)),
                        _ => panic!("unsupported types for negation: {}", operand),
                    }
                }
                Opcode::Bang => {
                    let operand = self.pop();
                    match operand {
                        Value::Boolean(false) | Value::None => self.push(Value::Boolean(true)),
                        _ => self.push(Value::Boolean(false)),
                    }
                }
                Opcode::Jump(i) => ip = i,
                Opcode::Judge(i) => {
                    let condition = self.pop();
                    match condition {
                        Value::Boolean(false) | Value::None => ip = i,
                        _ => {}
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
            println!("{:?}", compiler.instructions);
            let mut vm = Vm::new(compiler.consts, compiler.instructions);
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
            ("1 - 2", Value::Integer(-1)),
            ("1 * 2", Value::Integer(2)),
            ("4 / 2", Value::Integer(2)),
            ("50 / 2 * 2 + 10 - 5", Value::Integer(55)),
            ("5 * (2 + 10)", Value::Integer(60)),
            ("5 + 5 + 5 + 5 - 10", Value::Integer(10)),
            ("2 * 2 * 2 * 2 * 2", Value::Integer(32)),
            ("5 * 2 + 10", Value::Integer(20)),
            ("5 + 2 * 10", Value::Integer(25)),
            ("5 * (2 + 10)", Value::Integer(60)),
            ("-5", Value::Integer(-5)),
            ("-10", Value::Integer(-10)),
            ("-50 + 100 + -50", Value::Integer(0)),
            ("(5 + 10 * 2 + 15 / 3) * 2 + -10", Value::Integer(50)),
        ];
        run_vm_tests(tests);
    }

    #[test]
    fn test_boolean_arithmetic() {
        let tests = vec![
            ("true", Value::Boolean(true)),
            ("false", Value::Boolean(false)),
            ("1 < 2", Value::Boolean(true)),
            ("1 > 2", Value::Boolean(false)),
            ("1 < 1", Value::Boolean(false)),
            ("1 > 1", Value::Boolean(false)),
            ("1 == 1", Value::Boolean(true)),
            ("1 != 1", Value::Boolean(false)),
            ("1 == 2", Value::Boolean(false)),
            ("1 != 2", Value::Boolean(true)),
            ("true == true", Value::Boolean(true)),
            ("false == false", Value::Boolean(true)),
            ("true == false", Value::Boolean(false)),
            ("true != false", Value::Boolean(true)),
            ("false != true", Value::Boolean(true)),
            ("(1 < 2) == true", Value::Boolean(true)),
            ("(1 < 2) == false", Value::Boolean(false)),
            ("(1 > 2) == true", Value::Boolean(false)),
            ("(1 > 2) == false", Value::Boolean(true)),
            ("!true", Value::Boolean(false)),
            ("!false", Value::Boolean(true)),
            ("!5", Value::Boolean(false)),
            ("!!true", Value::Boolean(true)),
            ("!!false", Value::Boolean(false)),
            ("!!5", Value::Boolean(true)),
            ("!(if (false) { 5; })", Value::Boolean(true)),
        ];
        run_vm_tests(tests);
    }

    #[test]
    fn test_conditionals() {
        let tests = vec![
            ("if (true) { 10 }", Value::Integer(10)),
            ("if (true) { 10 } else { 20 }", Value::Integer(10)),
            ("if (false) { 10 } else { 20 } ", Value::Integer(20)),
            ("if (1) { 10 }", Value::Integer(10)),
            ("if (1 < 2) { 10 }", Value::Integer(10)),
            ("if (1 < 2) { 10 } else { 20 }", Value::Integer(10)),
            ("if (1 > 2) { 10 } else { 20 }", Value::Integer(20)),
            ("if (1 > 2) { 10 }", Value::None),
            ("if (false) { 10 }", Value::None),
            ("if ((if (false) { 10 })) { 10 } else { 20 }", Value::Integer(20)),
            ("if (true) {} else { 10 }", Value::None),
        ];
        run_vm_tests(tests);
    }
}
