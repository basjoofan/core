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

    fn symbol(&mut self, symbol: Symbol) -> usize {
        match symbol {
            Symbol::Global(index) => self.emit(Opcode::GetGlobal(index)),
            Symbol::Local(index, free) => {
                if free {
                    self.emit(Opcode::GetFree(index))
                } else {
                    self.emit(Opcode::GetLocal(index))
                }
            }
            Symbol::Function => self.emit(Opcode::Current),
        }
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
                match self.symbols.resolve(name.as_str()) {
                    Some(symbol) => self.symbol(symbol),
                    None => match native::get(name.as_str()) {
                        Some(index) => self.emit(Opcode::Native(index)),
                        None => Err(format!("Undefined variable: {}", name))?,
                    },
                };
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
                let symbol = self.symbols.define(name.as_str());
                let opcode = match symbol {
                    Symbol::Global(index) => Opcode::SetGlobal(*index),
                    Symbol::Local(index, _) => Opcode::SetLocal(*index),
                    Symbol::Function => Err(format!("Cannot redefine function: {}", name))?,
                };
                self.assemble(*value)?;
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
                self.assemble(Expr::Call(
                    Box::new(Expr::Function(
                        None,
                        vec![String::from("left")],
                        vec![Expr::If(
                            Box::new(Expr::Ident(String::from("left"))),
                            vec![Expr::Ident(String::from("left"))],
                            vec![*right],
                        )],
                    )),
                    vec![*left],
                ))?;
            }
            Expr::Binary(Token { kind: Kind::La, .. }, left, right) => {
                self.assemble(Expr::Call(
                    Box::new(Expr::Function(
                        None,
                        vec![String::from("left")],
                        vec![Expr::If(
                            Box::new(Expr::Ident(String::from("left"))),
                            vec![*right],
                            vec![Expr::Ident(String::from("left"))],
                        )],
                    )),
                    vec![*left],
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
            Expr::Function(name, parameters, body) => {
                self.enter();
                if let Some(name) = name {
                    self.symbols.function(name.as_str());
                }
                for parameter in parameters.iter() {
                    self.symbols.define(parameter);
                }
                self.block(body, true)?;
                let frees = self.symbols.frees();
                let count = frees.len();
                let length = self.symbols.length();
                let opcodes = self.leave();
                for free in frees {
                    self.symbol(free);
                }
                let number = parameters.len();
                let index = self.save(Value::Function(opcodes, length, number));
                self.emit(Opcode::Closure(index, count));
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
                let length = asserts.len();
                let format = Expr::Call(Box::new(Expr::Ident(String::from("format"))), places);
                let body = vec![
                    // let result = http(format(...));
                    Expr::Let(
                        String::from("result"),
                        Box::new(Expr::Call(Box::new(Expr::Ident(String::from("http"))), vec![format])),
                    ),
                    // let response = result.response;
                    Expr::Let(
                        String::from("response"),
                        Box::new(Expr::Field(Box::new(Expr::Ident(String::from("result"))), String::from("response"))),
                    ),
                    // let asserts = fn(status, version){[
                    //   fn(){
                    //     let expr = "{left} {token} {right}";
                    //     let left = left;
                    //     let right = right;
                    //     let result = left token right;
                    //   } ...
                    // ]}(response.status, response.version);
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
                                            Expr::Binary(token, left, right) => Some(Expr::Call(
                                                Box::new(Expr::Function(
                                                    None,
                                                    vec![],
                                                    vec![
                                                        Expr::Let(
                                                            String::from("expr"),
                                                            Box::new(Expr::String(format!("{left} {token} {right}"))),
                                                        ),
                                                        Expr::Let(String::from("left"), left),
                                                        Expr::Let(String::from("compare"), Box::new(Expr::String(format!("{token}")))),
                                                        Expr::Let(String::from("right"), right),
                                                        Expr::Map(vec![
                                                            (Expr::String(String::from("expr")), Expr::Ident(String::from("expr"))),
                                                            (Expr::String(String::from("left")), Expr::Ident(String::from("left"))),
                                                            (Expr::String(String::from("compare")), Expr::Ident(String::from("compare"))),
                                                            (Expr::String(String::from("right")), Expr::Ident(String::from("right"))),
                                                            (
                                                                Expr::String(String::from("result")),
                                                                Expr::Binary(
                                                                    token,
                                                                    Box::new(Expr::Ident(String::from("left"))),
                                                                    Box::new(Expr::Ident(String::from("right"))),
                                                                ),
                                                            ),
                                                        ]),
                                                    ],
                                                )),
                                                vec![],
                                            )),
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
                    // println("=== TEST  {name}", name);
                    Expr::Call(
                        Box::new(Expr::Ident(String::from("println"))),
                        vec![Expr::String(String::from("=== TEST  {name}")), Expr::String(name.to_owned())],
                    ),
                    // let log = fn(assert){ println("{expr} => {left} {compare} {right} => {result}", assert.expr, assert.left, assert.compare, assert.right, assert.result) };
                    Expr::Let(
                        String::from("log"),
                        Box::new(Expr::Function(
                            None,
                            vec![String::from("assert")],
                            vec![Expr::Call(
                                Box::new(Expr::Ident(String::from("println"))),
                                vec![
                                    Expr::String(String::from("{expr} => {left} {compare} {right} => {result}")),
                                    Expr::Field(Box::new(Expr::Ident(String::from("assert"))), String::from("expr")),
                                    Expr::Field(Box::new(Expr::Ident(String::from("assert"))), String::from("left")),
                                    Expr::Field(Box::new(Expr::Ident(String::from("assert"))), String::from("compare")),
                                    Expr::Field(Box::new(Expr::Ident(String::from("assert"))), String::from("right")),
                                    Expr::Field(Box::new(Expr::Ident(String::from("assert"))), String::from("result")),
                                ],
                            )],
                        )),
                    ),
                    // fn(){ log(asserts[i]); ... }();
                    Expr::Call(
                        Box::new(Expr::Function(
                            None,
                            vec![],
                            (0..length)
                                .map(|i| {
                                    Expr::Call(
                                        Box::new(Expr::Ident(String::from("log"))),
                                        vec![Expr::Index(
                                            Box::new(Expr::Ident(String::from("asserts"))),
                                            Box::new(Expr::Integer(i as i64)),
                                        )],
                                    )
                                })
                                .collect(),
                        )),
                        vec![],
                    ),
                    // let flag = if (true && asserts[i].result ... ) { "PASS" }else { "FAIL" };
                    Expr::Let(
                        String::from("flag"),
                        Box::new(Expr::If(
                            Box::new((0..length).fold(Expr::Boolean(true), |e, i| {
                                Expr::Binary(
                                    Token {
                                        kind: Kind::La,
                                        literal: String::from("&&"),
                                    },
                                    Box::new(e),
                                    Box::new(Expr::Field(
                                        Box::new(Expr::Index(
                                            Box::new(Expr::Ident(String::from("asserts"))),
                                            Box::new(Expr::Integer(i as i64)),
                                        )),
                                        String::from("result"),
                                    )),
                                )
                            })),
                            vec![Expr::String(String::from("PASS"))],
                            vec![Expr::String(String::from("FAIL"))],
                        )),
                    ),
                    // println("--- {flag}  {name} ({total})", flag, "name", result.time.total);
                    Expr::Call(
                        Box::new(Expr::Ident(String::from("println"))),
                        vec![
                            Expr::String(String::from("--- {flag}  {name} ({total}ms)")),
                            Expr::Ident(String::from("flag")),
                            Expr::String(name.to_owned()),
                            Expr::Binary(
                                Token {
                                    kind: Kind::Slash,
                                    literal: String::from("/"),
                                },
                                Box::new(Expr::Field(
                                    Box::new(Expr::Field(Box::new(Expr::Ident(String::from("result"))), String::from("time"))),
                                    String::from("total"),
                                )),
                                Box::new(Expr::Float(1000000.00)),
                            ),
                        ],
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
                self.emit(Opcode::Closure(index, usize::MIN));
                let symbol = self.symbols.define(name.as_str());
                let opcode = match symbol {
                    Symbol::Global(index) => Opcode::SetGlobal(*index),
                    Symbol::Local(_, _) => Err(format!("Cannot define test in local: {}", name))?,
                    Symbol::Function => Err(format!("Cannot redefine function: {}", name))?,
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
                vec![Value::Function(
                    vec![
                        Opcode::GetLocal(0),
                        Opcode::Judge(4),
                        Opcode::GetLocal(0),
                        Opcode::Jump(5),
                        Opcode::True,
                        Opcode::Return,
                    ],
                    1,
                    1,
                )],
                vec![Opcode::Closure(0, 0), Opcode::True, Opcode::Call(1), Opcode::Pop],
            ),
            (
                "true && true",
                vec![Value::Function(
                    vec![
                        Opcode::GetLocal(0),
                        Opcode::Judge(4),
                        Opcode::True,
                        Opcode::Jump(5),
                        Opcode::GetLocal(0),
                        Opcode::Return,
                    ],
                    1,
                    1,
                )],
                vec![Opcode::Closure(0, 0), Opcode::True, Opcode::Call(1), Opcode::Pop],
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
                vec![Opcode::Closure(2, 0), Opcode::Pop],
            ),
            (
                "fn() { 6 + 8 }",
                vec![
                    Value::Integer(6),
                    Value::Integer(8),
                    Value::Function(vec![Opcode::Const(0), Opcode::Const(1), Opcode::Add, Opcode::Return], 0, 0),
                ],
                vec![Opcode::Closure(2, 0), Opcode::Pop],
            ),
            (
                "fn() { 1; 2 }",
                vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Function(vec![Opcode::Const(0), Opcode::Pop, Opcode::Const(1), Opcode::Return], 0, 0),
                ],
                vec![Opcode::Closure(2, 0), Opcode::Pop],
            ),
            (
                "fn() { }",
                vec![Value::Function(vec![Opcode::None, Opcode::Return], 0, 0)],
                vec![Opcode::Closure(0, 0), Opcode::Pop],
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
                vec![Opcode::Closure(1, 0), Opcode::Call(0), Opcode::Pop],
            ),
            (
                "let no_arg = fn() { 22 }; no_arg();",
                vec![Value::Integer(22), Value::Function(vec![Opcode::Const(0), Opcode::Return], 0, 0)],
                vec![
                    Opcode::Closure(1, 0),
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
                    Opcode::Closure(0, 0),
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
                    Opcode::Closure(0, 0),
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
                vec![Opcode::Const(0), Opcode::SetGlobal(0), Opcode::Closure(1, 0), Opcode::Pop],
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
                vec![Opcode::Closure(1, 0), Opcode::Pop],
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
                vec![Opcode::Closure(2, 0), Opcode::Pop],
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
                vec![Opcode::Closure(0, 0), Opcode::Pop],
            ),
        ];
        run_compiler_tests(tests);
    }

    #[test]
    fn test_function_closure() {
        let tests = vec![
            (
                "
                fn(a) {
                    fn(b) {
                        a + b
                    }
                }
                 ",
                vec![
                    Value::Function(vec![Opcode::GetFree(0), Opcode::GetLocal(0), Opcode::Add, Opcode::Return], 1, 1),
                    Value::Function(vec![Opcode::GetLocal(0), Opcode::Closure(0, 1), Opcode::Return], 1, 1),
                ],
                vec![Opcode::Closure(1, 0), Opcode::Pop],
            ),
            (
                "
                fn(a) {
                    fn(b) {
                        fn(c) {
                            a + b + c
                        }
                    }
			    };
                ",
                vec![
                    Value::Function(
                        vec![
                            Opcode::GetFree(0),
                            Opcode::GetFree(1),
                            Opcode::Add,
                            Opcode::GetLocal(0),
                            Opcode::Add,
                            Opcode::Return,
                        ],
                        1,
                        1,
                    ),
                    Value::Function(
                        vec![Opcode::GetFree(0), Opcode::GetLocal(0), Opcode::Closure(0, 2), Opcode::Return],
                        1,
                        1,
                    ),
                    Value::Function(vec![Opcode::GetLocal(0), Opcode::Closure(1, 1), Opcode::Return], 1, 1),
                ],
                vec![Opcode::Closure(2, 0), Opcode::Pop],
            ),
            (
                "
                let global = 55;
                fn() {
                    let a = 66;
                    fn() {
                        let b = 77;
                        fn() {
                            let c = 88;
                            global + a + b + c;
                        }
                    }
                }
                ",
                vec![
                    Value::Integer(55),
                    Value::Integer(66),
                    Value::Integer(77),
                    Value::Integer(88),
                    Value::Function(
                        vec![
                            Opcode::Const(3),
                            Opcode::SetLocal(0),
                            Opcode::GetGlobal(0),
                            Opcode::GetFree(0),
                            Opcode::Add,
                            Opcode::GetFree(1),
                            Opcode::Add,
                            Opcode::GetLocal(0),
                            Opcode::Add,
                            Opcode::Return,
                        ],
                        1,
                        0,
                    ),
                    Value::Function(
                        vec![
                            Opcode::Const(2),
                            Opcode::SetLocal(0),
                            Opcode::GetFree(0),
                            Opcode::GetLocal(0),
                            Opcode::Closure(4, 2),
                            Opcode::Return,
                        ],
                        1,
                        0,
                    ),
                    Value::Function(
                        vec![
                            Opcode::Const(1),
                            Opcode::SetLocal(0),
                            Opcode::GetLocal(0),
                            Opcode::Closure(5, 1),
                            Opcode::Return,
                        ],
                        1,
                        0,
                    ),
                ],
                vec![Opcode::Const(0), Opcode::SetGlobal(0), Opcode::Closure(6, 0), Opcode::Pop],
            ),
        ];
        run_compiler_tests(tests);
    }

    #[test]
    fn test_function_recursive() {
        let tests = vec![
            (
                "
                let countDown = fn(x) { countDown(x - 1); };
                countDown(1);
                ",
                vec![
                    Value::Integer(1),
                    Value::Function(
                        vec![
                            Opcode::Current,
                            Opcode::GetLocal(0),
                            Opcode::Const(0),
                            Opcode::Sub,
                            Opcode::Call(1),
                            Opcode::Return,
                        ],
                        1,
                        1,
                    ),
                    Value::Integer(1),
                ],
                vec![
                    Opcode::Closure(1, 0),
                    Opcode::SetGlobal(0),
                    Opcode::GetGlobal(0),
                    Opcode::Const(2),
                    Opcode::Call(1),
                    Opcode::Pop,
                ],
            ),
            (
                "
                let wrapper = fn() {
                    let countDown = fn(x) { countDown(x - 1); };
                    countDown(1);
                };
                wrapper();
                ",
                vec![
                    Value::Integer(1),
                    Value::Function(
                        vec![
                            Opcode::Current,
                            Opcode::GetLocal(0),
                            Opcode::Const(0),
                            Opcode::Sub,
                            Opcode::Call(1),
                            Opcode::Return,
                        ],
                        1,
                        1,
                    ),
                    Value::Integer(1),
                    Value::Function(
                        vec![
                            Opcode::Closure(1, 0),
                            Opcode::SetLocal(0),
                            Opcode::GetLocal(0),
                            Opcode::Const(2),
                            Opcode::Call(1),
                            Opcode::Return,
                        ],
                        1,
                        0,
                    ),
                ],
                vec![
                    Opcode::Closure(3, 0),
                    Opcode::SetGlobal(0),
                    Opcode::GetGlobal(0),
                    Opcode::Call(0),
                    Opcode::Pop,
                ],
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
                Opcode::Closure(1, 0),
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
                Value::String(String::from("=== TEST  {name}")),
                Value::String(String::from("request")),
                Value::String(String::from("{expr} => {left} {compare} {right} => {result}")),
                Value::String(String::from("expr")),
                Value::String(String::from("left")),
                Value::String(String::from("compare")),
                Value::String(String::from("right")),
                Value::String(String::from("result")),
                Value::Function(
                    vec![
                        Opcode::Native(1),
                        Opcode::Const(7),
                        Opcode::GetLocal(0),
                        Opcode::Const(8),
                        Opcode::Field,
                        Opcode::GetLocal(0),
                        Opcode::Const(9),
                        Opcode::Field,
                        Opcode::GetLocal(0),
                        Opcode::Const(10),
                        Opcode::Field,
                        Opcode::GetLocal(0),
                        Opcode::Const(11),
                        Opcode::Field,
                        Opcode::GetLocal(0),
                        Opcode::Const(12),
                        Opcode::Field,
                        Opcode::Call(6),
                        Opcode::Return,
                    ],
                    1,
                    1,
                ),
                Value::Function(vec![Opcode::None, Opcode::Return], 0, 0),
                Value::String(String::from("PASS")),
                Value::String(String::from("FAIL")),
                Value::String(String::from("--- {flag}  {name} ({total}ms)")),
                Value::String(String::from("request")),
                Value::String(String::from("time")),
                Value::String(String::from("total")),
                Value::Float(1000000.00),
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
                        Opcode::Closure(2, 0),
                        Opcode::GetLocal(2),
                        Opcode::Const(3),
                        Opcode::Field,
                        Opcode::GetLocal(2),
                        Opcode::Const(4),
                        Opcode::Field,
                        Opcode::Call(2),
                        Opcode::SetLocal(3),
                        Opcode::Native(1),
                        Opcode::Const(5),
                        Opcode::Const(6),
                        Opcode::Call(2),
                        Opcode::Pop,
                        Opcode::Closure(13, 0),
                        Opcode::SetLocal(4),
                        Opcode::Closure(14, 0),
                        Opcode::Call(0),
                        Opcode::Pop,
                        Opcode::True,
                        Opcode::Judge(34),
                        Opcode::Const(15),
                        Opcode::Jump(35),
                        Opcode::Const(16),
                        Opcode::SetLocal(5),
                        Opcode::Native(1),
                        Opcode::Const(17),
                        Opcode::GetLocal(5),
                        Opcode::Const(18),
                        Opcode::GetLocal(1),
                        Opcode::Const(19),
                        Opcode::Field,
                        Opcode::Const(20),
                        Opcode::Field,
                        Opcode::Const(21),
                        Opcode::Div,
                        Opcode::Call(4),
                        Opcode::Pop,
                        Opcode::GetLocal(2),
                        Opcode::Return,
                    ],
                    6,
                    1,
                ),
            ],
            vec![Opcode::Closure(22, 0), Opcode::SetGlobal(0)],
        )];
        run_compiler_tests(tests);
    }
}
