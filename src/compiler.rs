use crate::Expr;
use crate::Kind;
use crate::Opcode;
use crate::Symbol;
use crate::Symbols;
use crate::Value;

pub struct Compiler {
    consts: Vec<Value>,
    symbols: Symbols,
    scopes: Vec<Scope>,
    index: usize,
}

#[derive(Debug)]
struct Scope {
    opcodes: Vec<Opcode>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            consts: Vec::new(),
            symbols: Symbols::new(),
            scopes: vec![Scope { opcodes: vec![] }],
            index: usize::MIN,
        }
    }

    pub fn consts(&self) -> &Vec<Value> {
        &self.consts
    }

    fn scope(&mut self) -> &mut Scope {
        &mut self.scopes[self.index]
    }

    fn enter(&mut self) {
        self.symbols = self.symbols.clone().wrap();
        self.scopes.push(Scope { opcodes: vec![] });
        self.index += 1;
    }

    fn leave(&mut self) -> Vec<Opcode> {
        self.symbols = self.symbols.clone().peel();
        let scope = self.scopes.remove(self.index);
        self.index -= 1;
        scope.opcodes
    }

    fn emit(&mut self, opcode: Opcode) -> usize {
        self.scope().opcodes.push(opcode);
        self.scope().opcodes.len() - 1
    }

    fn save(&mut self, value: Value) -> usize {
        self.consts.push(value);
        self.consts.len() - 1
    }

    pub fn compile(&mut self, source: &Vec<Expr>) -> Result<Vec<Opcode>, String> {
        self.batch(source)?;
        let opcodes = self.scopes.remove(self.index).opcodes;
        self.scopes = vec![Scope { opcodes: vec![] }];
        Ok(opcodes)
    }

    fn batch(&mut self, source: &Vec<Expr>) -> Result<(), String> {
        for expr in source.iter() {
            self.assemble(expr)?;
            match expr {
                Expr::Let(..) | Expr::Return(..) => {}
                _ => {
                    self.emit(Opcode::Pop);
                }
            }
        }
        Ok(())
    }

    fn assemble(&mut self, expr: &Expr) -> Result<(), String> {
        match expr {
            Expr::Ident(_, name) => {
                match self.symbols.resolve(name) {
                    Some(Symbol::Global(index)) => self.emit(Opcode::GetGlobal(*index)),
                    Some(Symbol::Local(index)) => self.emit(Opcode::GetLocal(*index)),
                    None => Err(format!("Undefined variable: {}", name))?,
                };
            }
            Expr::Integer(_, integer) => {
                let integer = Value::Integer(*integer);
                let index = self.save(integer);
                self.emit(Opcode::Const(index));
            }
            Expr::Float(_, _) => todo!(),
            Expr::Boolean(_, boolean) => {
                if *boolean {
                    self.emit(Opcode::True);
                } else {
                    self.emit(Opcode::False);
                }
            }
            Expr::String(_, string) => {
                let string = Value::String(string.clone());
                let index = self.save(string);
                self.emit(Opcode::Const(index));
            }
            Expr::Let(_, name, value) => {
                self.assemble(value)?;
                let symbol = self.symbols.define(name);
                let opcode = match symbol {
                    Symbol::Global(index) => Opcode::SetGlobal(*index),
                    Symbol::Local(index) => Opcode::SetLocal(*index),
                };
                self.emit(opcode);
            }
            Expr::Return(_, value) => {
                self.assemble(value)?;
                self.emit(Opcode::Return);
            }
            Expr::Unary(token, right) => {
                self.assemble(right)?;
                match token.kind {
                    Kind::Minus => self.emit(Opcode::Minus),
                    Kind::Bang => self.emit(Opcode::Bang),
                    _ => Err(format!("Unknown operator: {}", token))?,
                };
            }
            Expr::Binary(token, left, right) => {
                self.assemble(left)?;
                self.assemble(right)?;
                match token.kind {
                    Kind::Plus => self.emit(Opcode::Add),
                    Kind::Minus => self.emit(Opcode::Sub),
                    Kind::Star => self.emit(Opcode::Mul),
                    Kind::Slash => self.emit(Opcode::Div),
                    Kind::Lt => self.emit(Opcode::Lt),
                    Kind::Gt => self.emit(Opcode::Gt),
                    Kind::Eq => self.emit(Opcode::Eq),
                    Kind::Ne => self.emit(Opcode::Ne),
                    _ => Err(format!("Unknown operator: {}", token))?,
                };
            }
            Expr::Paren(_, value) => self.assemble(value)?,
            Expr::If(_, condition, consequence, alternative) => {
                self.assemble(condition)?;
                let judge_index = self.scope().opcodes.len();
                if consequence.is_empty() {
                    self.emit(Opcode::None);
                } else {
                    self.batch(consequence)?;
                    if let Some(Opcode::Pop) = self.scope().opcodes.last() {
                        self.scope().opcodes.pop();
                    }
                }
                let judge_position = self.scope().opcodes.len() + 2;
                self.scope().opcodes.insert(judge_index, Opcode::Judge(judge_position));
                let jump_index = self.scope().opcodes.len();
                if alternative.is_empty() {
                    self.emit(Opcode::None);
                } else {
                    self.batch(alternative)?;
                    if let Some(Opcode::Pop) = self.scope().opcodes.last() {
                        self.scope().opcodes.pop();
                    }
                }
                let jump_position = self.scope().opcodes.len() + 1;
                self.scope().opcodes.insert(jump_index, Opcode::Jump(jump_position));
            }
            Expr::Function(_, parameters, body) => {
                self.enter();
                for parameter in parameters.iter() {
                    self.symbols.define(parameter);
                }
                if body.is_empty() {
                    self.emit(Opcode::None);
                } else {
                    self.batch(body)?;
                    if let Some(Opcode::Pop) = self.scope().opcodes.last() {
                        self.scope().opcodes.pop();
                    }
                }
                match self.scope().opcodes.last() {
                    Some(Opcode::Return) => {}
                    _ => {
                        self.emit(Opcode::Return);
                    }
                }
                let length = self.symbols.length();
                let opcodes = self.leave();
                let number = parameters.len();
                let index = self.save(Value::Function(opcodes, length, number));
                self.emit(Opcode::Const(index));
            }
            Expr::Call(_, function, arguments) => {
                self.assemble(function)?;
                for arg in arguments.iter() {
                    self.assemble(arg)?;
                }
                self.emit(Opcode::Call(arguments.len()));
            }
            Expr::Array(_, elements) => {
                for expr in elements.iter() {
                    self.assemble(expr)?;
                }
                self.emit(Opcode::Array(elements.len()));
            }
            Expr::Map(_, pairs) => {
                for (key, value) in pairs.iter() {
                    self.assemble(key)?;
                    self.assemble(value)?;
                }
                self.emit(Opcode::Map(pairs.len()));
            }
            Expr::Index(_, left, index) => {
                self.assemble(left)?;
                self.assemble(index)?;
                self.emit(Opcode::Index);
            }
            Expr::Field(_, _, _) => todo!(),
            Expr::Request(_, _, _, _) => todo!(),
            Expr::Test(_, _, _) => todo!(),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Compiler;
    use crate::Expr;
    use crate::Kind;
    use crate::Opcode;
    use crate::Parser;
    use crate::Token;
    use crate::Value;

    fn run_compiler_tests(tests: Vec<(&str, Vec<Value>, Vec<Opcode>)>) {
        for (text, consts, opcodes) in tests {
            let source = Parser::new(text).parse().unwrap();
            let mut compiler = Compiler::new();
            match compiler.compile(&source) {
                Ok(compiled) => {
                    assert_eq!(compiled, opcodes);
                    assert_eq!(compiler.consts, consts);
                }
                Err(message) => panic!("compile error: {}", message),
            }
        }
    }

    #[test]
    fn test_integer_arithmetic() {
        let tests = vec![
            (
                "1 + 2",
                vec![Value::Integer(1), Value::Integer(2)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Add, Opcode::Pop],
            ),
            (
                "1; 2",
                vec![Value::Integer(1), Value::Integer(2)],
                vec![Opcode::Const(0), Opcode::Pop, Opcode::Const(1), Opcode::Pop],
            ),
            (
                "1 - 2",
                vec![Value::Integer(1), Value::Integer(2)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Sub, Opcode::Pop],
            ),
            (
                "1 * 2",
                vec![Value::Integer(1), Value::Integer(2)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Mul, Opcode::Pop],
            ),
            (
                "2 / 1",
                vec![Value::Integer(2), Value::Integer(1)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Div, Opcode::Pop],
            ),
            (
                "-1",
                vec![Value::Integer(1)],
                vec![Opcode::Const(0), Opcode::Minus, Opcode::Pop],
            ),
        ];
        run_compiler_tests(tests);
    }

    #[test]
    fn test_boolean_arithmetic() {
        let tests = vec![
            ("true", vec![], vec![Opcode::True, Opcode::Pop]),
            ("false", vec![], vec![Opcode::False, Opcode::Pop]),
            (
                "1 < 2",
                vec![Value::Integer(1), Value::Integer(2)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Lt, Opcode::Pop],
            ),
            (
                "1 > 2",
                vec![Value::Integer(1), Value::Integer(2)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Gt, Opcode::Pop],
            ),
            (
                "1 == 2",
                vec![Value::Integer(1), Value::Integer(2)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Eq, Opcode::Pop],
            ),
            (
                "1 != 2",
                vec![Value::Integer(1), Value::Integer(2)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Ne, Opcode::Pop],
            ),
            (
                "true == false",
                vec![],
                vec![Opcode::True, Opcode::False, Opcode::Eq, Opcode::Pop],
            ),
            (
                "true != false",
                vec![],
                vec![Opcode::True, Opcode::False, Opcode::Ne, Opcode::Pop],
            ),
            ("!true", vec![], vec![Opcode::True, Opcode::Bang, Opcode::Pop]),
        ];
        run_compiler_tests(tests);
    }

    #[test]
    fn test_if_conditional() {
        let tests = vec![
            (
                "if (true) { 10 }; 3333;",
                vec![Value::Integer(10), Value::Integer(3333)],
                vec![
                    Opcode::True,
                    Opcode::Judge(4),
                    Opcode::Const(0),
                    Opcode::Jump(5),
                    Opcode::None,
                    Opcode::Pop,
                    Opcode::Const(1),
                    Opcode::Pop,
                ],
            ),
            (
                "if (true) { 10 } else { 20 }; 3333;",
                vec![Value::Integer(10), Value::Integer(20), Value::Integer(3333)],
                vec![
                    Opcode::True,
                    Opcode::Judge(4),
                    Opcode::Const(0),
                    Opcode::Jump(5),
                    Opcode::Const(1),
                    Opcode::Pop,
                    Opcode::Const(2),
                    Opcode::Pop,
                ],
            ),
            (
                "if (true) {} else { 10 }; 3333;",
                vec![Value::Integer(10), Value::Integer(3333)],
                vec![
                    Opcode::True,
                    Opcode::Judge(4),
                    Opcode::None,
                    Opcode::Jump(5),
                    Opcode::Const(0),
                    Opcode::Pop,
                    Opcode::Const(1),
                    Opcode::Pop,
                ],
            ),
            (
                "if (true) { 0; 1; 2 } else { 3; 4; 5 }",
                vec![
                    Value::Integer(0),
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                    Value::Integer(4),
                    Value::Integer(5),
                ],
                vec![
                    Opcode::True,
                    Opcode::Judge(8),
                    Opcode::Const(0),
                    Opcode::Pop,
                    Opcode::Const(1),
                    Opcode::Pop,
                    Opcode::Const(2),
                    Opcode::Jump(13),
                    Opcode::Const(3),
                    Opcode::Pop,
                    Opcode::Const(4),
                    Opcode::Pop,
                    Opcode::Const(5),
                    Opcode::Pop,
                ],
            ),
        ];
        run_compiler_tests(tests);
    }

    #[test]
    fn test_let_global() {
        let tests = vec![
            (
                "let one = 1;let two = 2;",
                vec![Value::Integer(1), Value::Integer(2)],
                vec![
                    Opcode::Const(0),
                    Opcode::SetGlobal(0),
                    Opcode::Const(1),
                    Opcode::SetGlobal(1),
                ],
            ),
            (
                "let one = 1;one;",
                vec![Value::Integer(1)],
                vec![
                    Opcode::Const(0),
                    Opcode::SetGlobal(0),
                    Opcode::GetGlobal(0),
                    Opcode::Pop,
                ],
            ),
            (
                "let one = 1;let two = one;two;",
                vec![Value::Integer(1)],
                vec![
                    Opcode::Const(0),
                    Opcode::SetGlobal(0),
                    Opcode::GetGlobal(0),
                    Opcode::SetGlobal(1),
                    Opcode::GetGlobal(1),
                    Opcode::Pop,
                ],
            ),
        ];
        run_compiler_tests(tests);
    }

    #[test]
    fn test_string_literal() {
        let tests = vec![
            (
                r#""hello world""#,
                vec![Value::String(String::from("hello world"))],
                vec![Opcode::Const(0), Opcode::Pop],
            ),
            (
                r#""hello" + " world""#,
                vec![
                    Value::String(String::from("hello")),
                    Value::String(String::from(" world")),
                ],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Add, Opcode::Pop],
            ),
        ];
        run_compiler_tests(tests);
    }

    #[test]
    fn test_array_literal() {
        let tests = vec![
            ("[]", vec![], vec![Opcode::Array(0), Opcode::Pop]),
            (
                "[1, 2, 3]",
                vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)],
                vec![
                    Opcode::Const(0),
                    Opcode::Const(1),
                    Opcode::Const(2),
                    Opcode::Array(3),
                    Opcode::Pop,
                ],
            ),
            (
                "[1 + 2, 3 - 4, 5 * 6]",
                vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                    Value::Integer(4),
                    Value::Integer(5),
                    Value::Integer(6),
                ],
                vec![
                    Opcode::Const(0),
                    Opcode::Const(1),
                    Opcode::Add,
                    Opcode::Const(2),
                    Opcode::Const(3),
                    Opcode::Sub,
                    Opcode::Const(4),
                    Opcode::Const(5),
                    Opcode::Mul,
                    Opcode::Array(3),
                    Opcode::Pop,
                ],
            ),
        ];
        run_compiler_tests(tests);
    }

    #[test]
    fn test_map_literal() {
        let tests = vec![
            ("{}", vec![], vec![Opcode::Map(0), Opcode::Pop]),
            (
                "{1: 2, 3: 4, 5: 6}",
                vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                    Value::Integer(4),
                    Value::Integer(5),
                    Value::Integer(6),
                ],
                vec![
                    Opcode::Const(0),
                    Opcode::Const(1),
                    Opcode::Const(2),
                    Opcode::Const(3),
                    Opcode::Const(4),
                    Opcode::Const(5),
                    Opcode::Map(3),
                    Opcode::Pop,
                ],
            ),
            (
                "{1: 2 + 3, 4: 5 * 6}",
                vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                    Value::Integer(4),
                    Value::Integer(5),
                    Value::Integer(6),
                ],
                vec![
                    Opcode::Const(0),
                    Opcode::Const(1),
                    Opcode::Const(2),
                    Opcode::Add,
                    Opcode::Const(3),
                    Opcode::Const(4),
                    Opcode::Const(5),
                    Opcode::Mul,
                    Opcode::Map(2),
                    Opcode::Pop,
                ],
            ),
        ];
        run_compiler_tests(tests);
    }

    #[test]
    fn test_index_value() {
        let tests = vec![
            (
                "[1, 2, 3][1 + 1]",
                vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                    Value::Integer(1),
                    Value::Integer(1),
                ],
                vec![
                    Opcode::Const(0),
                    Opcode::Const(1),
                    Opcode::Const(2),
                    Opcode::Array(3),
                    Opcode::Const(3),
                    Opcode::Const(4),
                    Opcode::Add,
                    Opcode::Index,
                    Opcode::Pop,
                ],
            ),
            (
                "{1: 2}[2 - 1]",
                vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(2),
                    Value::Integer(1),
                ],
                vec![
                    Opcode::Const(0),
                    Opcode::Const(1),
                    Opcode::Map(1),
                    Opcode::Const(2),
                    Opcode::Const(3),
                    Opcode::Sub,
                    Opcode::Index,
                    Opcode::Pop,
                ],
            ),
        ];
        run_compiler_tests(tests);
    }

    #[test]
    fn test_function_literal() {
        let tests = vec![
            (
                "fn() { return 5 + 10 }",
                vec![
                    Value::Integer(5),
                    Value::Integer(10),
                    Value::Function(
                        vec![Opcode::Const(0), Opcode::Const(1), Opcode::Add, Opcode::Return],
                        0,
                        0,
                    ),
                ],
                vec![Opcode::Const(2), Opcode::Pop],
            ),
            (
                "fn() { 6 + 8 }",
                vec![
                    Value::Integer(6),
                    Value::Integer(8),
                    Value::Function(
                        vec![Opcode::Const(0), Opcode::Const(1), Opcode::Add, Opcode::Return],
                        0,
                        0,
                    ),
                ],
                vec![Opcode::Const(2), Opcode::Pop],
            ),
            (
                "fn() { 1; 2 }",
                vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Function(
                        vec![Opcode::Const(0), Opcode::Pop, Opcode::Const(1), Opcode::Return],
                        0,
                        0,
                    ),
                ],
                vec![Opcode::Const(2), Opcode::Pop],
            ),
            (
                "fn() { }",
                vec![Value::Function(vec![Opcode::None, Opcode::Return], 0, 0)],
                vec![Opcode::Const(0), Opcode::Pop],
            ),
        ];
        run_compiler_tests(tests);
    }

    #[test]
    fn test_function_call() {
        let tests = vec![
            (
                "fn() { 2 }();",
                vec![
                    Value::Integer(2),
                    Value::Function(vec![Opcode::Const(0), Opcode::Return], 0, 0),
                ],
                vec![Opcode::Const(1), Opcode::Call(0), Opcode::Pop],
            ),
            (
                "let no_arg = fn() { 22 }; no_arg();",
                vec![
                    Value::Integer(22),
                    Value::Function(vec![Opcode::Const(0), Opcode::Return], 0, 0),
                ],
                vec![
                    Opcode::Const(1),
                    Opcode::SetGlobal(0),
                    Opcode::GetGlobal(0),
                    Opcode::Call(0),
                    Opcode::Pop,
                ],
            ),
            (
                "let oneArg = fn(a) { a };
                 oneArg(2);",
                vec![
                    Value::Function(vec![Opcode::GetLocal(0), Opcode::Return], 1, 1),
                    Value::Integer(2),
                ],
                vec![
                    Opcode::Const(0),
                    Opcode::SetGlobal(0),
                    Opcode::GetGlobal(0),
                    Opcode::Const(1),
                    Opcode::Call(1),
                    Opcode::Pop,
                ],
            ),
            (
                "let manyArg = fn(a, b, c) { a; b; c };
			     manyArg(6, 7, 8);",
                vec![
                    Value::Function(
                        vec![
                            Opcode::GetLocal(0),
                            Opcode::Pop,
                            Opcode::GetLocal(1),
                            Opcode::Pop,
                            Opcode::GetLocal(2),
                            Opcode::Return,
                        ],
                        3,
                        3,
                    ),
                    Value::Integer(6),
                    Value::Integer(7),
                    Value::Integer(8),
                ],
                vec![
                    Opcode::Const(0),
                    Opcode::SetGlobal(0),
                    Opcode::GetGlobal(0),
                    Opcode::Const(1),
                    Opcode::Const(2),
                    Opcode::Const(3),
                    Opcode::Call(3),
                    Opcode::Pop,
                ],
            ),
        ];
        run_compiler_tests(tests);
    }

    #[test]
    fn test_variable_scope() {
        let mut compiler = Compiler::new();
        assert_eq!(compiler.index, 0);
        let _ = compiler.assemble(&Expr::Let(
            Token {
                kind: Kind::Let,
                literal: String::from("let"),
            },
            String::from("a"),
            Box::new(Expr::Integer(
                Token {
                    kind: Kind::Integer,
                    literal: String::from("2"),
                },
                2,
            )),
        ));
        assert_eq!(compiler.scope().opcodes.last(), Some(&Opcode::SetGlobal(0)));
        compiler.emit(Opcode::Mul);
        compiler.enter();
        assert_eq!(compiler.index, 1);
        compiler.emit(Opcode::Sub);
        assert_eq!(compiler.scope().opcodes.len(), 1);
        assert_eq!(compiler.scope().opcodes.last(), Some(&Opcode::Sub));
        compiler.leave();
        assert_eq!(compiler.index, 0);
        compiler.emit(Opcode::Add);
        assert_eq!(compiler.scope().opcodes.len(), 4);
        assert_eq!(compiler.scope().opcodes.last(), Some(&Opcode::Add));
    }

    #[test]
    fn test_let_local() {
        let tests = vec![
            (
                "let num = 55;
			     fn() { num }",
                vec![
                    Value::Integer(55),
                    Value::Function(vec![Opcode::GetGlobal(0), Opcode::Return], 0, 0),
                ],
                vec![Opcode::Const(0), Opcode::SetGlobal(0), Opcode::Const(1), Opcode::Pop],
            ),
            (
                "fn() {
				   let num = 55;
				   num
			     }",
                vec![
                    Value::Integer(55),
                    Value::Function(
                        vec![
                            Opcode::Const(0),
                            Opcode::SetLocal(0),
                            Opcode::GetLocal(0),
                            Opcode::Return,
                        ],
                        1,
                        0,
                    ),
                ],
                vec![Opcode::Const(1), Opcode::Pop],
            ),
            (
                "fn() {
				   let a = 55;
				   let b = 77;
				   a + b
			     }",
                vec![
                    Value::Integer(55),
                    Value::Integer(77),
                    Value::Function(
                        vec![
                            Opcode::Const(0),
                            Opcode::SetLocal(0),
                            Opcode::Const(1),
                            Opcode::SetLocal(1),
                            Opcode::GetLocal(0),
                            Opcode::GetLocal(1),
                            Opcode::Add,
                            Opcode::Return,
                        ],
                        2,
                        0,
                    ),
                ],
                vec![Opcode::Const(2), Opcode::Pop],
            ),
        ];
        run_compiler_tests(tests);
    }
}
