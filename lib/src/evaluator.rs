use crate::http;
use crate::native;
use crate::Assert;
use crate::Context;
use crate::Expr;
use crate::Kind;
use crate::Record;
use crate::Token;
use crate::Value;
use std::collections::HashMap;

async fn eval_expr(expr: &Expr, context: &mut Context) -> Result<Value, String> {
    match expr {
        Expr::Integer(integer) => eval_integer_literal(integer),
        Expr::Float(float) => eval_float_literal(float),
        Expr::Boolean(boolean) => eval_boolean_literal(boolean),
        Expr::String(string) => eval_string_literal(string),
        Expr::Array(items) => Box::pin(eval_array_literal(items, context)).await,
        Expr::Map(pairs) => Box::pin(eval_map_literal(pairs, context)).await,
        Expr::Index(value, index) => Box::pin(eval_index_expr(value, index, context)).await,
        Expr::Field(map, field) => Box::pin(eval_field_expr(map, field, context)).await,
        Expr::Ident(ident) => eval_ident_expr(ident, context),
        Expr::Let(name, expr) => Box::pin(eval_let_expr(name, expr, context)).await,
        Expr::Unary(token, right) => Box::pin(eval_unary_expr(token, right, context)).await,
        Expr::Binary(token, left, right) => Box::pin(eval_binary_expr(token, left, right, context)).await,
        Expr::Paren(expr) => Box::pin(eval_expr(expr, context)).await,
        Expr::If(condition, consequence, alternative) => Box::pin(eval_if_expr(condition, consequence, alternative, context)).await,
        Expr::Call(name, arguments) => Box::pin(eval_call_expr(name, arguments, context)).await,
    }
}

fn eval_integer_literal(integer: &i64) -> Result<Value, String> {
    Ok(Value::Integer(*integer))
}

fn eval_float_literal(float: &f64) -> Result<Value, String> {
    Ok(Value::Float(*float))
}

fn eval_boolean_literal(boolean: &bool) -> Result<Value, String> {
    Ok(Value::Boolean(*boolean))
}

fn eval_string_literal(string: &String) -> Result<Value, String> {
    Ok(Value::String(string.to_owned()))
}

async fn eval_array_literal(items: &[Expr], context: &mut Context) -> Result<Value, String> {
    Ok(Value::Array(eval_list(items, context).await?))
}

async fn eval_map_literal(pairs: &Vec<(Expr, Expr)>, context: &mut Context) -> Result<Value, String> {
    let mut map = HashMap::new();
    for (key, value) in pairs {
        let key = eval_expr(key, context).await?;
        let value = eval_expr(value, context).await?;
        map.insert(key.to_string(), value);
    }
    Ok(Value::Map(map))
}

async fn eval_index_expr(value: &Expr, index: &Expr, context: &mut Context) -> Result<Value, String> {
    // TODO enhance indent expr get variable use reference
    let value = eval_expr(value, context).await?;
    let index = eval_expr(index, context).await?;
    match (value, index) {
        (Value::Array(mut items), Value::Integer(index)) => {
            let index = index as usize;
            if index < items.len() {
                let item = items.remove(index);
                Ok(item)
            } else {
                Ok(Value::Null)
            }
        }
        (Value::Map(mut pairs), key) => {
            let element = pairs.remove(&key.to_string());
            match element {
                Some(element) => Ok(element),
                None => Ok(Value::Null),
            }
        }
        (value, _) => Err(format!("index operator not support: {:?}", value)),
    }
}

async fn eval_field_expr(map: &Expr, field: &String, context: &mut Context) -> Result<Value, String> {
    // TODO enhance indent expr get variable use reference
    match eval_expr(map, context).await? {
        Value::Map(mut pairs) => {
            let value = pairs.remove(field);
            Ok(match value {
                Some(value) => value,
                None => Value::Null,
            })
        }
        map => Err(format!("field operator not support: {:?}", map)),
    }
}

fn eval_ident_expr(ident: &String, context: &mut Context) -> Result<Value, String> {
    match context.get(ident) {
        Some(value) => Ok(value.to_owned()),
        None => Err(format!("ident:{} not found", ident)),
    }
}

async fn eval_let_expr(name: &String, expr: &Expr, context: &mut Context) -> Result<Value, String> {
    let value = eval_expr(expr, context).await?;
    context.set(name.to_owned(), value.to_owned());
    Ok(value)
}

async fn eval_unary_expr(token: &Token, right: &Expr, context: &mut Context) -> Result<Value, String> {
    let right = eval_expr(right, context).await?;
    match (token.kind, right) {
        (Kind::Not, Value::Boolean(false)) | (Kind::Not, Value::Null) => Ok(Value::Boolean(true)),
        (Kind::Not, Value::Integer(integer)) => Ok(Value::Integer(!integer)),
        (Kind::Not, _) => Ok(Value::Boolean(false)),
        (Kind::Sub, Value::Integer(integer)) => Ok(Value::Integer(-integer)),
        (Kind::Sub, Value::Float(float)) => Ok(Value::Float(-float)),
        (_, right) => Err(format!("unknown operator: {}{:?}", token, right)),
    }
}

async fn eval_binary_expr(token: &Token, left: &Expr, right: &Expr, context: &mut Context) -> Result<Value, String> {
    match token.kind {
        Kind::Add => eval_expr(left, context).await? + eval_expr(right, context).await?,
        Kind::Sub => eval_expr(left, context).await? - eval_expr(right, context).await?,
        Kind::Mul => eval_expr(left, context).await? * eval_expr(right, context).await?,
        Kind::Div => eval_expr(left, context).await? / eval_expr(right, context).await?,
        Kind::Rem => eval_expr(left, context).await? % eval_expr(right, context).await?,
        Kind::Bx => eval_expr(left, context).await? ^ eval_expr(right, context).await?,
        Kind::Bo => eval_expr(left, context).await? | eval_expr(right, context).await?,
        Kind::Ba => eval_expr(left, context).await? & eval_expr(right, context).await?,
        Kind::Sl => eval_expr(left, context).await? << eval_expr(right, context).await?,
        Kind::Sr => eval_expr(left, context).await? >> eval_expr(right, context).await?,
        Kind::Lo => match eval_expr(left, context).await? {
            Value::Boolean(false) | Value::Null => eval_expr(right, context).await,
            left => Ok(left),
        },
        Kind::La => match eval_expr(left, context).await? {
            left @ (Value::Boolean(false) | Value::Null) => Ok(left),
            _ => eval_expr(right, context).await,
        },
        Kind::Lt => Ok(Value::Boolean(eval_expr(left, context).await? < eval_expr(right, context).await?)),
        Kind::Gt => Ok(Value::Boolean(eval_expr(left, context).await? > eval_expr(right, context).await?)),
        Kind::Le => Ok(Value::Boolean(eval_expr(left, context).await? <= eval_expr(right, context).await?)),
        Kind::Ge => Ok(Value::Boolean(eval_expr(left, context).await? >= eval_expr(right, context).await?)),
        Kind::Eq => Ok(Value::Boolean(eval_expr(left, context).await? == eval_expr(right, context).await?)),
        Kind::Ne => Ok(Value::Boolean(eval_expr(left, context).await? != eval_expr(right, context).await?)),
        _ => Err(format!("not support operator: {} {} {}", left, token, right)),
    }
}

async fn eval_if_expr(condition: &Expr, consequence: &[Expr], alternative: &[Expr], context: &mut Context) -> Result<Value, String> {
    let condition = eval_expr(condition, context).await?;
    match condition {
        Value::Boolean(false) | Value::Null => eval_block(alternative, context).await,
        _ => eval_block(consequence, context).await,
    }
}

async fn eval_call_expr(name: &str, arguments: &[Expr], context: &mut Context) -> Result<Value, String> {
    let arguments = eval_list(arguments, context).await?;
    match context.request(name) {
        Some((message, exprs)) => {
            let name = name.to_string();
            let regex = regex::Regex::new(r"\{\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*\}").unwrap();
            let matches = regex.find_iter(message);
            let mut ranges = Vec::new();
            matches.for_each(|m| ranges.push((m.as_str()[1..m.as_str().len() - 1].trim(), m.range())));
            ranges.reverse();
            let mut message = message.to_string();
            for (variable, range) in ranges.into_iter() {
                let variable = match context.get(variable) {
                    Some(variable) => variable.to_string(),
                    None => Value::Null.to_string(),
                };
                message.replace_range(range, variable.as_str());
            }
            let client = http::Client::default();
            let (request, response, time, error) = client.send(message.as_str()).await;
            let map = response.to_map();
            let mut local = Context::from(map);
            let mut asserts = Vec::new();
            for assert in exprs {
                if let Expr::Binary(token, left, right) = assert {
                    let expr = format!("{} {} {}", left, token, right);
                    let left = eval_expr(left, &mut local).await.unwrap_or(Value::Null);
                    let right = eval_expr(right, &mut local).await.unwrap_or(Value::Null);
                    if let Some(result) = match token.kind {
                        Kind::Lt => Some(left < right),
                        Kind::Gt => Some(left > right),
                        Kind::Le => Some(left <= right),
                        Kind::Ge => Some(left >= right),
                        Kind::Eq => Some(left == right),
                        Kind::Ne => Some(left != right),
                        _ => None,
                    } {
                        asserts.push(Assert {
                            expr,
                            left: left.to_string(),
                            compare: token.to_string(),
                            right: right.to_string(),
                            result,
                        });
                    };
                }
            }
            context.push(Record {
                name,
                request,
                response,
                time,
                error,
                asserts,
            });
            Ok(Value::Map(local.into_map()))
        }
        None => match name {
            "println" => Ok(native::println(arguments)?),
            "print" => Ok(native::print(arguments)?),
            "format" => Ok(native::format(arguments)?),
            "length" => Ok(native::length(arguments)?),
            "append" => Ok(native::append(arguments)?),
            _ => Err(format!("function {} not found", name)),
        },
    }
}

pub async fn eval_block(exprs: &[Expr], context: &mut Context) -> Result<Value, String> {
    let mut result = Value::Null;
    for expr in exprs {
        result = eval_expr(expr, context).await?;
    }
    Ok(result)
}

async fn eval_list(items: &[Expr], context: &mut Context) -> Result<Vec<Value>, String> {
    let mut values = Vec::with_capacity(items.len());
    for item in items {
        values.push(eval_expr(item, context).await?);
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::eval_block;
    use crate::parser::Parser;
    use crate::parser::Source;
    use crate::Context;
    use crate::Value;
    use std::collections::HashMap;

    async fn run_eval_tests(tests: Vec<(&str, Value)>) {
        for (text, expect) in tests {
            let Source { exprs, requests, .. } = Parser::new(text).parse().unwrap();
            let mut context = Context::new();
            context.extend(requests);
            match eval_block(&exprs, &mut context).await {
                Ok(value) => {
                    println!("{:?} => {} = {}", exprs, value, expect);
                    assert_eq!(value, expect);
                }
                Err(message) => panic!("machine error: {}", message),
            }
        }
    }

    #[tokio::test]
    async fn test_integer_arithmetic() {
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
            ("!!5", Value::Integer(5)),
            ("!-3", Value::Integer(2)),
            ("5 ^ 3", Value::Integer(6)),
            ("5 | 3", Value::Integer(7)),
            ("5 & 3", Value::Integer(1)),
            ("5 << 2", Value::Integer(20)),
            ("5 >> 2", Value::Integer(1)),
            ("-5 >> 2", Value::Integer(-2)),
        ];
        run_eval_tests(tests).await;
    }

    #[tokio::test]
    async fn test_float_arithmetic() {
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
        run_eval_tests(tests).await;
    }

    #[tokio::test]
    async fn test_boolean_arithmetic() {
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
            ("!!true", Value::Boolean(true)),
            ("!!false", Value::Boolean(false)),
            ("!(if (false) { 5; })", Value::Boolean(true)),
        ];
        run_eval_tests(tests).await;
    }

    #[tokio::test]
    async fn test_string_literal() {
        let tests = vec![
            (r#""hello world""#, Value::String(String::from("hello world"))),
            (r#""hello" + " world""#, Value::String(String::from("hello world"))),
            (r#""hello"+" world"+"!""#, Value::String(String::from("hello world!"))),
        ];
        run_eval_tests(tests).await;
    }

    #[tokio::test]
    async fn test_logical_expr() {
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
        run_eval_tests(tests).await;
    }

    #[tokio::test]
    async fn test_array_literal() {
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
        run_eval_tests(tests).await;
    }

    #[tokio::test]
    async fn test_map_literal() {
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
        run_eval_tests(tests).await;
    }

    #[tokio::test]
    async fn test_index_expr() {
        let tests = vec![
            ("[1, 2, 3][1]", Value::Integer(2)),
            ("[1, 2, 3][0 + 2]", Value::Integer(3)),
            ("[[1, 1, 1]][0][0]", Value::Integer(1)),
            ("[][0]", Value::Null),
            ("[1, 2, 3][99]", Value::Null),
            ("[1][-1]", Value::Null),
            ("{1: 1, 2: 2}[1]", Value::Integer(1)),
            ("{1: 1, 2: 2}[2]", Value::Integer(2)),
            ("{1: 1}[0]", Value::Null),
            ("{}[0]", Value::Null),
        ];
        run_eval_tests(tests).await;
    }

    #[tokio::test]
    async fn test_field_expr() {
        let tests = vec![("{\"a\": 2}.a", Value::Integer(2))];
        run_eval_tests(tests).await;
    }

    #[tokio::test]
    async fn test_let_expr() {
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
        run_eval_tests(tests).await;
    }

    #[tokio::test]
    async fn test_if_expr() {
        let tests = vec![
            ("if (true) { 10 }", Value::Integer(10)),
            ("if (true) { 10 } else { 20 }", Value::Integer(10)),
            ("if (false) { 10 } else { 20 } ", Value::Integer(20)),
            ("if (1) { 10 }", Value::Integer(10)),
            ("if (1 < 2) { 10 }", Value::Integer(10)),
            ("if (1 < 2) { 10 } else { 20 }", Value::Integer(10)),
            ("if (1 > 2) { 10 } else { 20 }", Value::Integer(20)),
            ("if (1 > 2) { 10 }", Value::Null),
            ("if (false) { 10 }", Value::Null),
            ("if ((if (false) { 10 })) { 10 } else { 20 }", Value::Integer(20)),
            ("if (true) {} else { 10 }", Value::Null),
            ("if (true) { 1; 2 } else { 3 }", Value::Integer(2)),
        ];
        run_eval_tests(tests).await;
    }

    #[tokio::test]
    async fn test_call_native() {
        let tests = vec![
            ("length(\"\")", Value::Integer(0)),
            ("length(\"two\")", Value::Integer(3)),
            ("length(\"hello world\")", Value::Integer(11)),
            ("length([])", Value::Integer(0)),
            ("length([1, 2, 3])", Value::Integer(3)),
        ];
        run_eval_tests(tests).await;
    }

    #[tokio::test]
    async fn test_request_literal() {
        crate::tests::start_server(30006).await;
        let tests = vec![(
            r#"
            request get`
                GET http://{host}:30006/get
                Host: {{host}}
                Connection: close
            `[];
            let host = "127.0.0.1";
            let response = get();
            response.status
            "#,
            Value::Integer(200),
        )];
        run_eval_tests(tests).await;
    }

    #[tokio::test]
    async fn test_request_asserts() {
        crate::tests::start_server(30007).await;
        let tests = vec![(
            r#"
            request get`
                GET http://{host}:30007/get
                Host: {host}
                Connection: close
            `[status == 200];
            let host = "127.0.0.1";
            let response = get();
            response.status
            "#,
            Value::Integer(200),
        )];
        run_eval_tests(tests).await;
    }
}
