// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 3.0 или новее.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Выражения строк сетки профиля 5.02. В модуле их считает QgsExpression,
//! здесь - свой разбор той части, что нужна строкам сетки: числа, поля
//! таблицы (имя или имя в двойных кавычках), `+ - * /`, скобки, унарный
//! минус. Пустое значение поля и деление на ноль дают пусто (NaN), как
//! NULL в выражениях QGIS.

use serde_json::Value as J;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(f64),
    Field(String),
    Neg(Box<Expr>),
    Bin(char, Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Name(String),
    Op(char),
}

fn tokens(s: &str) -> Result<Vec<Tok>, String> {
    let cs: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit()
            || (c == '.' && cs.get(i + 1).is_some_and(char::is_ascii_digit))
        {
            let start = i;
            while i < cs.len() && (cs[i].is_ascii_digit() || cs[i] == '.') {
                i += 1;
            }
            if i < cs.len() && (cs[i] == 'e' || cs[i] == 'E') {
                let mut j = i + 1;
                if j < cs.len() && (cs[j] == '+' || cs[j] == '-') {
                    j += 1;
                }
                if j < cs.len() && cs[j].is_ascii_digit() {
                    i = j;
                    while i < cs.len() && cs[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            let t: String = cs[start..i].iter().collect();
            out.push(Tok::Num(t.parse().map_err(|_| format!("число «{t}»"))?));
        } else if c == '"' {
            let mut name = String::new();
            i += 1;
            loop {
                match cs.get(i) {
                    None => return Err("нет закрывающей кавычки".into()),
                    Some('"') if cs.get(i + 1) == Some(&'"') => {
                        name.push('"');
                        i += 2;
                    }
                    Some('"') => {
                        i += 1;
                        break;
                    }
                    Some(&ch) => {
                        name.push(ch);
                        i += 1;
                    }
                }
            }
            out.push(Tok::Name(name));
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < cs.len() && (cs[i].is_alphanumeric() || cs[i] == '_') {
                i += 1;
            }
            out.push(Tok::Name(cs[start..i].iter().collect()));
        } else if "+-*/()".contains(c) {
            out.push(Tok::Op(c));
            i += 1;
        } else {
            return Err(format!("знак «{c}»"));
        }
    }
    Ok(out)
}

struct Parser {
    t: Vec<Tok>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.i)
    }

    fn expr(&mut self) -> Result<Expr, String> {
        let mut a = self.term()?;
        while let Some(Tok::Op(op @ ('+' | '-'))) = self.peek().cloned() {
            self.i += 1;
            a = Expr::Bin(op, Box::new(a), Box::new(self.term()?));
        }
        Ok(a)
    }

    fn term(&mut self) -> Result<Expr, String> {
        let mut a = self.unary()?;
        while let Some(Tok::Op(op @ ('*' | '/'))) = self.peek().cloned() {
            self.i += 1;
            a = Expr::Bin(op, Box::new(a), Box::new(self.unary()?));
        }
        Ok(a)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        match self.peek().cloned() {
            Some(Tok::Op('-')) => {
                self.i += 1;
                Ok(Expr::Neg(Box::new(self.unary()?)))
            }
            Some(Tok::Op('+')) => {
                self.i += 1;
                self.unary()
            }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Result<Expr, String> {
        let tok = self.peek().cloned();
        self.i += 1;
        match tok {
            Some(Tok::Num(v)) => Ok(Expr::Num(v)),
            Some(Tok::Name(n)) => Ok(Expr::Field(n)),
            Some(Tok::Op('(')) => {
                let e = self.expr()?;
                match self.peek() {
                    Some(Tok::Op(')')) => {
                        self.i += 1;
                        Ok(e)
                    }
                    _ => Err("нет закрывающей скобки".into()),
                }
            }
            Some(Tok::Op(c)) => Err(format!("знак «{c}» не на месте")),
            None => Err("выражение оборвано".into()),
        }
    }
}

impl Expr {
    /// Разбор выражения. Ошибка - текст причины.
    pub fn parse(s: &str) -> Result<Expr, String> {
        let mut p = Parser {
            t: tokens(s)?,
            i: 0,
        };
        let e = p.expr()?;
        if p.i < p.t.len() {
            return Err("лишнее после выражения".into());
        }
        Ok(e)
    }

    /// Поля, на которые ссылается выражение.
    pub fn fields(&self) -> Vec<&str> {
        match self {
            Expr::Num(_) => Vec::new(),
            Expr::Field(n) => vec![n.as_str()],
            Expr::Neg(a) => a.fields(),
            Expr::Bin(_, a, b) => {
                let mut v = a.fields();
                v.extend(b.fields());
                v
            }
        }
    }

    /// Значение на записи: поле - число или текст числа, иначе NaN.
    pub fn eval(&self, attrs: &J) -> f64 {
        match self {
            Expr::Num(v) => *v,
            Expr::Field(n) => super::common::num(&attrs[n]).unwrap_or(f64::NAN),
            Expr::Neg(a) => -a.eval(attrs),
            Expr::Bin(op, a, b) => {
                let (x, y) = (a.eval(attrs), b.eval(attrs));
                match op {
                    '+' => x + y,
                    '-' => x - y,
                    '*' => x * y,
                    _ if y == 0.0 => f64::NAN,
                    _ => x / y,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn grid_expressions() {
        let a = json!({"z_dem": 151.5, "z_axis": 150.0, "pipe": null});
        let e = Expr::parse("\"z_dem\" - (\"z_axis\" - 0.16 - 0.3)").unwrap();
        assert!((e.eval(&a) - 1.96).abs() < 1e-12);
        assert_eq!(e.fields(), vec!["z_dem", "z_axis"]);
        assert!(Expr::parse("pipe * 2").unwrap().eval(&a).is_nan());
        assert!((Expr::parse("-z_axis / 2e1").unwrap().eval(&a) + 7.5).abs() < 1e-12);
        assert!(Expr::parse("(1 + 2").is_err());
        assert!(Expr::parse("1 ^ 2").is_err());
    }
}
