use crate::Opcode;
use crate::Value;
use std::collections::HashMap;

pub struct Vm<'a> {
    consts: &'a Vec<Value>,
    globals: &'a mut Vec<Value>,
    stack: Vec<Value>,
    sp: usize,
    frames: Vec<Frame>,
    index: usize,
}

#[derive(Debug)]
struct Frame {
    opcodes: Vec<Opcode>,
    fp: usize,
    bp: usize,
}

impl<'a> Vm<'a> {
    pub fn new(consts: &'a Vec<Value>, globals: &'a mut Vec<Value>, opcodes: Vec<Opcode>) -> Self {
        Self {
            consts,
            globals,
            stack: Vec::new(),
            sp: usize::MIN,
            frames: vec![Frame {
                opcodes,
                fp: usize::MIN,
                bp: usize::MIN,
            }],
            index: usize::MIN,
        }
    }

    fn frame(&mut self) -> &mut Frame {
        &mut self.frames[self.index]
    }

    fn enter(&mut self, frame: Frame) {
        self.frames.push(frame);
        self.index += 1;
    }

    fn leave(&mut self) -> Frame {
        let frame = self.frames.remove(self.index);
        self.index -= 1;
        frame
    }

    pub fn run(&mut self) {
        while self.frame().fp < self.frame().opcodes.len() {
            let fp = self.frame().fp;
            let opcode = self.frame().opcodes[fp];
            self.frame().fp += 1;
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
                            self.push(Value::Integer(left + right));
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
                        (Value::String(left), Value::String(right), Opcode::Add) => {
                            self.push(Value::String(format!("{}{}", left, right)))
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
                Opcode::Jump(i) => self.frame().fp = i,
                Opcode::Judge(i) => {
                    let condition = self.pop();
                    match condition {
                        Value::Boolean(false) | Value::None => self.frame().fp = i,
                        _ => {}
                    }
                }
                Opcode::GetGlobal(index) => {
                    self.push(self.globals[index].clone());
                }
                Opcode::SetGlobal(index) => {
                    let value = self.pop();
                    self.globals.insert(index, value);
                }
                Opcode::Array(length) => {
                    let mut array = Vec::with_capacity(length);
                    for index in self.sp - length..self.sp {
                        array.push(self.stack[index].clone());
                    }
                    self.sp -= length;
                    self.push(Value::Array(array));
                }
                Opcode::Map(length) => {
                    let mut map = HashMap::new();
                    for index in (self.sp - length * 2..self.sp).step_by(2) {
                        let key = self.stack[index].clone();
                        let value = self.stack[index + 1].clone();
                        map.insert(key.to_string(), value);
                    }
                    self.sp -= length * 2;
                    self.push(Value::Map(map));
                }
                Opcode::Index => {
                    let index = self.pop();
                    let left = self.pop();
                    match (left, index) {
                        (Value::Array(array), Value::Integer(index)) => {
                            if index < 0 || index as usize >= array.len() {
                                self.push(Value::None);
                            } else {
                                self.push(array[index as usize].clone());
                            }
                        }
                        (Value::Map(map), key) => match map.get(key.to_string().as_str()) {
                            Some(value) => self.push(value.clone()),
                            None => self.push(Value::None),
                        },
                        (left, index) => panic!("unsupported types for index: {}[{}]", left, index),
                    }
                }
                Opcode::Call(number) => {
                    let function = self.stack.remove(self.sp - 1 - number);
                    self.sp -= 1;
                    match function {
                        Value::Function(opcodes, length, arity) => {
                            if number != arity {
                                panic!("wrong number of arguments: want={}, got={}", arity, number);
                            }
                            self.enter(Frame {
                                opcodes,
                                fp: usize::MIN,
                                bp: self.sp - number,
                            });
                            self.sp += length;
                            self.stack.resize(self.sp, Value::None);
                        }
                        non => panic!("calling non function: {}", non.kind()),
                    };
                }
                Opcode::Return => {
                    let value = self.pop();
                    let frame = self.leave();
                    self.sp = frame.bp;
                    self.push(value);
                }
                Opcode::GetLocal(mut index) => {
                    index += self.frame().bp;
                    self.push(self.stack[index].clone());
                }
                Opcode::SetLocal(mut index) => {
                    let value = self.pop();
                    index += self.frame().bp;
                    self.stack.insert(index, value);
                }
            }
        }
    }

    pub fn push(&mut self, value: Value) {
        self.stack.resize(self.sp + 1, Value::None);
        self.stack[self.sp] = value;
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
    use crate::Parser;
    use crate::Value;
    use crate::Vm;
    use std::collections::HashMap;

    fn run_vm_tests(tests: Vec<(&str, Value)>) {
        for (text, value) in tests {
            let source = Parser::new(text).parse().unwrap();
            let mut compiler = Compiler::new();
            let mut globals = Vec::new();
            match compiler.compile(&source) {
                Ok(opcodes) => {
                    println!("opcodes: {:?}", opcodes);
                    let mut vm = Vm::new(compiler.consts(), &mut globals, opcodes);
                    vm.run();
                    println!("{} = {}", vm.past(), value);
                    assert_eq!(vm.past(), &value);
                }
                Err(message) => panic!("vm error: {}", message),
            }
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
    fn test_test_if_conditional() {
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
            ("if (true) { 1; 2 } else { 3 }", Value::Integer(2)),
        ];
        run_vm_tests(tests);
    }

    #[test]
    fn test_let_global() {
        let tests = vec![
            ("let one = 1; one", Value::Integer(1)),
            ("let one = 1; let two = 2; one + two", Value::Integer(3)),
            ("let one = 1; let two = one + one; one + two", Value::Integer(3)),
            ("let one = 1; one;", Value::Integer(1)),
            ("let one = 1; let two = 2; one + two;", Value::Integer(3)),
            ("let one = 1; let two = one + one; one + two;", Value::Integer(3)),
        ];
        run_vm_tests(tests);
    }

    #[test]
    fn test_string_literal() {
        let tests = vec![
            (r#""hello world""#, Value::String(String::from("hello world"))),
            (r#""hello" + " world""#, Value::String(String::from("hello world"))),
            (r#""hello"+" world"+"!""#, Value::String(String::from("hello world!"))),
        ];
        run_vm_tests(tests);
    }

    #[test]
    fn test_array_literal() {
        let tests = vec![
            ("[]", Value::Array(vec![])),
            (
                "[1, 2, 3]",
                Value::Array(vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)]),
            ),
            (
                "[1 + 2, 3 - 4, 5 * 6]",
                Value::Array(vec![Value::Integer(3), Value::Integer(-1), Value::Integer(30)]),
            ),
        ];
        run_vm_tests(tests);
    }

    #[test]
    fn test_map_literal() {
        let tests = vec![
            ("{}", Value::Map(HashMap::new())),
            (
                "{1: 2, 2: 3}",
                Value::Map(HashMap::from_iter(vec![
                    (String::from("1"), Value::Integer(2)),
                    (String::from("2"), Value::Integer(3)),
                ])),
            ),
            (
                "{1 + 1: 2 * 2, 3 + 3: 4 * 4}",
                Value::Map(HashMap::from_iter(vec![
                    (String::from("2"), Value::Integer(4)),
                    (String::from("6"), Value::Integer(16)),
                ])),
            ),
        ];
        run_vm_tests(tests);
    }

    #[test]
    fn test_index_value() {
        let tests = vec![
            ("[1, 2, 3][1]", Value::Integer(2)),
            ("[1, 2, 3][0 + 2]", Value::Integer(3)),
            ("[[1, 1, 1]][0][0]", Value::Integer(1)),
            ("[][0]", Value::None),
            ("[1, 2, 3][99]", Value::None),
            ("[1][-1]", Value::None),
            ("{1: 1, 2: 2}[1]", Value::Integer(1)),
            ("{1: 1, 2: 2}[2]", Value::Integer(2)),
            ("{1: 1}[0]", Value::None),
            ("{}[0]", Value::None),
        ];
        run_vm_tests(tests);
    }

    #[test]
    fn test_function_call() {
        let tests = vec![
            (
                "let fivePlusTen = fn() { 5 + 10; };
                 fivePlusTen();",
                Value::Integer(15),
            ),
            (
                "let one = fn() { 1; };
                 let two = fn() { 2; };
                 one() + two()",
                Value::Integer(3),
            ),
            (
                "let a = fn() { 1 };
                 let b = fn() { a() + 1 };
                 let c = fn() { b() + 1 };
                 c();",
                Value::Integer(3),
            ),
            (
                "let earlyExit = fn() { return 99; 100; };
		         earlyExit();",
                Value::Integer(99),
            ),
            (
                "let earlyExit = fn() { return 99; return 100; };
		         earlyExit();",
                Value::Integer(99),
            ),
            (
                "let noReturn = fn() { };
		         noReturn();",
                Value::None,
            ),
            (
                "let noReturn = fn() { };
		         let noReturnTwo = fn() { noReturn(); };
		         noReturn();
		         noReturnTwo();",
                Value::None,
            ),
        ];
        run_vm_tests(tests);
    }

    #[test]
    fn test_let_local() {
        let tests = vec![
            (
                "let one = fn() { let one = 1; one };
                 one();",
                Value::Integer(1),
            ),
            (
                "let oneAndTwo = fn() { let one = 1; let two = 2; one + two; };
                 oneAndTwo();",
                Value::Integer(3),
            ),
            (
                "let oneAndTwo = fn() { let one = 1; let two = 2; one + two; };
		         let threeAndFour = fn() { let three = 3; let four = 4; three + four; };
		         oneAndTwo() + threeAndFour();",
                Value::Integer(10),
            ),
            (
                "let firstFoobar = fn() { let foobar = 50; foobar; };
		         let secondFoobar = fn() { let foobar = 100; foobar; };
		         firstFoobar() + secondFoobar();",
                Value::Integer(150),
            ),
            (
                "let globalSeed = 50;
                 let minusOne = fn() { let num = 1; globalSeed - num; }
                 let minusTwo = fn() { let num = 2; globalSeed - num; }
                 minusOne() + minusTwo();",
                Value::Integer(97),
            ),
        ];
        run_vm_tests(tests);
    }

    #[test]
    fn test_function_nesting() {
        let tests = vec![
            (
                "let returnsOne = fn() { 1; };
		         let returnsOneReturner = fn() { returnsOne; };
		         returnsOneReturner()();",
                Value::Integer(1),
            ),
            (
                "let returnsOneReturner = fn() {
			     let returnsOne = fn() { 1; };
			         returnsOne;
		         };
		         returnsOneReturner()();",
                Value::Integer(1),
            ),
        ];
        run_vm_tests(tests);
    }

    #[test]
    fn test_function_arguments() {
        let tests = vec![
            (
                "let identity = fn(a) { a; };
		         identity(4);",
                Value::Integer(4),
            ),
            (
                "let sum = fn(a, b) { a + b; };
		         sum(1, 2);",
                Value::Integer(3),
            ),
            (
                "let sum = fn(a, b) {
			        let c = a + b;
			        c;
		         };
		         sum(2, 3);",
                Value::Integer(5),
            ),
            (
                "let sum = fn(a, b) {
			        let c = a + b;
			        c;
		         };
		         sum(1, 2) + sum(3, 4);",
                Value::Integer(10),
            ),
            (
                "let sum = fn(a, b) {
			        let c = a + b;
			        c;
		         };
                 let outer = fn() {
		            sum(1, 2) + sum(3, 4);
                 };
                 outer();",
                Value::Integer(10),
            ),
            (
                "let globalNum = 10;
                 let sum = fn(a, b) {
                     let c = a + b;
                     c + globalNum;
                 };
                 let outer = fn() {
                     sum(1, 2) + sum(3, 4) + globalNum;
                 };
                 outer() + globalNum;",
                Value::Integer(50),
            ),
        ];
        run_vm_tests(tests);
    }

    #[test]
    fn test_function_panic() {
        let tests = vec![
            ("fn() { 1; }(1);", "wrong number of arguments: want=0, got=1"),
            ("fn(a) { a; }();", "wrong number of arguments: want=1, got=0"),
            ("fn(a, b) { a + b; }(1);", "wrong number of arguments: want=2, got=1"),
        ];
        for (text, message) in tests {
            let result = std::panic::catch_unwind(|| run_vm_tests(vec![(text, Value::None)]));
            assert!(result.is_err());
            assert_eq!(*result.unwrap_err().downcast::<String>().unwrap(), message);
        }
    }
}
