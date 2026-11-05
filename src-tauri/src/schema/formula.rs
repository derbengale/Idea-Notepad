use crate::error::AppError;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    Text,
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    LParen,
    RParen,
    Comma,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Not,
    True,
    False,
    Null,
    End,
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, AppError> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0usize;
    while index < chars.len() {
        let current = chars[index];
        if current.is_whitespace() {
            index += 1;
            continue;
        }
        match current {
            '+' => {
                tokens.push(Token::Plus);
                index += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                index += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                index += 1;
            }
            '/' => {
                tokens.push(Token::Slash);
                index += 1;
            }
            '%' => {
                tokens.push(Token::Percent);
                index += 1;
            }
            '^' => {
                tokens.push(Token::Caret);
                index += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                index += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                index += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                index += 1;
            }
            '=' => {
                if chars.get(index + 1) == Some(&'=') {
                    index += 2;
                } else {
                    index += 1;
                }
                tokens.push(Token::Eq);
            }
            '!' => {
                if chars.get(index + 1) == Some(&'=') {
                    tokens.push(Token::Ne);
                    index += 2;
                } else {
                    tokens.push(Token::Not);
                    index += 1;
                }
            }
            '<' => {
                if chars.get(index + 1) == Some(&'=') {
                    tokens.push(Token::Le);
                    index += 2;
                } else {
                    tokens.push(Token::Lt);
                    index += 1;
                }
            }
            '>' => {
                if chars.get(index + 1) == Some(&'=') {
                    tokens.push(Token::Ge);
                    index += 2;
                } else {
                    tokens.push(Token::Gt);
                    index += 1;
                }
            }
            '&' => {
                if chars.get(index + 1) == Some(&'&') {
                    tokens.push(Token::And);
                    index += 2;
                } else {
                    return Err(AppError::validation("Unexpected character '&'"));
                }
            }
            '|' => {
                if chars.get(index + 1) == Some(&'|') {
                    tokens.push(Token::Or);
                    index += 2;
                } else {
                    return Err(AppError::validation("Unexpected character '|'"));
                }
            }
            '"' | '\'' => {
                let quote = current;
                index += 1;
                let mut closed = false;
                while index < chars.len() {
                    let ch = chars[index];
                    if ch == '\\' && chars.get(index + 1).is_some() {
                        index += 2;
                        continue;
                    }
                    if ch == quote {
                        closed = true;
                        index += 1;
                        break;
                    }
                    index += 1;
                }
                if !closed {
                    return Err(AppError::validation("Unterminated string literal"));
                }
                tokens.push(Token::Text);
            }
            _ if current.is_ascii_digit() || (current == '.' && index + 1 < chars.len()) => {
                let start = index;
                let mut seen_dot = false;
                while index < chars.len() {
                    let ch = chars[index];
                    if ch.is_ascii_digit() {
                        index += 1;
                    } else if ch == '.' && !seen_dot {
                        seen_dot = true;
                        index += 1;
                    } else {
                        break;
                    }
                }
                if index < chars.len() && (chars[index] == 'e' || chars[index] == 'E') {
                    index += 1;
                    if index < chars.len() && (chars[index] == '+' || chars[index] == '-') {
                        index += 1;
                    }
                    while index < chars.len() && chars[index].is_ascii_digit() {
                        index += 1;
                    }
                }
                let text: String = chars[start..index].iter().collect();
                let number = text
                    .parse::<f64>()
                    .map_err(|_| AppError::validation("Invalid number literal"))?;
                if !number.is_finite() {
                    return Err(AppError::validation("Number literal is not finite"));
                }
                tokens.push(Token::Number(number));
            }
            _ if current.is_ascii_alphabetic() || current == '_' => {
                let start = index;
                while index < chars.len()
                    && (chars[index].is_ascii_alphanumeric() || chars[index] == '_')
                {
                    index += 1;
                }
                let word: String = chars[start..index].iter().collect();
                match word.as_str() {
                    "and" => tokens.push(Token::And),
                    "or" => tokens.push(Token::Or),
                    "not" => tokens.push(Token::Not),
                    "true" => tokens.push(Token::True),
                    "false" => tokens.push(Token::False),
                    "null" => tokens.push(Token::Null),
                    _ => tokens.push(Token::Ident(word)),
                }
            }
            _ => {
                return Err(AppError::validation(format!(
                    "Unexpected character '{current}'"
                )))
            }
        }
    }
    tokens.push(Token::End);
    Ok(tokens)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp {
    Negate,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    Text,
    Bool(bool),
    Null,
    Ident(String),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

pub fn parse(input: &str) -> Result<Expr, AppError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser {
        tokens,
        position: 0,
    };
    let expr = parser.parse_or()?;
    if parser.peek() != &Token::End {
        return Err(AppError::validation("Unexpected trailing input"));
    }
    Ok(expr)
}

impl Parser {
    fn peek(&self) -> &Token {
        self.tokens.get(self.position).unwrap_or(&Token::End)
    }

    fn advance(&mut self) -> Token {
        let token = self.peek().clone();
        self.position += 1;
        token
    }

    fn consume(&mut self, expected: &Token) -> Result<(), AppError> {
        if self.peek() == expected {
            self.position += 1;
            Ok(())
        } else {
            Err(AppError::validation(format!(
                "Expected {expected:?}, found {:?}",
                self.peek()
            )))
        }
    }

    fn parse_or(&mut self) -> Result<Expr, AppError> {
        let mut left = self.parse_and()?;
        while self.peek() == &Token::Or {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Binary(BinOp::Or, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, AppError> {
        let mut left = self.parse_equality()?;
        while self.peek() == &Token::And {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::Binary(BinOp::And, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, AppError> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = match self.peek() {
                Token::Eq => BinOp::Eq,
                Token::Ne => BinOp::Ne,
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, AppError> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                Token::Lt => BinOp::Lt,
                Token::Le => BinOp::Le,
                Token::Gt => BinOp::Gt,
                Token::Ge => BinOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, AppError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, AppError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, AppError> {
        match self.peek() {
            Token::Minus => {
                self.advance();
                Ok(Expr::Unary(UnOp::Negate, Box::new(self.parse_unary()?)))
            }
            Token::Plus => {
                self.advance();
                self.parse_unary()
            }
            Token::Not => {
                self.advance();
                Ok(Expr::Unary(UnOp::Not, Box::new(self.parse_unary()?)))
            }
            _ => self.parse_power(),
        }
    }

    fn parse_power(&mut self) -> Result<Expr, AppError> {
        let left = self.parse_primary()?;
        if self.peek() == &Token::Caret {
            self.advance();
            let right = self.parse_unary()?;
            return Ok(Expr::Binary(BinOp::Pow, Box::new(left), Box::new(right)));
        }
        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expr, AppError> {
        match self.advance() {
            Token::Number(value) => Ok(Expr::Number(value)),
            Token::Text => Ok(Expr::Text),
            Token::True => Ok(Expr::Bool(true)),
            Token::False => Ok(Expr::Bool(false)),
            Token::Null => Ok(Expr::Null),
            Token::Ident(name) => {
                if self.peek() == &Token::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    if self.peek() != &Token::RParen {
                        loop {
                            args.push(self.parse_or()?);
                            if self.peek() == &Token::Comma {
                                self.advance();
                                continue;
                            }
                            break;
                        }
                    }
                    self.consume(&Token::RParen)?;
                    Ok(Expr::Call(name, args))
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            Token::LParen => {
                let expr = self.parse_or()?;
                self.consume(&Token::RParen)?;
                Ok(expr)
            }
            other => Err(AppError::validation(format!("Unexpected token {other:?}"))),
        }
    }
}

pub fn identifiers(expr: &Expr) -> Vec<String> {
    let mut out = Vec::new();
    collect_identifiers(expr, &mut out);
    out.sort();
    out.dedup();
    out
}

fn collect_identifiers(expr: &Expr, out: &mut Vec<String>) {
    match expr {
        Expr::Ident(name) => out.push(name.clone()),
        Expr::Unary(_, inner) => collect_identifiers(inner, out),
        Expr::Binary(_, left, right) => {
            collect_identifiers(left, out);
            collect_identifiers(right, out);
        }
        Expr::Call(_, args) => {
            for arg in args {
                collect_identifiers(arg, out);
            }
        }
        _ => {}
    }
}

pub fn evaluate(expr: &Expr, values: &HashMap<String, Option<f64>>) -> Option<f64> {
    eval(expr, values)
}

fn eval(expr: &Expr, values: &HashMap<String, Option<f64>>) -> Option<f64> {
    match expr {
        Expr::Number(value) => Some(*value),
        Expr::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
        Expr::Text | Expr::Null => None,
        Expr::Ident(name) => values.get(name).copied().flatten(),
        Expr::Unary(op, inner) => {
            let value = eval(inner, values)?;
            match op {
                UnOp::Negate => Some(-value),
                UnOp::Not => Some(if value == 0.0 { 1.0 } else { 0.0 }),
            }
        }
        Expr::Binary(op, left, right) => match op {
            BinOp::And => {
                let left_value = eval(left, values);
                if left_value == Some(0.0) {
                    return Some(0.0);
                }
                let right_value = eval(right, values);
                match (left_value, right_value) {
                    (Some(a), Some(b)) => Some(if a != 0.0 && b != 0.0 { 1.0 } else { 0.0 }),
                    _ => None,
                }
            }
            BinOp::Or => {
                let left_value = eval(left, values);
                if left_value.map(|v| v != 0.0) == Some(true) {
                    return Some(1.0);
                }
                let right_value = eval(right, values);
                match (left_value, right_value) {
                    (Some(a), Some(b)) => Some(if a != 0.0 || b != 0.0 { 1.0 } else { 0.0 }),
                    _ => None,
                }
            }
            _ => {
                let a = eval(left, values)?;
                let b = eval(right, values)?;
                numeric_binary(*op, a, b)
            }
        },
        Expr::Call(name, args) => eval_call(name, args, values),
    }
}

fn numeric_binary(op: BinOp, a: f64, b: f64) -> Option<f64> {
    let result = match op {
        BinOp::Add => a + b,
        BinOp::Sub => a - b,
        BinOp::Mul => a * b,
        BinOp::Div => {
            if b == 0.0 {
                return None;
            }
            a / b
        }
        BinOp::Mod => {
            if b == 0.0 {
                return None;
            }
            a % b
        }
        BinOp::Pow => {
            let value = a.powf(b);
            if value.is_finite() {
                value
            } else {
                return None;
            }
        }
        BinOp::Eq => {
            return Some(if (a - b).abs() < f64::EPSILON {
                1.0
            } else {
                0.0
            })
        }
        BinOp::Ne => {
            return Some(if (a - b).abs() < f64::EPSILON {
                0.0
            } else {
                1.0
            })
        }
        BinOp::Lt => return Some(if a < b { 1.0 } else { 0.0 }),
        BinOp::Le => return Some(if a <= b { 1.0 } else { 0.0 }),
        BinOp::Gt => return Some(if a > b { 1.0 } else { 0.0 }),
        BinOp::Ge => return Some(if a >= b { 1.0 } else { 0.0 }),
        BinOp::And | BinOp::Or => return None,
    };
    if result.is_finite() {
        Some(result)
    } else {
        None
    }
}

fn eval_call(name: &str, args: &[Expr], values: &HashMap<String, Option<f64>>) -> Option<f64> {
    match name {
        "if" => {
            if args.len() != 3 {
                return None;
            }
            let condition = eval(&args[0], values);
            if condition.map(|value| value != 0.0).unwrap_or(false) {
                eval(&args[1], values)
            } else {
                eval(&args[2], values)
            }
        }
        "coalesce" => {
            for arg in args {
                if let Some(value) = eval(arg, values) {
                    return Some(value);
                }
            }
            None
        }
        "min" | "max" => {
            if args.is_empty() {
                return None;
            }
            let mut collected = Vec::with_capacity(args.len());
            for arg in args {
                collected.push(eval(arg, values)?);
            }
            let mut result = collected[0];
            for value in collected.iter().skip(1) {
                if (name == "min" && *value < result) || (name == "max" && *value > result) {
                    result = *value;
                }
            }
            Some(result)
        }
        "round" => {
            if args.is_empty() || args.len() > 2 {
                return None;
            }
            let value = eval(&args[0], values)?;
            let digits = match args.get(1) {
                Some(expr) => eval(expr, values)?,
                None => 0.0,
            };
            let factor = 10f64.powi(digits.clamp(0.0, 12.0) as i32);
            let rounded = (value * factor).round() / factor;
            if rounded.is_finite() {
                Some(rounded)
            } else {
                None
            }
        }
        "floor" => {
            if args.len() != 1 {
                return None;
            }
            Some(eval(&args[0], values)?.floor())
        }
        "ceil" => {
            if args.len() != 1 {
                return None;
            }
            Some(eval(&args[0], values)?.ceil())
        }
        "abs" => {
            if args.len() != 1 {
                return None;
            }
            Some(eval(&args[0], values)?.abs())
        }
        "clamp" => {
            if args.len() != 3 {
                return None;
            }
            let value = eval(&args[0], values)?;
            let low = eval(&args[1], values)?;
            let high = eval(&args[2], values)?;
            Some(value.max(low).min(high))
        }
        _ => None,
    }
}

pub fn find_cycle(formulas: &[(String, String)]) -> Result<(), AppError> {
    let graph: HashMap<String, Vec<String>> = formulas
        .iter()
        .map(|(key, expression)| {
            let references = parse(expression)
                .map(|expr| identifiers(&expr))
                .unwrap_or_default();
            (key.clone(), references)
        })
        .collect();
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for key in graph.keys() {
        if detects_cycle(key, &graph, &mut visiting, &mut visited) {
            return Err(AppError::validation(format!(
                "Formula cycle detected at '{key}'"
            )));
        }
    }
    Ok(())
}

fn detects_cycle(
    key: &str,
    graph: &HashMap<String, Vec<String>>,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
) -> bool {
    if visited.contains(key) {
        return false;
    }
    if !visiting.insert(key.to_string()) {
        return true;
    }
    if let Some(deps) = graph.get(key) {
        for dep in deps {
            if graph.contains_key(dep) && detects_cycle(dep, graph, visiting, visited) {
                return true;
            }
        }
    }
    visiting.remove(key);
    visited.insert(key.to_string());
    false
}
