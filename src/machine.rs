use crate::native;
use crate::Opcode;
use crate::Value;
use std::collections::HashMap;

pub struct Machine<'a> {
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
    frees: Vec<Value>,
    length: usize,
    number: usize,
}

impl<'a> Machine<'a> {
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
                frees: vec![],
                length: usize::MIN,
                number: usize::MIN,
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
                | Opcode::Rem
                | Opcode::Bx
                | Opcode::Bo
                | Opcode::Ba
                | Opcode::Sl
                | Opcode::Sr
                | Opcode::Lt
                | Opcode::Gt
                | Opcode::Le
                | Opcode::Ge
                | Opcode::Eq
                | Opcode::Ne => {
                    let right = self.pop();
                    let left = self.pop();
                    match (left, right, opcode) {
                        (Value::Integer(left), Value::Integer(right), Opcode::Add) => self.push(Value::Integer(left + right)),
                        (Value::Integer(left), Value::Integer(right), Opcode::Sub) => self.push(Value::Integer(left - right)),
                        (Value::Integer(left), Value::Integer(right), Opcode::Mul) => self.push(Value::Integer(left * right)),
                        (Value::Integer(left), Value::Integer(right), Opcode::Div) => self.push(Value::Integer(left / right)),
                        (Value::Integer(left), Value::Integer(right), Opcode::Rem) => self.push(Value::Integer(left % right)),
                        (Value::Integer(left), Value::Integer(right), Opcode::Bx) => self.push(Value::Integer(left ^ right)),
                        (Value::Integer(left), Value::Integer(right), Opcode::Bo) => self.push(Value::Integer(left | right)),
                        (Value::Integer(left), Value::Integer(right), Opcode::Ba) => self.push(Value::Integer(left & right)),
                        (Value::Integer(left), Value::Integer(right), Opcode::Sl) => self.push(Value::Integer(left << right)),
                        (Value::Integer(left), Value::Integer(right), Opcode::Sr) => self.push(Value::Integer(left >> right)),
                        (Value::Integer(left), Value::Integer(right), Opcode::Lt) => self.push(Value::Boolean(left < right)),
                        (Value::Integer(left), Value::Integer(right), Opcode::Gt) => self.push(Value::Boolean(left > right)),
                        (Value::Integer(left), Value::Integer(right), Opcode::Le) => self.push(Value::Boolean(left <= right)),
                        (Value::Integer(left), Value::Integer(right), Opcode::Ge) => self.push(Value::Boolean(left >= right)),
                        (Value::Integer(left), Value::Integer(right), Opcode::Eq) => self.push(Value::Boolean(left == right)),
                        (Value::Integer(left), Value::Integer(right), Opcode::Ne) => self.push(Value::Boolean(left != right)),
                        (Value::Float(left), Value::Float(right), Opcode::Add) => self.push(Value::Float(left + right)),
                        (Value::Float(left), Value::Float(right), Opcode::Sub) => self.push(Value::Float(left - right)),
                        (Value::Float(left), Value::Float(right), Opcode::Mul) => self.push(Value::Float(left * right)),
                        (Value::Float(left), Value::Float(right), Opcode::Div) => self.push(Value::Float(left / right)),
                        (Value::Float(left), Value::Float(right), Opcode::Rem) => self.push(Value::Float(left % right)),
                        (Value::Float(left), Value::Float(right), Opcode::Lt) => self.push(Value::Boolean(left < right)),
                        (Value::Float(left), Value::Float(right), Opcode::Gt) => self.push(Value::Boolean(left > right)),
                        (Value::Float(left), Value::Float(right), Opcode::Le) => self.push(Value::Boolean(left <= right)),
                        (Value::Float(left), Value::Float(right), Opcode::Ge) => self.push(Value::Boolean(left >= right)),
                        (Value::Float(left), Value::Float(right), Opcode::Eq) => self.push(Value::Boolean(left == right)),
                        (Value::Float(left), Value::Float(right), Opcode::Ne) => self.push(Value::Boolean(left != right)),
                        (Value::Integer(left), Value::Float(right), Opcode::Add) => self.push(Value::Float(left as f64 + right)),
                        (Value::Integer(left), Value::Float(right), Opcode::Sub) => self.push(Value::Float(left as f64 - right)),
                        (Value::Integer(left), Value::Float(right), Opcode::Mul) => self.push(Value::Float(left as f64 * right)),
                        (Value::Integer(left), Value::Float(right), Opcode::Div) => self.push(Value::Float(left as f64 / right)),
                        (Value::Integer(left), Value::Float(right), Opcode::Rem) => self.push(Value::Float(left as f64 % right)),
                        (Value::Integer(left), Value::Float(right), Opcode::Lt) => self.push(Value::Boolean((left as f64) < right)),
                        (Value::Integer(left), Value::Float(right), Opcode::Gt) => self.push(Value::Boolean((left as f64) > right)),
                        (Value::Integer(left), Value::Float(right), Opcode::Le) => self.push(Value::Boolean((left as f64) <= right)),
                        (Value::Integer(left), Value::Float(right), Opcode::Ge) => self.push(Value::Boolean((left as f64) >= right)),
                        (Value::Integer(left), Value::Float(right), Opcode::Eq) => self.push(Value::Boolean((left as f64) == right)),
                        (Value::Integer(left), Value::Float(right), Opcode::Ne) => self.push(Value::Boolean((left as f64) != right)),
                        (Value::Float(left), Value::Integer(right), Opcode::Add) => self.push(Value::Float(left + right as f64)),
                        (Value::Float(left), Value::Integer(right), Opcode::Sub) => self.push(Value::Float(left - right as f64)),
                        (Value::Float(left), Value::Integer(right), Opcode::Mul) => self.push(Value::Float(left * right as f64)),
                        (Value::Float(left), Value::Integer(right), Opcode::Div) => self.push(Value::Float(left / right as f64)),
                        (Value::Float(left), Value::Integer(right), Opcode::Rem) => self.push(Value::Float(left % right as f64)),
                        (Value::Float(left), Value::Integer(right), Opcode::Lt) => self.push(Value::Boolean(left < right as f64)),
                        (Value::Float(left), Value::Integer(right), Opcode::Gt) => self.push(Value::Boolean(left > right as f64)),
                        (Value::Float(left), Value::Integer(right), Opcode::Le) => self.push(Value::Boolean(left <= right as f64)),
                        (Value::Float(left), Value::Integer(right), Opcode::Ge) => self.push(Value::Boolean(left >= right as f64)),
                        (Value::Float(left), Value::Integer(right), Opcode::Eq) => self.push(Value::Boolean(left == right as f64)),
                        (Value::Float(left), Value::Integer(right), Opcode::Ne) => self.push(Value::Boolean(left != right as f64)),
                        (Value::Boolean(left), Value::Boolean(right), Opcode::Eq) => self.push(Value::Boolean(left == right)),
                        (Value::Boolean(left), Value::Boolean(right), Opcode::Ne) => self.push(Value::Boolean(left != right)),
                        (Value::Boolean(left), Value::Boolean(right), Opcode::Bx) => self.push(Value::Boolean(left ^ right)),
                        (Value::Boolean(left), Value::Boolean(right), Opcode::Bo) => self.push(Value::Boolean(left | right)),
                        (Value::Boolean(left), Value::Boolean(right), Opcode::Ba) => self.push(Value::Boolean(left & right)),
                        (Value::String(left), Value::String(right), Opcode::Add) => self.push(Value::String(left + &right)),
                        (left, right, opcode) => {
                            panic!("unsupported types for binary operation: {} {:?} {}", left, opcode, right)
                        }
                    }
                }
                Opcode::True => self.push(Value::Boolean(true)),
                Opcode::False => self.push(Value::Boolean(false)),
                Opcode::Neg => {
                    let operand = self.pop();
                    match operand {
                        Value::Integer(value) => self.push(Value::Integer(-value)),
                        Value::Float(value) => self.push(Value::Float(-value)),
                        _ => panic!("unsupported types for negation: {}", operand),
                    }
                }
                Opcode::Not => {
                    let operand = self.pop();
                    match operand {
                        Value::Boolean(false) | Value::None => self.push(Value::Boolean(true)),
                        Value::Integer(value) => self.push(Value::Integer(!value)),
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
                    self.globals.resize(index + 1, Value::None);
                    self.globals[index] = value;
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
                        Value::Closure(opcodes, length, arity, frees) => {
                            if number != arity {
                                panic!("wrong number of arguments: want={}, got={}", arity, number);
                            }
                            self.enter(Frame {
                                opcodes,
                                fp: usize::MIN,
                                bp: self.sp - number,
                                frees,
                                length,
                                number,
                            });
                            self.sp = self.frame().bp + length;
                            self.stack.resize(self.sp, Value::None);
                        }
                        Value::Integer(index) => {
                            let arguments = self.stack[self.sp - number..self.sp].to_vec();
                            self.sp -= number;
                            self.push(native::call(index as isize)(arguments));
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
                    self.stack[index] = value;
                }
                Opcode::Native(index) => {
                    self.push(Value::Integer(index as i64));
                }
                Opcode::Closure(index, count) => match self.consts[index].clone() {
                    Value::Function(opcodes, length, arity) => {
                        let mut frees = Vec::with_capacity(count);
                        for i in 0..count {
                            frees.insert(i, self.stack[self.sp - count + i].clone());
                        }
                        self.sp -= count;
                        self.push(Value::Closure(opcodes, length, arity, frees));
                    }
                    non => panic!("non function: {}", non.kind()),
                },
                Opcode::GetFree(index) => {
                    let value = self.frame().frees[index].clone();
                    self.push(value)
                }
                Opcode::Current => {
                    let opcodes = self.frame().opcodes.clone();
                    let length = self.frame().length;
                    let number = self.frame().number;
                    let frees = self.frame().frees.clone();
                    self.push(Value::Closure(opcodes, length, number, frees))
                }
                Opcode::Field => {
                    let field = self.pop();
                    let object = self.pop();
                    match (object, field) {
                        (Value::Map(object), Value::String(field)) => match object.get(field.as_str()) {
                            Some(value) => self.push(value.clone()),
                            None => self.push(Value::None),
                        },
                        (object, field) => panic!("unsupported types for field: {}.{}", object, field),
                    }
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
    use crate::Machine;
    use crate::Parser;
    use crate::Value;
    use std::collections::HashMap;

    fn run_machine_tests(tests: Vec<(&str, Value)>) {
        for (text, value) in tests {
            let source = Parser::new(text).parse().unwrap();
            let mut compiler = Compiler::new();
            let mut globals = Vec::new();
            match compiler.compile(source) {
                Ok(opcodes) => {
                    println!("opcodes: {:?}", opcodes);
                    println!("consts: {:?}", compiler.consts());
                    let mut machine = Machine::new(compiler.consts(), &mut globals, opcodes);
                    machine.run();
                    println!("{} = {}", machine.past(), value);
                    assert_eq!(machine.past(), &value);
                }
                Err(message) => panic!("machine error: {}", message),
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
            ("7 % 3", Value::Integer(1)),
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
            ("!5", Value::Integer(-6)),
            ("!-3", Value::Integer(2)),
            ("5 ^ 3", Value::Integer(6)),
            ("5 | 3", Value::Integer(7)),
            ("5 & 3", Value::Integer(1)),
            ("5 << 2", Value::Integer(20)),
            ("5 >> 2", Value::Integer(1)),
            ("-5 >> 2", Value::Integer(-2)),
        ];
        run_machine_tests(tests);
    }

    #[test]
    fn test_float_arithmetic() {
        let tests = vec![
            ("1.0", Value::Float(1.0)),
            ("0.2", Value::Float(0.2)),
            ("1.0 + 0.2", Value::Float(1.2)),
            ("1.2 - 1.0", Value::Float(0.19999999999999996)),
            ("0.1 * 0.2", Value::Float(0.020000000000000004)),
            ("4.0 / 2.0", Value::Float(2.0)),
            ("7.2 % 3.0", Value::Float(1.2000000000000002)),
            ("5.0 / 2.0 * 2.0 + 1.0 - 0.5", Value::Float(5.5)),
            ("5.0 * (0.2 + 1.0)", Value::Float(6.0)),
            ("0.5 + 0.5 + 0.5 + 0.5 - 1.0", Value::Float(1.0)),
            ("0.2 * 0.2 * 0.2 * 0.2 * 0.2", Value::Float(0.00032000000000000013)),
            ("0.5 * 2.2 + 1.1", Value::Float(2.2)),
            ("0.5 + 0.2 * 10.0", Value::Float(2.5)),
            ("0.5 * (2.0 + 10.0)", Value::Float(6.0)),
            ("-0.5", Value::Float(-0.5)),
            ("-1.0", Value::Float(-1.0)),
            ("-5.0 + 10.0 + -5.0", Value::Float(0.0)),
            ("(0.5 + 1.5 * 0.2 + 1.5 / 3.0) * 2.0 + -1.0", Value::Float(1.6)),
        ];
        run_machine_tests(tests);
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
            ("1 <= 2", Value::Boolean(true)),
            ("1 >= 2", Value::Boolean(false)),
            ("1 <= 1", Value::Boolean(true)),
            ("1 >= 1", Value::Boolean(true)),
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
            // ("!5", Value::Boolean(false)),
            // ("!!5", Value::Boolean(true)),
            ("!!true", Value::Boolean(true)),
            ("!!false", Value::Boolean(false)),
            ("!(if (false) { 5; })", Value::Boolean(true)),
        ];
        run_machine_tests(tests);
    }

    #[test]
    fn test_if_conditional() {
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
        run_machine_tests(tests);
    }

    #[test]
    fn test_logical_junction() {
        let tests = vec![
            ("true || true", Value::Boolean(true)),
            ("true || false", Value::Boolean(true)),
            ("false || true", Value::Boolean(true)),
            ("false || false", Value::Boolean(false)),
            ("\"Cat\" || \"Dog\"", Value::String(String::from("Cat"))),
            ("false || \"Cat\"", Value::String(String::from("Cat"))),
            ("\"Cat\" || false", Value::String(String::from("Cat"))),
            ("\"\" || false", Value::String(String::from(""))),
            ("false || \"\"", Value::String(String::from(""))),
            ("true && true", Value::Boolean(true)),
            ("true &&  false", Value::Boolean(false)),
            ("false && true", Value::Boolean(false)),
            ("false && false", Value::Boolean(false)),
            ("\"Cat\" && \"Dog\"", Value::String(String::from("Dog"))),
            ("false && \"Cat\"", Value::Boolean(false)),
            ("\"Cat\" && false", Value::Boolean(false)),
            ("\"\" && false", Value::Boolean(false)),
            ("false && \"\"", Value::Boolean(false)),
            ("true || false && false", Value::Boolean(true)),
            ("(true || false) && false", Value::Boolean(false)),
            ("true && (false || false)", Value::Boolean(false)),
            ("2 == 3 || (4 < 0 && 1 == 1)", Value::Boolean(false)),
            ("true && false && 1 == 1", Value::Boolean(false)),
            ("let flag = true && false && 1 == 1;", Value::Boolean(false)),
        ];
        run_machine_tests(tests);
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
            ("let one = 1;let one = 2;", Value::Integer(2)),
            ("let one = 1;let one = 2;one", Value::Integer(2)),
            ("let one = 1;let two = 2;let one = 3;one", Value::Integer(3)),
        ];
        run_machine_tests(tests);
    }

    #[test]
    fn test_string_literal() {
        let tests = vec![
            (r#""hello world""#, Value::String(String::from("hello world"))),
            (r#""hello" + " world""#, Value::String(String::from("hello world"))),
            (r#""hello"+" world"+"!""#, Value::String(String::from("hello world!"))),
        ];
        run_machine_tests(tests);
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
        run_machine_tests(tests);
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
        run_machine_tests(tests);
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
        run_machine_tests(tests);
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
        run_machine_tests(tests);
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
            ("let one = fn() { let a = 1; let a = 2; a }; one();", Value::Integer(2)),
            ("let one = fn() { let a = 1; let b = 2; let a = 3; a }; one();", Value::Integer(3)),
        ];
        run_machine_tests(tests);
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
        run_machine_tests(tests);
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
        run_machine_tests(tests);
    }

    #[test]
    fn test_function_panic() {
        let tests = vec![
            ("fn() { 1; }(1);", "wrong number of arguments: want=0, got=1"),
            ("fn(a) { a; }();", "wrong number of arguments: want=1, got=0"),
            ("fn(a, b) { a + b; }(1);", "wrong number of arguments: want=2, got=1"),
        ];
        for (text, message) in tests {
            let result = std::panic::catch_unwind(|| run_machine_tests(vec![(text, Value::None)]));
            assert!(result.is_err());
            assert_eq!(*result.unwrap_err().downcast::<String>().unwrap(), message);
        }
    }

    #[test]
    fn test_function_native() {
        let tests = vec![
            ("length(\"\")", Value::Integer(0)),
            ("length(\"two\")", Value::Integer(3)),
            ("length(\"hello world\")", Value::Integer(11)),
            (
                "length(1)",
                Value::Error(String::from("function length not supported type Integer")),
            ),
            (
                "length(\"one\", \"two\")",
                Value::Error(String::from("wrong number of arguments. got=2, want=1")),
            ),
            ("length([])", Value::Integer(0)),
            ("length([1, 2, 3])", Value::Integer(3)),
        ];
        run_machine_tests(tests);
    }

    #[test]
    fn test_function_closure() {
        let tests = vec![
            (
                "
                let newClosure = fn(a) {
                    fn() { a; };
                };
                let closure = newClosure(99);
                closure();
                 ",
                Value::Integer(99),
            ),
            (
                "
                let newAdder = fn(a, b) {
                    fn(c) { a + b + c };
                };
                let adder = newAdder(1, 2);
                adder(8);
                ",
                Value::Integer(11),
            ),
            (
                "
                let newAdder = fn(a, b) {
                    let c = a + b;
                    fn(d) { c + d };
                };
                let adder = newAdder(1, 2);
                adder(8);
                ",
                Value::Integer(11),
            ),
            (
                "
                let newAdderOuter = fn(a, b) {
                    let c = a + b;
                    fn(d) {
                        let e = d + c;
                        fn(f) { e + f; };
                    };
                };
                let newAdderInner = newAdderOuter(1, 2)
                let adder = newAdderInner(3);
                adder(8);
                ",
                Value::Integer(14),
            ),
            (
                "
                let a = 1;
                let newAdderOuter = fn(b) {
                    fn(c) {
                        fn(d) { a + b + c + d };
                    };
                };
                let newAdderInner = newAdderOuter(2)
                let adder = newAdderInner(3);
                adder(8);
                ",
                Value::Integer(14),
            ),
            (
                "
                let newClosure = fn(a, b) {
                    let one = fn() { a; };
                    let two = fn() { b; };
                    fn() { one() + two(); };
                };
                let closure = newClosure(9, 90);
                closure();
                ",
                Value::Integer(99),
            ),
        ];
        run_machine_tests(tests);
    }

    #[test]
    fn test_function_recursive() {
        let tests = vec![
            (
                "
                let countDown = fn(x) {
                    if (x == 0) {
                        return 0;
                    } else {
                        countDown(x - 1);
                    }
                };
                countDown(1);
                ",
                Value::Integer(0),
            ),
            (
                "
                let countDown = fn(x) {
                    if (x == 0) {
                        return 0;
                    } else {
                        countDown(x - 1);
                    }
                };
                let wrapper = fn() {
                    countDown(1);
                };
                wrapper();
                ",
                Value::Integer(0),
            ),
            (
                "
                let wrapper = fn() {
                    let countDown = fn(x) {
                        if (x == 0) {
                            return 0;
                        } else {
                            countDown(x - 1);
                        }
                    };
                    countDown(1);
                };
                wrapper();
                ",
                Value::Integer(0),
            ),
        ];
        run_machine_tests(tests);
    }

    #[test]
    fn test_function_fibonacci() {
        let tests = vec![(
            "
            let fibonacci = fn(x) {
                if (x == 0) {
                    return 0;
                } else {
                    if (x == 1) {
                        return 1;
                    } else {
                        fibonacci(x - 1) + fibonacci(x - 2);
                    }
                }
            };
            fibonacci(15);
            ",
            Value::Integer(610),
        )];
        run_machine_tests(tests);
    }

    #[test]
    fn test_test_block() {
        let tests = vec![(
            "
            test case { 2 }
            case();
            ",
            Value::Integer(2),
        )];
        run_machine_tests(tests);
    }

    #[test]
    fn test_object_field() {
        let tests = vec![("{\"a\": 2}.a", Value::Integer(2))];
        run_machine_tests(tests);
    }

    #[test]
    fn test_request_literal() {
        let tests = vec![
            (
                "rq request(host)`\nGET http://{host}/get\nHost: {host}\nConnection: close\n`
                 request(\"httpbin.org\").status;
                ",
                Value::Integer(200),
            ),
            (
                "let host = \"httpbin.org\";
                 rq request()`POST http://{host}/post\nHost: {host}\nConnection: close\n`
                 request().status;
                ",
                Value::Integer(200),
            ),
        ];
        run_machine_tests(tests);
    }

    #[test]
    fn test_request_asserts() {
        let tests = vec![
            (
                r#"let request = fn(){
                   let asserts = [true, false];
                   let flag = true && asserts[0] && asserts[1];
                   flag}
                   request();
                "#,
                Value::Boolean(false),
            ),
            (
                r#"let request = fn(host){
                   let result = http(format("
                    GET http://{host}/get
                    Host: {host}
                    Connection: close
                    ", host, host));
                   let response = result.response;
                   let asserts = fn(status, version) { [{"result":(status == 200)}, {"result":(1 == 2)}, {"result":(1 == 1)}] }(response.status, response.version);
                   println("asserts: {asserts}", asserts);
                   let flag = (((true && (asserts[0]).result) && (asserts[1]).result) && (asserts[2]).result);
                   response};
                   request("httpbin.org").status;
                "#,
                Value::Integer(200),
            ),
            (
                "rq request(host)`\nGET http://{host}/get\nHost: {host}\nConnection: close\n`[status == 200]
                 request(\"httpbin.org\").status;
                ",
                Value::Integer(200),
            ),
            (
                "let host = \"httpbin.org\";
                 rq request()`POST http://{host}/post\nHost: {host}\nConnection: close\n`[status == 200, 1==2]
                 request().status;
                ",
                Value::Integer(200),
            ),
        ];
        run_machine_tests(tests);
    }
}
