use super::Assert;
use super::Context;
use super::Expr;
use super::Kind;
use super::Parser;
use super::Record;
use super::Source;
use super::Token;
use super::Value;
use super::http;
use super::native;
use std::collections::HashMap;

enum EvalFlow {
    Value(Value),
    Break { label: Option<String>, value: Value },
    Continue { label: Option<String> },
}

impl Source {
    async fn eval_expr(&self, expr: &Expr, context: &mut Context) -> Result<Value, String> {
        match Box::pin(self.eval_flow(expr, context)).await? {
            EvalFlow::Value(value) => Ok(value),
            EvalFlow::Break { .. } => Err("break outside loop".to_string()),
            EvalFlow::Continue { .. } => Err("continue outside loop".to_string()),
        }
    }

    async fn eval_flow(&self, expr: &Expr, context: &mut Context) -> Result<EvalFlow, String> {
        match expr {
            Expr::Integer(integer) => Ok(EvalFlow::Value(self.eval_integer_literal(integer)?)),
            Expr::Float(float) => Ok(EvalFlow::Value(self.eval_float_literal(float)?)),
            Expr::Boolean(boolean) => Ok(EvalFlow::Value(self.eval_boolean_literal(boolean)?)),
            Expr::String(string) => Ok(EvalFlow::Value(self.eval_string_literal(string)?)),
            Expr::Array(items) => Ok(EvalFlow::Value(
                Box::pin(self.eval_array_literal(items, context)).await?,
            )),
            Expr::Map(pairs) => Ok(EvalFlow::Value(
                Box::pin(self.eval_map_literal(pairs, context)).await?,
            )),
            Expr::Index(value, index) => Ok(EvalFlow::Value(
                Box::pin(self.eval_index_expr(value, index, context)).await?,
            )),
            Expr::Field(map, field) => Ok(EvalFlow::Value(
                Box::pin(self.eval_field_expr(map, field, context)).await?,
            )),
            Expr::Ident(ident) => Ok(EvalFlow::Value(self.eval_ident_expr(ident, context)?)),
            Expr::Let(name, expr) => Ok(EvalFlow::Value(
                Box::pin(self.eval_let_expr(name, expr, context)).await?,
            )),
            Expr::Unary(token, right) => Ok(EvalFlow::Value(
                Box::pin(self.eval_unary_expr(token, right, context)).await?,
            )),
            Expr::Binary(token, left, right) => Ok(EvalFlow::Value(
                Box::pin(self.eval_binary_expr(token, left, right, context)).await?,
            )),
            Expr::Paren(expr) => Box::pin(self.eval_flow(expr, context)).await,
            Expr::If(condition, consequence, alternative) => {
                Box::pin(self.eval_if_expr(condition, consequence, alternative, context)).await
            }
            Expr::Function(_, _, _) => Ok(EvalFlow::Value(Value::Null)),
            Expr::Call(name, arguments) => Ok(EvalFlow::Value(
                Box::pin(self.eval_call_expr(name, arguments, context)).await?,
            )),
            Expr::Break(label, value) => {
                let value = match value {
                    Some(value) => Box::pin(self.eval_expr(value, context)).await?,
                    None => Value::Null,
                };
                Ok(EvalFlow::Break {
                    label: label.to_owned(),
                    value,
                })
            }
            Expr::Continue(label) => Ok(EvalFlow::Continue {
                label: label.to_owned(),
            }),
            Expr::Loop(label, body) => Box::pin(self.eval_loop_expr(label, body, context)).await,
            Expr::While(label, condition, body) => {
                Box::pin(self.eval_while_expr(label, condition, body, context)).await
            }
            Expr::For(label, binding, iterator, body) => {
                Box::pin(self.eval_for_expr(label, binding, iterator, body, context)).await
            }
            Expr::Range(start, end, inclusive) => Ok(EvalFlow::Value(
                Box::pin(self.eval_range_expr(start, end, *inclusive, context)).await?,
            )),
        }
    }

    fn eval_integer_literal(&self, integer: &i64) -> Result<Value, String> {
        Ok(Value::Integer(*integer))
    }

    fn eval_float_literal(&self, float: &f64) -> Result<Value, String> {
        Ok(Value::Float(*float))
    }

    fn eval_boolean_literal(&self, boolean: &bool) -> Result<Value, String> {
        Ok(Value::Boolean(*boolean))
    }

    fn eval_string_literal(&self, string: &String) -> Result<Value, String> {
        Ok(Value::String(string.to_owned()))
    }

    async fn eval_array_literal(
        &self,
        items: &[Expr],
        context: &mut Context,
    ) -> Result<Value, String> {
        Ok(Value::Array(self.eval_list(items, context).await?))
    }

    async fn eval_map_literal(
        &self,
        pairs: &Vec<(Expr, Expr)>,
        context: &mut Context,
    ) -> Result<Value, String> {
        let mut map = HashMap::new();
        for (key, value) in pairs {
            let key = self.eval_expr(key, context).await?;
            let value = self.eval_expr(value, context).await?;
            map.insert(key.to_string(), value);
        }
        Ok(Value::Map(map))
    }

    async fn eval_index_expr(
        &self,
        value: &Expr,
        index: &Expr,
        context: &mut Context,
    ) -> Result<Value, String> {
        // TODO enhance indent expr get variable use reference
        let value = self.eval_expr(value, context).await?;
        let index = self.eval_expr(index, context).await?;
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
            (value, _) => Err(format!("index operator not support: {value:?}")),
        }
    }

    async fn eval_field_expr(
        &self,
        map: &Expr,
        field: &String,
        context: &mut Context,
    ) -> Result<Value, String> {
        // TODO enhance indent expr get variable use reference
        match self.eval_expr(map, context).await? {
            Value::Map(mut pairs) => {
                let value = pairs.remove(field);
                Ok(match value {
                    Some(value) => value,
                    None => Value::Null,
                })
            }
            map => Err(format!("field operator not support: {map:?}")),
        }
    }

    fn eval_ident_expr(&self, ident: &String, context: &mut Context) -> Result<Value, String> {
        match context.get(ident) {
            Some(value) => Ok(value.to_owned()),
            None => Err(format!("ident: {ident} not found")),
        }
    }

    async fn eval_let_expr(
        &self,
        name: &String,
        expr: &Expr,
        context: &mut Context,
    ) -> Result<Value, String> {
        let value = self.eval_expr(expr, context).await?;
        context.set(name.to_owned(), value.to_owned());
        Ok(value)
    }

    async fn eval_unary_expr(
        &self,
        token: &Token,
        right: &Expr,
        context: &mut Context,
    ) -> Result<Value, String> {
        let right = self.eval_expr(right, context).await?;
        match (token.kind, right) {
            (Kind::Not, Value::Boolean(false)) | (Kind::Not, Value::Null) => {
                Ok(Value::Boolean(true))
            }
            (Kind::Not, Value::Integer(integer)) => Ok(Value::Integer(!integer)),
            (Kind::Not, _) => Ok(Value::Boolean(false)),
            (Kind::Sub, Value::Integer(integer)) => Ok(Value::Integer(-integer)),
            (Kind::Sub, Value::Float(float)) => Ok(Value::Float(-float)),
            (_, right) => Err(format!("unknown operator: {token}{right:?}")),
        }
    }

    async fn eval_binary_expr(
        &self,
        token: &Token,
        left: &Expr,
        right: &Expr,
        context: &mut Context,
    ) -> Result<Value, String> {
        match token.kind {
            Kind::Add => {
                self.eval_expr(left, context).await? + self.eval_expr(right, context).await?
            }
            Kind::Sub => {
                self.eval_expr(left, context).await? - self.eval_expr(right, context).await?
            }
            Kind::Mul => {
                self.eval_expr(left, context).await? * self.eval_expr(right, context).await?
            }
            Kind::Div => {
                self.eval_expr(left, context).await? / self.eval_expr(right, context).await?
            }
            Kind::Rem => {
                self.eval_expr(left, context).await? % self.eval_expr(right, context).await?
            }
            Kind::Bx => {
                self.eval_expr(left, context).await? ^ self.eval_expr(right, context).await?
            }
            Kind::Bo => {
                self.eval_expr(left, context).await? | self.eval_expr(right, context).await?
            }
            Kind::Ba => {
                self.eval_expr(left, context).await? & self.eval_expr(right, context).await?
            }
            Kind::Sl => {
                self.eval_expr(left, context).await? << self.eval_expr(right, context).await?
            }
            Kind::Sr => {
                self.eval_expr(left, context).await? >> self.eval_expr(right, context).await?
            }
            Kind::Lo => match self.eval_expr(left, context).await? {
                Value::Boolean(false) | Value::Null => self.eval_expr(right, context).await,
                left => Ok(left),
            },
            Kind::La => match self.eval_expr(left, context).await? {
                left @ (Value::Boolean(false) | Value::Null) => Ok(left),
                _ => self.eval_expr(right, context).await,
            },
            Kind::Lt => Ok(Value::Boolean(
                self.eval_expr(left, context).await? < self.eval_expr(right, context).await?,
            )),
            Kind::Gt => Ok(Value::Boolean(
                self.eval_expr(left, context).await? > self.eval_expr(right, context).await?,
            )),
            Kind::Le => Ok(Value::Boolean(
                self.eval_expr(left, context).await? <= self.eval_expr(right, context).await?,
            )),
            Kind::Ge => Ok(Value::Boolean(
                self.eval_expr(left, context).await? >= self.eval_expr(right, context).await?,
            )),
            Kind::Eq => Ok(Value::Boolean(
                self.eval_expr(left, context).await? == self.eval_expr(right, context).await?,
            )),
            Kind::Ne => Ok(Value::Boolean(
                self.eval_expr(left, context).await? != self.eval_expr(right, context).await?,
            )),
            _ => Err(format!("not support operator: {left} {token} {right}")),
        }
    }

    async fn eval_if_expr(
        &self,
        condition: &Expr,
        consequence: &[Expr],
        alternative: &[Expr],
        context: &mut Context,
    ) -> Result<EvalFlow, String> {
        let condition = self.eval_expr(condition, context).await?;
        match condition {
            Value::Boolean(false) | Value::Null => self.eval_block_flow(alternative, context).await,
            _ => self.eval_block_flow(consequence, context).await,
        }
    }

    async fn eval_range_expr(
        &self,
        start: &Option<Box<Expr>>,
        end: &Option<Box<Expr>>,
        inclusive: bool,
        context: &mut Context,
    ) -> Result<Value, String> {
        let start = match start {
            Some(start) => Some(self.eval_range_endpoint(start, context).await?),
            None => None,
        };
        let end = match end {
            Some(end) => Some(self.eval_range_endpoint(end, context).await?),
            None => None,
        };
        Ok(Value::Range {
            start,
            end,
            inclusive,
        })
    }

    async fn eval_range_endpoint(&self, expr: &Expr, context: &mut Context) -> Result<i64, String> {
        match self.eval_expr(expr, context).await? {
            Value::Integer(integer) => Ok(integer),
            value => Err(format!("range endpoint must be integer: {value:?}")),
        }
    }

    async fn eval_loop_expr(
        &self,
        label: &Option<String>,
        body: &[Expr],
        context: &mut Context,
    ) -> Result<EvalFlow, String> {
        loop {
            match self.eval_block_flow(body, context).await? {
                EvalFlow::Value(_) => {}
                EvalFlow::Break {
                    label: target,
                    value,
                } => {
                    if target.is_none() || target == *label {
                        return Ok(EvalFlow::Value(value));
                    }
                    return Ok(EvalFlow::Break {
                        label: target,
                        value,
                    });
                }
                EvalFlow::Continue { label: target } => {
                    if target.is_none() || target == *label {
                        continue;
                    }
                    return Ok(EvalFlow::Continue { label: target });
                }
            }
        }
    }

    async fn eval_while_expr(
        &self,
        label: &Option<String>,
        condition: &Expr,
        body: &[Expr],
        context: &mut Context,
    ) -> Result<EvalFlow, String> {
        let mut result = Value::Null;
        loop {
            match self.eval_expr(condition, context).await? {
                Value::Boolean(false) | Value::Null => return Ok(EvalFlow::Value(result)),
                _ => {}
            }
            match self.eval_block_flow(body, context).await? {
                EvalFlow::Value(value) => result = value,
                EvalFlow::Break {
                    label: target,
                    value,
                } => {
                    if target.is_none() || target == *label {
                        return Ok(EvalFlow::Value(value));
                    }
                    return Ok(EvalFlow::Break {
                        label: target,
                        value,
                    });
                }
                EvalFlow::Continue { label: target } => {
                    if target.is_none() || target == *label {
                        continue;
                    }
                    return Ok(EvalFlow::Continue { label: target });
                }
            }
        }
    }

    async fn eval_for_expr(
        &self,
        label: &Option<String>,
        binding: &String,
        iterator: &Expr,
        body: &[Expr],
        context: &mut Context,
    ) -> Result<EvalFlow, String> {
        let iterator = self.eval_expr(iterator, context).await?;
        let mut result = Value::Null;
        match iterator {
            Value::Array(items) => {
                for item in items {
                    if let Some(flow) = Box::pin(self.eval_for_iteration(
                        label,
                        binding,
                        item,
                        body,
                        context,
                        &mut result,
                    ))
                    .await?
                    {
                        return Ok(flow);
                    }
                }
            }
            Value::Range {
                start: Some(start),
                end: Some(end),
                inclusive,
            } => {
                if inclusive {
                    for integer in start..=end {
                        if let Some(flow) = Box::pin(self.eval_for_iteration(
                            label,
                            binding,
                            Value::Integer(integer),
                            body,
                            context,
                            &mut result,
                        ))
                        .await?
                        {
                            return Ok(flow);
                        }
                    }
                } else {
                    for integer in start..end {
                        if let Some(flow) = Box::pin(self.eval_for_iteration(
                            label,
                            binding,
                            Value::Integer(integer),
                            body,
                            context,
                            &mut result,
                        ))
                        .await?
                        {
                            return Ok(flow);
                        }
                    }
                }
            }
            Value::Range { .. } => {
                return Err("open-ended range cannot be used in for loop".to_string());
            }
            value => return Err(format!("for loop iterator not supported: {value:?}")),
        }
        Ok(EvalFlow::Value(result))
    }

    async fn eval_for_iteration(
        &self,
        label: &Option<String>,
        binding: &String,
        item: Value,
        body: &[Expr],
        context: &mut Context,
        result: &mut Value,
    ) -> Result<Option<EvalFlow>, String> {
        context.set(binding.to_owned(), item);
        match self.eval_block_flow(body, context).await? {
            EvalFlow::Value(value) => {
                *result = value;
                Ok(None)
            }
            EvalFlow::Break {
                label: target,
                value,
            } => {
                if target.is_none() || target == *label {
                    Ok(Some(EvalFlow::Value(value)))
                } else {
                    Ok(Some(EvalFlow::Break {
                        label: target,
                        value,
                    }))
                }
            }
            EvalFlow::Continue { label: target } => {
                if target.is_none() || target == *label {
                    Ok(None)
                } else {
                    Ok(Some(EvalFlow::Continue { label: target }))
                }
            }
        }
    }

    async fn eval_call_expr(
        &self,
        function: &Expr,
        arguments: &[Expr],
        context: &mut Context,
    ) -> Result<Value, String> {
        if let Expr::Field(left, request_name) = function
            && let Expr::Ident(client_name) = left.as_ref()
        {
            if !arguments.is_empty() {
                return Err("client request calls do not accept arguments".to_string());
            }
            return Box::pin(self.eval_client_request_call(client_name, request_name, context))
                .await;
        }
        let Expr::Ident(name) = function else {
            return Err(format!("function {function} not found"));
        };
        let arguments = self.eval_list(arguments, context).await?;
        match self.function(name) {
            Some((params, body)) => {
                let variables = params
                    .iter()
                    .zip(arguments)
                    .map(|(p, a)| (p.to_owned(), a))
                    .collect::<HashMap<String, Value>>();
                let mut local = Context::from(variables);
                self.eval_block(body, &mut local).await
            }
            None => match name.as_str() {
                "println" => Ok(native::println(arguments, context)?),
                "print" => Ok(native::print(arguments, context)?),
                "format" => Ok(native::format(arguments, context)?),
                "length" => Ok(native::length(arguments)?),
                "append" => Ok(native::append(arguments)?),
                _ => Err(format!("function {name} not found")),
            },
        }
    }

    async fn eval_client_request_call(
        &self,
        client_name: &str,
        request_name: &str,
        context: &mut Context,
    ) -> Result<Value, String> {
        match self.clients.get(client_name).and_then(|client| {
            client
                .request(request_name)
                .map(|request| (client, request))
        }) {
            Some((client, request_definition)) => {
                let name = format!("{client_name}.{request_name}");
                let message = self
                    .eval_request_message(client, request_definition, context)
                    .await?;
                let client = http::Client::new();
                let (request, response, time, error) = client.send(message.as_str()).await;
                let variables = response.to_map();
                let mut local = Context::from(variables);
                let mut asserts = Vec::new();
                for assert in &request_definition.asserts {
                    if let Expr::Binary(token, left, right) = assert {
                        let expr = format!("{left} {token} {right}");
                        let left = self
                            .eval_expr(left, &mut local)
                            .await
                            .unwrap_or(Value::Null);
                        let right = self
                            .eval_expr(right, &mut local)
                            .await
                            .unwrap_or(Value::Null);
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
            None => Err(format!("request {client_name}.{request_name} not found")),
        }
    }

    async fn eval_request_message(
        &self,
        client: &crate::client::Client,
        request: &crate::client::Request,
        context: &mut Context,
    ) -> Result<String, String> {
        let host = self
            .eval_request_string(&client.host, context, "host")
            .await?;
        let path = self
            .eval_request_string(&request.path, context, "path")
            .await?;
        let mut url = format!("{}://{host}", client.scheme.as_ref());
        if let Some(port) = client.port {
            url.push_str(&format!(":{port}"));
        }
        url.push_str(&path);

        if !request.params.is_empty() {
            let mut serializer = http::Serializer::new();
            for (key, value) in &request.params {
                let key = self.eval_request_scalar(key, context, "param key").await?;
                let value = self
                    .eval_request_scalar(value, context, "param value")
                    .await?;
                serializer.append(&key, &value);
            }
            url.push('?');
            url.push_str(&String::from_utf8_lossy(&serializer.finish()));
        }

        let mut headers = Vec::new();
        for (key, value) in &request.headers {
            headers.push((
                self.eval_request_scalar(key, context, "header name")
                    .await?,
                self.eval_request_scalar(value, context, "header value")
                    .await?,
            ));
        }
        let content_type = headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
            .map(|(_, value)| value.to_owned());

        let mut body = None;
        if let Some(expr) = &request.body {
            let value = self.eval_request_value(expr, context).await?;
            let media_type = content_type
                .as_deref()
                .and_then(|value| value.split(';').next())
                .map(str::trim)
                .map(str::to_ascii_lowercase);
            body = Some(match media_type.as_deref() {
                Some("application/x-www-form-urlencoded") | Some("multipart/form-data") => {
                    request_pairs(&value)?
                        .into_iter()
                        .map(|(key, value)| format!("{key}: {value}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                }
                Some(media_type)
                    if media_type == "application/json" || media_type.ends_with("+json") =>
                {
                    match value {
                        Value::String(value) => value,
                        value => value.to_json()?,
                    }
                }
                None => match value {
                    Value::String(value) => value,
                    value @ (Value::Map(_) | Value::Array(_)) => {
                        headers.push(("Content-Type".to_string(), "application/json".to_string()));
                        value.to_json()?
                    }
                    value => {
                        return Err(format!(
                            "body without Content-Type must be a string, object, or array, found {value:?}"
                        ));
                    }
                },
                Some(media_type) => match value {
                    Value::String(value) => value,
                    value => {
                        return Err(format!(
                            "body for Content-Type '{media_type}' must be a string, found {value:?}"
                        ));
                    }
                },
            });
        }

        let mut message = format!("{} {url}\n", request.method.as_ref());
        for (name, value) in headers {
            message.push_str(&format!("{name}: {value}\n"));
        }
        if let Some(body) = body {
            message.push('\n');
            message.push_str(&body);
        }
        Ok(message)
    }

    async fn eval_request_string(
        &self,
        expr: &Expr,
        context: &mut Context,
        field: &str,
    ) -> Result<String, String> {
        match self.eval_request_value(expr, context).await? {
            Value::String(value) => Ok(value),
            value => Err(format!(
                "{field} must evaluate to a string, found {value:?}"
            )),
        }
    }

    async fn eval_request_scalar(
        &self,
        expr: &Expr,
        context: &mut Context,
        field: &str,
    ) -> Result<String, String> {
        match self.eval_request_value(expr, context).await? {
            Value::String(value) => Ok(value),
            Value::Integer(value) => Ok(value.to_string()),
            Value::Float(value) => Ok(value.to_string()),
            Value::Boolean(value) => Ok(value.to_string()),
            value => Err(format!(
                "{field} must evaluate to a scalar, found {value:?}"
            )),
        }
    }

    async fn eval_request_value(
        &self,
        expr: &Expr,
        context: &mut Context,
    ) -> Result<Value, String> {
        let value = self.eval_expr(expr, context).await?;
        Box::pin(self.interpolate_value(value, context)).await
    }

    async fn interpolate_value(
        &self,
        value: Value,
        context: &mut Context,
    ) -> Result<Value, String> {
        match value {
            Value::String(value) => Ok(Value::String(self.interpolate(&value, context).await?)),
            Value::Array(values) => {
                let mut rendered = Vec::with_capacity(values.len());
                for value in values {
                    rendered.push(Box::pin(self.interpolate_value(value, context)).await?);
                }
                Ok(Value::Array(rendered))
            }
            Value::Map(values) => {
                let mut rendered = HashMap::with_capacity(values.len());
                for (key, value) in values {
                    let key = self.interpolate(&key, context).await?;
                    let value = Box::pin(self.interpolate_value(value, context)).await?;
                    rendered.insert(key, value);
                }
                Ok(Value::Map(rendered))
            }
            value => Ok(value),
        }
    }

    async fn interpolate(&self, input: &str, context: &mut Context) -> Result<String, String> {
        let mut output = String::new();
        let mut cursor = 0;
        while let Some(relative) = input[cursor..].find("\\(") {
            let start = cursor + relative;
            output.push_str(&input[cursor..start]);
            let expression_start = start + 2;
            let end = interpolation_end(input, expression_start)?;
            let expression = &input[expression_start..end];
            let source = Parser::new(expression)
                .parse()
                .map_err(|error| format!("invalid interpolation expression: {error}"))?;
            if source.exprs.len() != 1 || !source.clients.inner.is_empty() {
                return Err("interpolation must contain exactly one expression".to_string());
            }
            let value = self.eval_expr(&source.exprs[0], context).await?;
            output.push_str(&value.to_string());
            cursor = end + 1;
        }
        output.push_str(&input[cursor..]);
        Ok(output)
    }

    pub async fn eval_block(&self, exprs: &[Expr], context: &mut Context) -> Result<Value, String> {
        match self.eval_block_flow(exprs, context).await? {
            EvalFlow::Value(value) => Ok(value),
            EvalFlow::Break { .. } => Err("break outside loop".to_string()),
            EvalFlow::Continue { .. } => Err("continue outside loop".to_string()),
        }
    }

    async fn eval_block_flow(
        &self,
        exprs: &[Expr],
        context: &mut Context,
    ) -> Result<EvalFlow, String> {
        let mut result = Value::Null;
        for expr in exprs {
            match Box::pin(self.eval_flow(expr, context)).await? {
                EvalFlow::Value(value) => result = value,
                flow @ (EvalFlow::Break { .. } | EvalFlow::Continue { .. }) => return Ok(flow),
            }
        }
        Ok(EvalFlow::Value(result))
    }

    async fn eval_list(&self, items: &[Expr], context: &mut Context) -> Result<Vec<Value>, String> {
        let mut values = Vec::with_capacity(items.len());
        for item in items {
            values.push(self.eval_expr(item, context).await?);
        }
        Ok(values)
    }
}

fn interpolation_end(input: &str, start: usize) -> Result<usize, String> {
    let mut depth = 1usize;
    let mut quote = None;
    let mut escaped = false;
    for (offset, character) in input[start..].char_indices() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == active_quote {
                quote = None;
            }
            continue;
        }
        match character {
            '"' | '`' => quote = Some(character),
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(start + offset);
                }
            }
            _ => {}
        }
    }
    Err("unterminated interpolation expression".to_string())
}

fn request_pairs(value: &Value) -> Result<Vec<(String, String)>, String> {
    let Value::Array(entries) = value else {
        return Err("form and multipart bodies must be arrays of key-value pairs".to_string());
    };
    entries
        .iter()
        .map(|entry| {
            let Value::Array(pair) = entry else {
                return Err("body entry must be a key-value pair".to_string());
            };
            if pair.len() != 2 {
                return Err("body entry must contain exactly two values".to_string());
            }
            Ok((request_scalar(&pair[0])?, request_scalar(&pair[1])?))
        })
        .collect()
}

fn request_scalar(value: &Value) -> Result<String, String> {
    match value {
        Value::String(value) => Ok(value.to_owned()),
        Value::Integer(value) => Ok(value.to_string()),
        Value::Float(value) => Ok(value.to_string()),
        Value::Boolean(value) => Ok(value.to_string()),
        value => Err(format!("pair value must be a scalar, found {value:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::super::Context;
    use super::super::Parser;
    use super::super::Value;
    use std::collections::HashMap;

    async fn run_eval_tests(tests: Vec<(&str, Value)>) {
        for (text, expect) in tests {
            let source = Parser::new(text).parse().unwrap();
            let mut context = Context::new();
            let exprs = &source.exprs;
            match source.eval_block(exprs, &mut context).await {
                Ok(value) => {
                    println!("{exprs:?} => {value} = {expect}");
                    assert_eq!(value, expect);
                }
                Err(message) => panic!("machine error: {message}"),
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
            (
                "0.2 * 0.2 * 0.2 * 0.2 * 0.2",
                Value::Float(0.00032000000000000013),
            ),
            ("0.5 * 2.2 + 1.1", Value::Float(2.2)),
            ("0.5 + 0.2 * 10.0", Value::Float(2.5)),
            ("0.5 * (2.0 + 10.0)", Value::Float(6.0)),
            ("-0.5", Value::Float(-0.5)),
            ("-1.0", Value::Float(-1.0)),
            ("-5.0 + 10.0 + -5.0", Value::Float(0.0)),
            (
                "(0.5 + 1.5 * 0.2 + 1.5 / 3.0) * 2.0 + -1.0",
                Value::Float(1.6),
            ),
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
            (
                r#""hello world""#,
                Value::String(String::from("hello world")),
            ),
            (
                r#""hello" + " world""#,
                Value::String(String::from("hello world")),
            ),
            (
                r#""hello"+" world"+"!""#,
                Value::String(String::from("hello world!")),
            ),
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
                Value::Array(vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                ]),
            ),
            (
                "[1 + 2, 3 - 4, 5 * 6]",
                Value::Array(vec![
                    Value::Integer(3),
                    Value::Integer(-1),
                    Value::Integer(30),
                ]),
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
            (
                "let one = 1; let two = one + one; one + two",
                Value::Integer(3),
            ),
            ("let one = 1; one;", Value::Integer(1)),
            ("let one = 1; let two = 2; one + two;", Value::Integer(3)),
            (
                "let one = 1; let two = one + one; one + two;",
                Value::Integer(3),
            ),
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
            (
                "if ((if (false) { 10 })) { 10 } else { 20 }",
                Value::Integer(20),
            ),
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
    async fn test_call_function() {
        let tests = vec![
            ("fn identity(x) { x; }; identity(5);", Value::Integer(5)),
            // ("fn identity(x) { return x; }; identity(5);", Value::Integer(5)),
            ("fn double(x) { x * 2; }; double(5);", Value::Integer(10)),
            ("fn add(x, y) { x + y; }; add(5, 5);", Value::Integer(10)),
            (
                "fn add(x, y) { x + y; }; add(5 + 5, add(5, 5));",
                Value::Integer(20),
            ),
            (
                "fn one(x) { x + 1; } fn two(x) { one(one(x)); }; two(0);",
                Value::Integer(2),
            ),
            ("fn x(x) { x; }; x(5)", Value::Integer(5)),
            ("fn len(x) { x; }; len(10)", Value::Integer(10)),
        ];
        run_eval_tests(tests).await;
    }

    #[tokio::test]
    async fn test_call_fibonacci() {
        let tests = vec![(
            r#"
    fn fibonacci(x) {
        if (x == 0) {
          0
        } else {
          if (x == 1) {
            1
          } else {
            fibonacci(x - 1) + fibonacci(x -2)
          }
        }
      };
    fibonacci(22);
    "#,
            Value::Integer(17711),
        )];
        run_eval_tests(tests).await;
    }

    #[tokio::test]
    async fn test_loop_expr() {
        let tests = vec![
            ("loop { break 5 }", Value::Integer(5)),
            ("loop { break }", Value::Null),
            (
                "let i = 0; loop { let i = i + 1; if (i == 3) { break i } }",
                Value::Integer(3),
            ),
        ];
        run_eval_tests(tests).await;
    }

    #[tokio::test]
    async fn test_while_expr() {
        let tests = vec![(
            "let i = 0; while (i < 3) { let i = i + 1; i }",
            Value::Integer(3),
        )];
        run_eval_tests(tests).await;
    }

    #[tokio::test]
    async fn test_for_expr() {
        let tests = vec![
            (
                "let sum = 0; for x in [1, 2, 3] { let sum = sum + x }; sum",
                Value::Integer(6),
            ),
            (
                "let sum = 0; for x in 1..3 { let sum = sum + x }; sum",
                Value::Integer(3),
            ),
            (
                "let sum = 0; for x in 1..=3 { let sum = sum + x }; sum",
                Value::Integer(6),
            ),
        ];
        run_eval_tests(tests).await;
    }

    #[tokio::test]
    async fn test_for_range_breaks_without_collecting_entire_range() {
        let source = Parser::new("for x in 0..=9223372036854775807 { break x }")
            .parse()
            .unwrap();
        let mut context = Context::new();
        let value = source
            .eval_block(&source.exprs, &mut context)
            .await
            .unwrap();

        assert_eq!(value, Value::Integer(0));
    }

    #[tokio::test]
    async fn test_continue_expr() {
        let tests = vec![(
            "let sum = 0; for x in 1..=3 { if (x == 2) { continue }; let sum = sum + x }; sum",
            Value::Integer(4),
        )];
        run_eval_tests(tests).await;
    }

    #[tokio::test]
    async fn test_labeled_break_expr() {
        let tests = vec![(
            "'outer: loop { loop { break 'outer 7 }; break 1 }",
            Value::Integer(7),
        )];
        run_eval_tests(tests).await;
    }

    #[tokio::test]
    async fn test_loop_control_errors() {
        let tests = vec![
            ("break", "break outside loop"),
            ("continue", "continue outside loop"),
            (
                "for x in 1.. { x }",
                "open-ended range cannot be used in for loop",
            ),
        ];
        for (text, expected) in tests {
            let source = Parser::new(text).parse().unwrap();
            let mut context = Context::new();
            let error = source
                .eval_block(&source.exprs, &mut context)
                .await
                .unwrap_err();
            assert!(
                error.contains(expected),
                "{error} did not contain {expected}"
            );
        }
    }

    #[test]
    fn test_fibonacci() {
        fn fibonacci(x: i64) -> i64 {
            match x {
                0 => 0,
                1 => 1,
                _ => fibonacci(x - 1) + fibonacci(x - 2),
            }
        }
        println!("fibonacci(22):{}", fibonacci(22));
    }

    #[tokio::test]
    async fn test_client_request_call() {
        crate::tests::start_server(30006).await;
        let source = Parser::new(
            r#"
            client user {
                scheme: http,
                host: "127.0.0.1",
                port: 30006,
                requests: {
                    getIp: {
                        path: "/get",
                        method: GET,
                        headers: [["Connection", "close"]],
                        asserts: [status == 200],
                    },
                },
            }
            let host = "127.0.0.1";
            let response = user.getIp();
            response.status
            "#,
        )
        .parse()
        .unwrap();
        let mut context = Context::new();
        let value = source
            .eval_block(&source.exprs, &mut context)
            .await
            .unwrap();
        assert_eq!(value, Value::Integer(200));
        let records = context.records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].name, "user.getIp");
        assert!(records[0].asserts.iter().all(|assert| assert.result));
    }

    #[tokio::test]
    async fn test_client_request_call_rejects_arguments() {
        let source = Parser::new(
            r#"
            client user {
                scheme: http,
                host: "localhost",
                port: 30006,
                requests: { getIp: { path: "/get", method: GET } },
            }
            user.getIp(1)
            "#,
        )
        .parse()
        .unwrap();
        let mut context = Context::new();
        let error = source
            .eval_block(&source.exprs, &mut context)
            .await
            .unwrap_err();
        assert!(error.contains("client request calls do not accept arguments"));
    }

    #[tokio::test]
    async fn test_native_client_renders_expression_bodies_and_encodings() {
        let source = Parser::new(
            r#"
            client user {
                scheme: https,
                host: "example.com",
                requests: {
                    json: {
                        path: "/users/\(age + 1)",
                        method: POST,
                        params: [["tag", "a"], ["tag", "b"]],
                        body: {
                            name: "hello \(name)",
                            age: age + 1,
                            enabled: enabled,
                        },
                    },
                    jsonCase: {
                        path: "/json-case",
                        method: POST,
                        headers: [["Content-Type", "Application/JSON; Charset=UTF-8"]],
                        body: {ok: true},
                    },
                    form: {
                        path: "/form",
                        method: POST,
                        headers: [["Content-Type", "application/x-www-form-urlencoded"]],
                        body: [["age", age + 1], ["enabled", enabled]],
                    },
                    upload: {
                        path: "/upload",
                        method: POST,
                        headers: [["Content-Type", "multipart/form-data"]],
                        body: [["file", "@./avatar.png"], ["literal", "value"]],
                    },
                },
            }
            "#,
        )
        .parse()
        .unwrap();
        let client = source.clients.get("user").unwrap();
        let mut context = Context::new();
        context.set("name".to_string(), Value::String("Tom".to_string()));
        context.set("age".to_string(), Value::Integer(18));
        context.set("enabled".to_string(), Value::Boolean(true));

        let json = source
            .eval_request_message(client, client.request("json").unwrap(), &mut context)
            .await
            .unwrap();
        assert!(json.starts_with("POST https://example.com/users/19?tag=a&tag=b\n"));
        assert!(json.contains("Content-Type: application/json\n"));
        let body = json.split_once("\n\n").unwrap().1;
        let body: serde_json::Value = serde_json::from_str(body).unwrap();
        assert_eq!(body["name"], "hello Tom");
        assert_eq!(body["age"], 19);
        assert_eq!(body["enabled"], true);

        let json_case = source
            .eval_request_message(client, client.request("jsonCase").unwrap(), &mut context)
            .await
            .unwrap();
        assert!(json_case.contains("Content-Type: Application/JSON; Charset=UTF-8\n"));
        assert_eq!(json_case.split_once("\n\n").unwrap().1, "{\"ok\":true}");

        let form = source
            .eval_request_message(client, client.request("form").unwrap(), &mut context)
            .await
            .unwrap();
        assert!(form.ends_with("\nage: 19\nenabled: true"));

        let upload = source
            .eval_request_message(client, client.request("upload").unwrap(), &mut context)
            .await
            .unwrap();
        assert!(upload.ends_with("\nfile: @./avatar.png\nliteral: value"));
    }

    #[tokio::test]
    async fn test_native_client_reports_interpolation_errors() {
        let source = Parser::new(
            r#"
            client user {
                scheme: https,
                host: "example.com",
                requests: {
                    get: { path: "/\(missing)", method: GET },
                },
            }
            "#,
        )
        .parse()
        .unwrap();
        let client = source.clients.get("user").unwrap();
        let error = source
            .eval_request_message(client, client.request("get").unwrap(), &mut Context::new())
            .await
            .unwrap_err();
        assert!(error.contains("ident: missing not found"));
    }

    #[tokio::test]
    async fn test_unknown_client_request_reports_error() {
        let source = Parser::new("user.missing();").parse().unwrap();
        let mut context = Context::new();
        let error = source
            .eval_block(&source.exprs, &mut context)
            .await
            .unwrap_err();
        assert!(error.contains("request user.missing not found"));
    }
}
