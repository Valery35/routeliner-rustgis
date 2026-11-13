// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 2.0 или новее.
// SPDX-License-Identifier: GPL-2.0-or-later

//! Числа и шаблоны так, как их пишет Python модуля QGIS.
//!
//! Python `round(x, 3)` и `f"{x:.2f}"` округляют точное двоичное значение
//! числа, ровную половину - к чётной цифре. Здесь то же самое через точную
//! десятичную запись числа, поэтому поля `rl_*` и подписи ПК совпадают
//! с модулем и в случаях «ровно половина».

/// Число `x` с `d` знаками после точки, как `f"{x:.{d}f}"` в Python.
pub fn fixed(x: f64, d: usize) -> String {
    if x.is_nan() {
        return "nan".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "inf" } else { "-inf" }.into();
    }
    // точная запись: у числа не меньше 1e-3 дробная часть короче 80 цифр,
    // у меньшего ровной половины на 3-6 знаке не бывает
    let exact = format!("{:.80}", x.abs());
    let (int, frac) = exact.split_once('.').unwrap_or((exact.as_str(), ""));
    let frac = frac.as_bytes();
    let mut digits: Vec<u8> = int
        .bytes()
        .chain(frac.iter().take(d).copied())
        .map(|b| b - b'0')
        .collect();
    let rest = frac.get(d..).unwrap_or(&[]);
    let first = rest.first().map_or(0, |b| b - b'0');
    let tail = rest.iter().skip(1).any(|&b| b != b'0');
    let odd = digits.last().is_some_and(|v| v % 2 == 1);
    let mut int_len = int.len();
    if first > 5 || (first == 5 && (tail || odd)) {
        let mut i = digits.len();
        loop {
            if i == 0 {
                digits.insert(0, 1);
                int_len += 1;
                break;
            }
            i -= 1;
            if digits[i] == 9 {
                digits[i] = 0;
            } else {
                digits[i] += 1;
                break;
            }
        }
    }
    let mut s = String::new();
    if x.is_sign_negative() {
        s.push('-');
    }
    for (i, v) in digits.iter().enumerate() {
        if i == int_len {
            s.push('.');
        }
        s.push(char::from(b'0' + v));
    }
    s
}

/// Python `round(x, d)`.
pub fn round(x: f64, d: usize) -> f64 {
    if !x.is_finite() {
        return x;
    }
    fixed(x, d).parse().unwrap_or(x)
}

/// Python `f"{x:0{w}.{d}f}"`: нули слева до ширины `w`.
pub fn fixed_width(x: f64, w: usize, d: usize) -> String {
    let s = fixed(x, d);
    let (sign, body) = match s.strip_prefix('-') {
        Some(b) => ("-", b),
        None => ("", s.as_str()),
    };
    let pad = w.saturating_sub(s.len());
    format!("{sign}{}{body}", "0".repeat(pad))
}

/// Python `repr(x)` и `str(x)` для float: кратчайшая запись с `.0`
/// у целых, порядок `e+16` и `e-05` вне 1e-4..1e16.
pub fn repr(x: f64) -> String {
    if x.is_nan() {
        return "nan".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "inf" } else { "-inf" }.into();
    }
    let a = x.abs();
    if a == 0.0 || (1e-4..1e16).contains(&a) {
        let s = format!("{x}");
        return if s.contains('.') { s } else { format!("{s}.0") };
    }
    exp_style(&format!("{x:e}"))
}

/// Порядок по-питоновски: `1.5e-7` - `1.5e-07`, `1e16` - `1e+16`.
fn exp_style(s: &str) -> String {
    let Some((mant, exp)) = s.split_once('e') else {
        return s.to_string();
    };
    let (sign, digits) = match exp.strip_prefix('-') {
        Some(d) => ('-', d),
        None => ('+', exp),
    };
    format!("{mant}e{sign}{digits:0>2}")
}

/// Python `f"{x:g}"`: шесть значащих цифр без хвостовых нулей.
pub fn general(x: f64) -> String {
    if !x.is_finite() {
        return repr(x);
    }
    if x == 0.0 {
        return if x.is_sign_negative() { "-0" } else { "0" }.into();
    }
    let e = format!("{x:.5e}");
    let exp: i32 = e
        .split_once('e')
        .and_then(|(_, p)| p.parse().ok())
        .unwrap_or(0);
    if (-4..6).contains(&exp) {
        let s = fixed(x, usize::try_from(5 - exp).unwrap_or(0));
        strip_zeros(&s)
    } else {
        let (mant, _) = e.split_once('e').unwrap_or((e.as_str(), ""));
        exp_style(&format!("{}e{exp}", strip_zeros(mant)))
    }
}

fn strip_zeros(s: &str) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s.to_string()
    }
}

/// Python `a // b` для float (CPython `float_floor_div`).
pub fn floor_div(a: f64, b: f64) -> f64 {
    let mut m = a % b;
    let mut div = (a - m) / b;
    if m != 0.0 {
        if (b < 0.0) != (m < 0.0) {
            m += b;
            div -= 1.0;
        }
    } else {
        m = 0.0_f64.copysign(b);
    }
    let _ = m;
    if div != 0.0 {
        let mut fd = div.floor();
        if div - fd > 0.5 {
            fd += 1.0;
        }
        fd
    } else {
        0.0_f64.copysign(a / b)
    }
}

/// Сумма `numpy.sum` для одномерного массива: попарная по блокам 8 и 128.
pub fn np_sum(a: &[f64]) -> f64 {
    let n = a.len();
    if n < 8 {
        let mut res = 0.0;
        for v in a {
            res += v;
        }
        res
    } else if n <= 128 {
        let mut r = [0.0; 8];
        r.copy_from_slice(&a[..8]);
        let mut i = 8;
        while i < n - (n % 8) {
            for (j, rj) in r.iter_mut().enumerate() {
                *rj += a[i + j];
            }
            i += 8;
        }
        let mut res = ((r[0] + r[1]) + (r[2] + r[3])) + ((r[4] + r[5]) + (r[6] + r[7]));
        while i < n {
            res += a[i];
            i += 1;
        }
        res
    } else {
        let mut n2 = n / 2;
        n2 -= n2 % 8;
        np_sum(&a[..n2]) + np_sum(&a[n2..])
    }
}

/// Значение подстановки в шаблон.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    /// Текст как есть.
    S(String),
    /// Число с плавающей точкой.
    F(f64),
    /// Целое.
    I(i64),
    /// Вложенное сообщение, переводится и подставляется целиком.
    M(Box<Msg>),
}

/// Сообщение: русский шаблон модуля с подстановками в фигурных скобках
/// (`{m:.3f}`, `{t:g}`, `{rid}`). Переводится при показе: шаблон берётся
/// из `messages` в `tools.json` на языке RustGIS.
#[derive(Debug, Clone, PartialEq)]
pub struct Msg {
    pub text: &'static str,
    pub args: Vec<(&'static str, Arg)>,
}

impl Msg {
    pub fn new(text: &'static str) -> Self {
        Self {
            text,
            args: Vec::new(),
        }
    }

    pub fn f(mut self, k: &'static str, v: f64) -> Self {
        self.args.push((k, Arg::F(v)));
        self
    }

    pub fn i(mut self, k: &'static str, v: i64) -> Self {
        self.args.push((k, Arg::I(v)));
        self
    }

    pub fn s(mut self, k: &'static str, v: impl Into<String>) -> Self {
        self.args.push((k, Arg::S(v.into())));
        self
    }

    pub fn m(mut self, k: &'static str, v: Msg) -> Self {
        self.args.push((k, Arg::M(Box::new(v))));
        self
    }

    /// Текст на языке показа: `tr` переводит русский шаблон.
    pub fn render(&self, tr: &dyn Fn(&str) -> String) -> String {
        let t = tr(self.text);
        let mut out = String::new();
        let mut rest = t.as_str();
        while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            let after = &rest[open + 1..];
            let Some(close) = after.find('}') else {
                out.push_str(&rest[open..]);
                rest = "";
                break;
            };
            let field = &after[..close];
            let (name, spec) = field.split_once(':').unwrap_or((field, ""));
            match self.args.iter().find(|(k, _)| *k == name) {
                Some((_, a)) => out.push_str(&arg_text(a, spec, tr)),
                None => {
                    out.push('{');
                    out.push_str(field);
                    out.push('}');
                }
            }
            rest = &after[close + 1..];
        }
        out.push_str(rest);
        out
    }
}

fn arg_text(a: &Arg, spec: &str, tr: &dyn Fn(&str) -> String) -> String {
    let num = match a {
        Arg::F(v) => Some(*v),
        Arg::I(v) => Some(*v as f64),
        _ => None,
    };
    if let (Some(v), Some(d)) = (
        num,
        spec.strip_prefix('.')
            .and_then(|s| s.strip_suffix('f'))
            .and_then(|s| s.parse::<usize>().ok()),
    ) {
        return fixed(v, d);
    }
    if let (Some(v), "g") = (num, spec) {
        return general(v);
    }
    match a {
        Arg::S(s) => s.clone(),
        Arg::F(v) => repr(*v),
        Arg::I(v) => v.to_string(),
        Arg::M(m) => m.render(tr),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_like_python() {
        // ровная половина - к чётной цифре, иначе по точному значению
        assert_eq!(fixed(0.125, 2), "0.12");
        assert_eq!(fixed(0.375, 2), "0.38");
        assert_eq!(fixed(2.675, 2), "2.67");
        assert_eq!(fixed(2.5, 0), "2");
        assert_eq!(fixed(9.995, 2), "9.99");
        assert_eq!(fixed(99.999, 2), "100.00");
        assert_eq!(fixed(-0.001, 2), "-0.00");
        assert_eq!(fixed(1471.2365, 3), "1471.236");
        assert_eq!(round(1471.2365, 3), 1471.236);
        assert_eq!(fixed_width(5.0, 5, 2), "05.00");
        assert_eq!(fixed_width(35.5, 5, 2), "35.50");
    }

    #[test]
    fn repr_and_general() {
        assert_eq!(repr(699.3), "699.3");
        assert_eq!(repr(700.0), "700.0");
        assert_eq!(repr(1e16), "1e+16");
        assert_eq!(repr(1.5e-7), "1.5e-07");
        assert_eq!(general(0.01), "0.01");
        assert_eq!(general(50.0), "50");
        assert_eq!(general(1234567.0), "1.23457e+06");
        assert_eq!(general(0.00001), "1e-05");
    }

    #[test]
    fn floor_div_and_sum() {
        assert_eq!(floor_div(7.0, 2.0), 3.0);
        assert_eq!(floor_div(-7.0, 2.0), -4.0);
        assert_eq!(floor_div(-1e-6, 100.0), -1.0);
        let v: Vec<f64> = (0..300).map(|i| f64::from(i) * 0.1).collect();
        assert!((np_sum(&v) - 4485.0).abs() < 1e-9);
    }

    #[test]
    fn templates() {
        let m = Msg::new("мера {m:.3f} вне 0..{L:.3f}, допуск {t:g}, {n} раза, {x}")
            .f("m", 1.23456)
            .f("L", 10.0)
            .f("t", 0.01)
            .i("n", 2)
            .s("x", "R1");
        assert_eq!(
            m.render(&|s| s.to_string()),
            "мера 1.235 вне 0..10.000, допуск 0.01, 2 раза, R1"
        );
        let outer = Msg::new("система «{name}»: {why}")
            .s("name", "ведомость")
            .m("why", Msg::new("ведомость пуста"));
        let en = |s: &str| match s {
            "система «{name}»: {why}" => "system «{name}»: {why}".to_string(),
            "ведомость пуста" => "the ledger is empty".to_string(),
            o => o.to_string(),
        };
        assert_eq!(outer.render(&en), "system «ведомость»: the ledger is empty");
    }
}
