use crate::native;
use crate::Expr;
use crate::Kind;
use crate::Opcode;
use crate::Symbol;
use crate::Symbols;
use crate::Token;
use crate::Value;

#[derive(Clone)]
pub struct Compiler {
    consts: Vec<Value>,
    symbols: Symbols,
    scopes: Vec<Scope>,
    index: usize,
}

#[derive(Clone, Debug)]
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
        self.symbols = self.symbols.to_owned().wrap();
        self.scopes.push(Scope { opcodes: vec![] });
        self.index += 1;
    }

    fn leave(&mut self) -> Vec<Opcode> {
        self.symbols = self.symbols.to_owned().peel();
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

    pub fn compile(&mut self, source: Vec<Expr>) -> Result<Vec<Opcode>, String> {
        self.batch(source)?;
        let opcodes = self.scopes.remove(self.index).opcodes;
        self.scopes = vec![Scope { opcodes: vec![] }];
        Ok(opcodes)
    }

    fn batch(&mut self, source: Vec<Expr>) -> Result<(), String> {
        for expr in source.into_iter() {
            let pop = !matches!(&expr, Expr::Let(..) | Expr::Return(..) | Expr::Request(..) | Expr::Test(..));
            self.assemble(expr)?;
            pop.then(|| self.emit(Opcode::Pop));
        }
        Ok(())
    }

    fn block(&mut self, body: Vec<Expr>, flag: bool) -> Result<(), String> {
        if body.is_empty() {
            self.emit(Opcode::None);
        } else {
            self.batch(body)?;
            if let Some(Opcode::Pop) = self.scope().opcodes.last() {
                self.scope().opcodes.pop();
            }
        }
        flag.then(|| match self.scope().opcodes.last() {
            Some(Opcode::Return) => {}
            _ => {
                self.emit(Opcode::Return);
            }
        });
        Ok(())
    }

    fn assemble(&mut self, expr: Expr) -> Result<(), String> {
        match expr {
            Expr::Ident(name) => {
                let opcode = match self.symbols.resolve(name.as_str()) {
                    Some(Symbol::Global(index)) => Opcode::GetGlobal(*index),
                    Some(Symbol::Local(index)) => Opcode::GetLocal(*index),
                    None => match native::get(name.as_str()) {
                        Some(index) => Opcode::Native(index),
                        None => Err(format!("Undefined variable: {}", name))?,
                    },
                };
                self.emit(opcode);
            }
            Expr::Integer(integer) => {
                let integer = Value::Integer(integer);
                let index = self.save(integer);
                self.emit(Opcode::Const(index));
            }
            Expr::Float(float) => {
                let float = Value::Float(float);
                let index = self.save(float);
                self.emit(Opcode::Const(index));
            }
            Expr::Boolean(boolean) => {
                if boolean {
                    self.emit(Opcode::True);
                } else {
                    self.emit(Opcode::False);
                }
            }
            Expr::String(string) => {
                let string = Value::String(string);
                let index = self.save(string);
                self.emit(Opcode::Const(index));
            }
            Expr::Let(name, value) => {
                self.assemble(*value)?;
                // TODO fixme
                let symbol = match self.symbols.get(name.as_str()) {
                    Some(symbol) => symbol,
                    None => self.symbols.define(name.as_str()),
                };
                let opcode = match symbol {
                    Symbol::Global(index) => Opcode::SetGlobal(*index),
                    Symbol::Local(index) => Opcode::SetLocal(*index),
                };
                self.emit(opcode);
            }
            Expr::Return(value) => {
                self.assemble(*value)?;
                self.emit(Opcode::Return);
            }
            Expr::Unary(token, right) => {
                self.assemble(*right)?;
                match token.kind {
                    Kind::Minus => self.emit(Opcode::Neg),
                    Kind::Bang => self.emit(Opcode::Not),
                    _ => Err(format!("Unknown operator: {}", token))?,
                };
            }
            Expr::Binary(Token { kind: Kind::Lo, .. }, left, right) => {
                self.assemble(Expr::Let(String::from("_left"), Box::new(*left)))?;
                self.assemble(Expr::If(
                    Box::new(Expr::Ident(String::from("_left"))),
                    vec![Expr::Ident(String::from("_left"))],
                    vec![*right],
                ))?;
            }
            Expr::Binary(Token { kind: Kind::La, .. }, left, right) => {
                self.assemble(Expr::Let(String::from("_left"), Box::new(*left)))?;
                self.assemble(Expr::If(
                    Box::new(Expr::Ident(String::from("_left"))),
                    vec![*right],
                    vec![Expr::Ident(String::from("_left"))],
                ))?;
            }
            Expr::Binary(token, left, right) => {
                self.assemble(*left)?;
                self.assemble(*right)?;
                match token.kind {
                    Kind::Plus => self.emit(Opcode::Add),
                    Kind::Minus => self.emit(Opcode::Sub),
                    Kind::Star => self.emit(Opcode::Mul),
                    Kind::Slash => self.emit(Opcode::Div),
                    Kind::Percent => self.emit(Opcode::Rem),
                    Kind::Bx => self.emit(Opcode::Bx),
                    Kind::Bo => self.emit(Opcode::Bo),
                    Kind::Ba => self.emit(Opcode::Ba),
                    Kind::Ll => self.emit(Opcode::Sl),
                    Kind::Gg => self.emit(Opcode::Sr),
                    Kind::Lt => self.emit(Opcode::Lt),
                    Kind::Gt => self.emit(Opcode::Gt),
                    Kind::Le => self.emit(Opcode::Le),
                    Kind::Ge => self.emit(Opcode::Ge),
                    Kind::Eq => self.emit(Opcode::Eq),
                    Kind::Ne => self.emit(Opcode::Ne),
                    _ => Err(format!("Unknown operator: {}", token))?,
                };
            }
            Expr::Paren(value) => self.assemble(*value)?,
            Expr::If(condition, consequence, alternative) => {
                self.assemble(*condition)?;
                let judge_index = self.emit(Opcode::Judge(usize::MAX));
                self.block(consequence, false)?;
                let jump_index = self.emit(Opcode::Jump(usize::MAX));
                let judge_position = self.scope().opcodes.len();
                if let Some(opcode) = self.scope().opcodes.get_mut(judge_index) {
                    *opcode = Opcode::Judge(judge_position)
                }
                self.block(alternative, false)?;
                let jump_position = self.scope().opcodes.len();
                if let Some(opcode) = self.scope().opcodes.get_mut(jump_index) {
                    *opcode = Opcode::Jump(jump_position)
                }
            }
            Expr::Function(_, parameters, body) => {
                self.enter();
                for parameter in parameters.iter() {
                    self.symbols.define(parameter);
                }
                self.block(body, true)?;
                let length = self.symbols.length();
                let opcodes = self.leave();
                let number = parameters.len();
                let index = self.save(Value::Function(opcodes, length, number));
                self.emit(Opcode::Const(index));
            }
            Expr::Call(function, arguments) => {
                self.assemble(*function)?;
                let length = arguments.len();
                for arg in arguments.into_iter() {
                    self.assemble(arg)?;
                }
                self.emit(Opcode::Call(length));
            }
            Expr::Array(elements) => {
                let length = elements.len();
                for expr in elements.into_iter() {
                    self.assemble(expr)?;
                }
                self.emit(Opcode::Array(length));
            }
            Expr::Map(pairs) => {
                let length = pairs.len();
                for (key, value) in pairs.into_iter() {
                    self.assemble(key)?;
                    self.assemble(value)?;
                }
                self.emit(Opcode::Map(length));
            }
            Expr::Index(left, index) => {
                self.assemble(*left)?;
                self.assemble(*index)?;
                self.emit(Opcode::Index);
            }
            Expr::Field(object, field) => {
                self.assemble(*object)?;
                let field = Value::String(field);
                let index = self.save(field);
                self.emit(Opcode::Const(index));
                self.emit(Opcode::Field);
            }
            Expr::Request(name, parameters, message, asserts) => {
                let regex = regex::Regex::new(r"\{\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*\}").unwrap();
                let matches = regex.find_iter(message.as_str());
                let mut places = Vec::new();
                matches.for_each(|m| {
                    let literal = &m.as_str()[1..m.as_str().len() - 1];
                    places.push(Expr::Ident(String::from(literal)))
                });
                places.insert(usize::MIN, Expr::String(message));
                let format = Expr::Call(Box::new(Expr::Ident(String::from("format"))), places);
                let body = vec![
                    // let record = http(format(...));
                    Expr::Let(
                        String::from("record"),
                        Box::new(Expr::Call(Box::new(Expr::Ident(String::from("http"))), vec![format])),
                    ),
                    // let response = record.response;
                    Expr::Let(
                        String::from("response"),
                        Box::new(Expr::Field(Box::new(Expr::Ident(String::from("record"))), String::from("response"))),
                    ),
                    // let asserts = fn(status, version){
                    //     [{ "expr": "{left} {token} {right}",
                    //       "left": left,
                    //       "compare": token,
                    //       "right": right,
                    //       "result": (left token right)
                    //     }, ...]
                    // }(response.status, response.version);
                    Expr::Let(
                        String::from("asserts"),
                        Box::new(Expr::Call(
                            Box::new(Expr::Function(
                                None,
                                vec![String::from("status"), String::from("version")],
                                vec![Expr::Array(
                                    asserts
                                        .into_iter()
                                        .filter_map(|assert| match assert {
                                            Expr::Binary(token, left, right) => Some(Expr::Map(vec![
                                                (Expr::String(String::from("expr")), Expr::String(format!("{left} {token} {right}"))),
                                                (Expr::String(String::from("left")), *left.clone()),
                                                (Expr::String(String::from("compare")), Expr::String(format!("{token}"))),
                                                (Expr::String(String::from("right")), *right.clone()),
                                                (Expr::String(String::from("result")), Expr::Binary(token, left, right)),
                                            ])),
                                            _ => None,
                                        })
                                        .collect(),
                                )],
                            )),
                            vec![
                                Expr::Field(Box::new(Expr::Ident(String::from("response"))), String::from("status")),
                                Expr::Field(Box::new(Expr::Ident(String::from("response"))), String::from("version")),
                            ],
                        )),
                    ),
                    // track({"name": name,
                    //         "request": record.request,
                    //         "response": record.response,
                    //         "time": record.time,
                    //         "asserts": asserts
                    //         "error": record.error });
                    Expr::Call(
                        Box::new(Expr::Ident(String::from("track"))),
                        vec![Expr::Map(vec![
                            (Expr::String(String::from("name")), Expr::String(name.to_owned())),
                            (
                                Expr::String(String::from("request")),
                                Expr::Field(Box::new(Expr::Ident(String::from("record"))), String::from("request")),
                            ),
                            (
                                Expr::String(String::from("response")),
                                Expr::Field(Box::new(Expr::Ident(String::from("record"))), String::from("response")),
                            ),
                            (
                                Expr::String(String::from("time")),
                                Expr::Field(Box::new(Expr::Ident(String::from("record"))), String::from("time")),
                            ),
                            (Expr::String(String::from("asserts")), Expr::Ident(String::from("asserts"))),
                            (
                                Expr::String(String::from("error")),
                                Expr::Field(Box::new(Expr::Ident(String::from("record"))), String::from("error")),
                            ),
                        ])],
                    ),
                    // return response;
                    Expr::Ident(String::from("response")),
                ];
                self.assemble(Expr::Let(name, Box::new(Expr::Function(None, parameters, body))))?;
            }
            Expr::Test(name, block) => {
                self.enter();
                self.block(block, true)?;
                let opcodes = self.leave();
                let index = self.save(Value::Function(opcodes, usize::MIN, usize::MIN));
                self.emit(Opcode::Const(index));
                let symbol = self.symbols.define(name.as_str());
                let opcode = match symbol {
                    Symbol::Global(index) => Opcode::SetGlobal(*index),
                    Symbol::Local(_) => Err(format!("Cannot define test in local: {}", name))?,
                };
                self.emit(opcode);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Compiler;
    use crate::Expr;
    use crate::Opcode;
    use crate::Parser;
    use crate::Value;

    fn run_compiler_tests(tests: Vec<(&str, Vec<Value>, Vec<Opcode>)>) {
        for (text, consts, opcodes) in tests {
            let source = Parser::new(text).parse().unwrap();
            let mut compiler = Compiler::new();
            match compiler.compile(source) {
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
                "7 % 3",
                vec![Value::Integer(7), Value::Integer(3)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Rem, Opcode::Pop],
            ),
            ("-1", vec![Value::Integer(1)], vec![Opcode::Const(0), Opcode::Neg, Opcode::Pop]),
            (
                "1 ^ 2",
                vec![Value::Integer(1), Value::Integer(2)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Bx, Opcode::Pop],
            ),
            (
                "1 | 2",
                vec![Value::Integer(1), Value::Integer(2)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Bo, Opcode::Pop],
            ),
            (
                "1 & 2",
                vec![Value::Integer(1), Value::Integer(2)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Ba, Opcode::Pop],
            ),
            ("!1", vec![Value::Integer(1)], vec![Opcode::Const(0), Opcode::Not, Opcode::Pop]),
            (
                "1 << 2",
                vec![Value::Integer(1), Value::Integer(2)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Sl, Opcode::Pop],
            ),
            (
                "1 >> 2",
                vec![Value::Integer(1), Value::Integer(2)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Sr, Opcode::Pop],
            ),
        ];
        run_compiler_tests(tests);
    }

    #[test]
    fn test_float_arithmetic() {
        let tests = vec![
            (
                "1.0 + 2.0",
                vec![Value::Float(1.0), Value::Float(2.0)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Add, Opcode::Pop],
            ),
            (
                "1.0; 0.20",
                vec![Value::Float(1.0), Value::Float(0.20)],
                vec![Opcode::Const(0), Opcode::Pop, Opcode::Const(1), Opcode::Pop],
            ),
            (
                "1.0 - 0.2",
                vec![Value::Float(1.0), Value::Float(0.2)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Sub, Opcode::Pop],
            ),
            (
                "1.0 * 2.0",
                vec![Value::Float(1.0), Value::Float(2.0)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Mul, Opcode::Pop],
            ),
            (
                "1.0 / 2.0",
                vec![Value::Float(1.0), Value::Float(2.0)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Div, Opcode::Pop],
            ),
            ("-1.0", vec![Value::Float(1.0)], vec![Opcode::Const(0), Opcode::Neg, Opcode::Pop]),
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
                "1 <= 2",
                vec![Value::Integer(1), Value::Integer(2)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Le, Opcode::Pop],
            ),
            (
                "1 >= 2",
                vec![Value::Integer(1), Value::Integer(2)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Ge, Opcode::Pop],
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
            ("true == false", vec![], vec![Opcode::True, Opcode::False, Opcode::Eq, Opcode::Pop]),
            ("true != false", vec![], vec![Opcode::True, Opcode::False, Opcode::Ne, Opcode::Pop]),
            ("!true", vec![], vec![Opcode::True, Opcode::Not, Opcode::Pop]),
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
            (
                "
                if (false) { 0 } else { if (false) { 1 } else { 2 } }; 3;
                ",
                vec![Value::Integer(0), Value::Integer(1), Value::Integer(2), Value::Integer(3)],
                vec![
                    Opcode::False,
                    Opcode::Judge(4),
                    Opcode::Const(0),
                    Opcode::Jump(9),
                    Opcode::False,
                    Opcode::Judge(8),
                    Opcode::Const(1),
                    Opcode::Jump(9),
                    Opcode::Const(2),
                    Opcode::Pop,
                    Opcode::Const(3),
                    Opcode::Pop,
                ],
            ),
        ];
        run_compiler_tests(tests);
    }

    #[test]
    fn test_logical_junction() {
        let tests = vec![
            (
                "true || true",
                vec![],
                vec![
                    Opcode::True,
                    Opcode::SetGlobal(0),
                    Opcode::GetGlobal(0),
                    Opcode::Judge(6),
                    Opcode::GetGlobal(0),
                    Opcode::Jump(7),
                    Opcode::True,
                    Opcode::Pop,
                ],
            ),
            (
                "true && true",
                vec![],
                vec![
                    Opcode::True,
                    Opcode::SetGlobal(0),
                    Opcode::GetGlobal(0),
                    Opcode::Judge(6),
                    Opcode::True,
                    Opcode::Jump(7),
                    Opcode::GetGlobal(0),
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
                vec![Opcode::Const(0), Opcode::SetGlobal(0), Opcode::Const(1), Opcode::SetGlobal(1)],
            ),
            (
                "let one = 1;one;",
                vec![Value::Integer(1)],
                vec![Opcode::Const(0), Opcode::SetGlobal(0), Opcode::GetGlobal(0), Opcode::Pop],
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
            (
                "let one = 1;let one = 2;",
                vec![Value::Integer(1), Value::Integer(2)],
                vec![Opcode::Const(0), Opcode::SetGlobal(0), Opcode::Const(1), Opcode::SetGlobal(0)],
            ),
            (
                "let one = 1;let two = 2;let one = 3;",
                vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)],
                vec![
                    Opcode::Const(0),
                    Opcode::SetGlobal(0),
                    Opcode::Const(1),
                    Opcode::SetGlobal(1),
                    Opcode::Const(2),
                    Opcode::SetGlobal(0),
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
                vec![Value::String(String::from("hello")), Value::String(String::from(" world"))],
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
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Const(2), Opcode::Array(3), Opcode::Pop],
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
                vec![Value::Integer(1), Value::Integer(2), Value::Integer(2), Value::Integer(1)],
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
                    Value::Function(vec![Opcode::Const(0), Opcode::Const(1), Opcode::Add, Opcode::Return], 0, 0),
                ],
                vec![Opcode::Const(2), Opcode::Pop],
            ),
            (
                "fn() { 6 + 8 }",
                vec![
                    Value::Integer(6),
                    Value::Integer(8),
                    Value::Function(vec![Opcode::Const(0), Opcode::Const(1), Opcode::Add, Opcode::Return], 0, 0),
                ],
                vec![Opcode::Const(2), Opcode::Pop],
            ),
            (
                "fn() { 1; 2 }",
                vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Function(vec![Opcode::Const(0), Opcode::Pop, Opcode::Const(1), Opcode::Return], 0, 0),
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
                vec![Value::Integer(2), Value::Function(vec![Opcode::Const(0), Opcode::Return], 0, 0)],
                vec![Opcode::Const(1), Opcode::Call(0), Opcode::Pop],
            ),
            (
                "let no_arg = fn() { 22 }; no_arg();",
                vec![Value::Integer(22), Value::Function(vec![Opcode::Const(0), Opcode::Return], 0, 0)],
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
                vec![Value::Function(vec![Opcode::GetLocal(0), Opcode::Return], 1, 1), Value::Integer(2)],
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
        let _ = compiler.assemble(Expr::Let(String::from("a"), Box::new(Expr::Integer(2))));
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
                        vec![Opcode::Const(0), Opcode::SetLocal(0), Opcode::GetLocal(0), Opcode::Return],
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
            (
                "fn() {
				   let a = 1;
				   let a = 2;
			     }",
                vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Function(
                        vec![
                            Opcode::Const(0),
                            Opcode::SetLocal(0),
                            Opcode::Const(1),
                            Opcode::SetLocal(0),
                            Opcode::Return,
                        ],
                        1,
                        0,
                    ),
                ],
                vec![Opcode::Const(2), Opcode::Pop],
            ),
            (
                "fn() {
				   let a = 1;
                   let b = 2;
				   let a = 3;
			     }",
                vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                    Value::Function(
                        vec![
                            Opcode::Const(0),
                            Opcode::SetLocal(0),
                            Opcode::Const(1),
                            Opcode::SetLocal(1),
                            Opcode::Const(2),
                            Opcode::SetLocal(0),
                            Opcode::Return,
                        ],
                        2,
                        0,
                    ),
                ],
                vec![Opcode::Const(3), Opcode::Pop],
            ),
        ];
        run_compiler_tests(tests);
    }

    #[test]
    fn test_function_native() {
        let tests = vec![
            (
                r#"
                length([]);
                println("Hello world!");
                "#,
                vec![Value::String(String::from("Hello world!"))],
                vec![
                    Opcode::Native(3),
                    Opcode::Array(0),
                    Opcode::Call(1),
                    Opcode::Pop,
                    Opcode::Native(1),
                    Opcode::Const(0),
                    Opcode::Call(1),
                    Opcode::Pop,
                ],
            ),
            (
                r#"
                fn() { length([]) }
                "#,
                vec![Value::Function(
                    vec![Opcode::Native(3), Opcode::Array(0), Opcode::Call(1), Opcode::Return],
                    0,
                    0,
                )],
                vec![Opcode::Const(0), Opcode::Pop],
            ),
        ];
        run_compiler_tests(tests);
    }

    #[test]
    fn test_test_block() {
        let tests = vec![(
            "
            test case { 2 }
            case();
            ",
            vec![Value::Integer(2), Value::Function(vec![Opcode::Const(0), Opcode::Return], 0, 0)],
            vec![
                Opcode::Const(1),
                Opcode::SetGlobal(0),
                Opcode::GetGlobal(0),
                Opcode::Call(0),
                Opcode::Pop,
            ],
        )];
        run_compiler_tests(tests);
    }

    #[test]
    fn test_object_field() {
        let tests = vec![(
            "{\"a\": 2}.a",
            vec![
                Value::String(String::from("a")),
                Value::Integer(2),
                Value::String(String::from("a")),
            ],
            vec![
                Opcode::Const(0),
                Opcode::Const(1),
                Opcode::Map(1),
                Opcode::Const(2),
                Opcode::Field,
                Opcode::Pop,
            ],
        )];
        run_compiler_tests(tests);
    }

    #[test]
    fn test_request_literal() {
        let tests = vec![(
            "rq request(host)`\nGET http://{host}/api\nHost: example.com\n`",
            vec![
                Value::String(String::from("\nGET http://{host}/api\nHost: example.com\n")),
                Value::String(String::from("response")),
                Value::Function(vec![Opcode::Array(0), Opcode::Return], 2, 2),
                Value::String(String::from("status")),
                Value::String(String::from("version")),
                Value::String(String::from("name")),
                Value::String(String::from("request")),
                Value::String(String::from("request")),
                Value::String(String::from("request")),
                Value::String(String::from("response")),
                Value::String(String::from("response")),
                Value::String(String::from("time")),
                Value::String(String::from("time")),
                Value::String(String::from("asserts")),
                Value::String(String::from("error")),
                Value::String(String::from("error")),
                Value::Function(
                    vec![
                        Opcode::Native(-1),
                        Opcode::Native(2),
                        Opcode::Const(0),
                        Opcode::GetLocal(0),
                        Opcode::Call(2),
                        Opcode::Call(1),
                        Opcode::SetLocal(1),
                        Opcode::GetLocal(1),
                        Opcode::Const(1),
                        Opcode::Field,
                        Opcode::SetLocal(2),
                        Opcode::Const(2),
                        Opcode::GetLocal(2),
                        Opcode::Const(3),
                        Opcode::Field,
                        Opcode::GetLocal(2),
                        Opcode::Const(4),
                        Opcode::Field,
                        Opcode::Call(2),
                        Opcode::SetLocal(3),
                        Opcode::Native(-2),
                        Opcode::Const(5),
                        Opcode::Const(6),
                        Opcode::Const(7),
                        Opcode::GetLocal(1),
                        Opcode::Const(8),
                        Opcode::Field,
                        Opcode::Const(9),
                        Opcode::GetLocal(1),
                        Opcode::Const(10),
                        Opcode::Field,
                        Opcode::Const(11),
                        Opcode::GetLocal(1),
                        Opcode::Const(12),
                        Opcode::Field,
                        Opcode::Const(13),
                        Opcode::GetLocal(3),
                        Opcode::Const(14),
                        Opcode::GetLocal(1),
                        Opcode::Const(15),
                        Opcode::Field,
                        Opcode::Map(6),
                        Opcode::Call(1),
                        Opcode::Pop,
                        Opcode::GetLocal(2),
                        Opcode::Return,
                    ],
                    4,
                    1,
                ),
            ],
            vec![Opcode::Const(16), Opcode::SetGlobal(0)],
        )];
        run_compiler_tests(tests);
    }
}
