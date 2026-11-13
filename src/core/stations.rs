// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 2.0 или новее.
// SPDX-License-Identifier: GPL-2.0-or-later

//! Разбор и запись пикетов (`routeliner/core/stations.py`).
//!
//! Режим разбора задаётся явно. Значение пикета всегда в метрах пикетажа,
//! в меру по оси его переводит [`super::chainage::ChainageSystem`].
//!
//! - `PkPlus` - `ПК 15+35`, `15+35`, `ПК15+35.5`, `15+35,5`, `ПК -1+50`,
//!   `ПК 15`, `+35` (тот же пикет, что у предыдущей записи);
//! - `KmPlus` - `км 1+535`, `1+535.2`;
//! - `Rail` - `км 12 ПК 3+45`, километры и пикеты 1..10 с единицы;
//! - `Meters` - `1535`, `1535,5`, числа из таблицы;
//! - `Km` - `1,535` или 1.535, километры дробью.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value as J;

use super::errors::{CoreError, ErrorCode};
use crate::pyfmt::{self, Msg};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StationFormat {
    PkPlus,
    KmPlus,
    Rail,
    Meters,
    Km,
}

impl StationFormat {
    /// Порядок пунктов поля FORMAT модуля.
    pub const ALL: [StationFormat; 5] = [
        StationFormat::PkPlus,
        StationFormat::KmPlus,
        StationFormat::Rail,
        StationFormat::Meters,
        StationFormat::Km,
    ];

    pub fn value(self) -> &'static str {
        match self {
            StationFormat::PkPlus => "pk_plus",
            StationFormat::KmPlus => "km_plus",
            StationFormat::Rail => "rail",
            StationFormat::Meters => "meters",
            StationFormat::Km => "km",
        }
    }
}

/// Пикет: метры пикетажа.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Station {
    pub value: f64,
}

const NUM: &str = r"\d+(?:[.,]\d+)?";

fn re(p: &str) -> Regex {
    // шаблоны постоянные, ошибку в них ловит первый же тест
    Regex::new(p).unwrap_or_else(|e| unreachable!("{e}"))
}

static RE_PK_PLUS: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(
        r"^(?:пк|pk)?\s*(?P<sign>[-−–])?\s*(?P<pk>\d+)\s*(?:\+\s*(?P<plus>{NUM}))?$"
    ))
});
static RE_REL: LazyLock<Regex> = LazyLock::new(|| re(&format!(r"^\+\s*(?P<plus>{NUM})$")));
static RE_KM_PLUS: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(
        r"^(?:км|km)?\s*(?P<sign>[-−–])?\s*(?P<km>\d+)\s*\+\s*(?P<plus>{NUM})$"
    ))
});
static RE_RAIL: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(
        r"^(?:км|km)\s*(?P<km>\d+)\s*[,;]?\s*(?:пк|pk)\s*(?P<pk>\d+)\s*(?:\+\s*(?P<plus>{NUM}))?$"
    ))
});
static RE_NUM: LazyLock<Regex> =
    LazyLock::new(|| re(&format!(r"^(?P<sign>[-−–])?\s*(?P<v>{NUM})$")));
static RE_WS: LazyLock<Regex> = LazyLock::new(|| re(r"\s+"));

/// Число из записи: запятая - десятичная точка. Цифры не ASCII Python
/// понимает, здесь они дают ошибку разбора.
fn num(text: &str) -> Option<f64> {
    text.replace(',', ".").parse().ok()
}

fn norm(raw: &str) -> String {
    let s = raw.trim().to_lowercase().replace('\u{a0}', " ");
    RE_WS.replace_all(&s, " ").into_owned()
}

/// Значение ячейки как `str()` Python: для текста ошибки.
pub fn py_str(raw: &J) -> String {
    match raw {
        J::Null => "None".into(),
        J::Bool(b) => if *b { "True" } else { "False" }.into(),
        J::Number(n) => match (n.as_i64(), n.as_f64()) {
            (Some(i), _) => i.to_string(),
            (None, Some(f)) => pyfmt::repr(f),
            _ => n.to_string(),
        },
        J::String(s) => s.clone(),
        other => other.to_string(),
    }
}

enum Got {
    Value(f64),
    NoMatch,
    Err(CoreError),
}

/// Разбор и запись пикетов одного режима.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StationParser {
    pub fmt: StationFormat,
    pub picket_length: f64,
    pub decimals: usize,
}

impl StationParser {
    /// Длина пикета не больше нуля - None (в модуле ValueError).
    pub fn new(fmt: StationFormat, picket_length: f64) -> Option<Self> {
        (picket_length > 0.0).then_some(Self {
            fmt,
            picket_length,
            decimals: 2,
        })
    }

    pub fn parse(&self, raw: &J, previous: Option<&Station>) -> Result<Station, CoreError> {
        match raw {
            J::Null => return Err(self.fail(raw, Msg::new("пустое значение"))),
            J::Number(n) => {
                let v = n.as_f64().unwrap_or(f64::NAN);
                if v.is_nan() {
                    return Err(self.fail(raw, Msg::new("пустое значение")));
                }
                return match self.fmt {
                    StationFormat::Meters => Ok(Station { value: v }),
                    StationFormat::Km => Ok(Station { value: v * 1000.0 }),
                    _ => Err(self.fail(raw, Msg::new("число без «+» в режиме записи с плюсом"))),
                };
            }
            _ => {}
        }
        let s = norm(&py_str(raw));
        if s.is_empty() {
            return Err(self.fail(raw, Msg::new("пустое значение")));
        }
        let got = match self.fmt {
            StationFormat::PkPlus => self.parse_pk_plus(&s, previous),
            StationFormat::KmPlus => parse_km_plus(&s),
            StationFormat::Rail => parse_rail(&s),
            StationFormat::Meters => parse_meters(&s),
            StationFormat::Km => parse_km(&s),
        };
        match got {
            Got::Value(value) => Ok(Station { value }),
            Got::Err(e) => Err(e),
            Got::NoMatch => Err(self.fail(
                raw,
                Msg::new("не соответствует формату {fmt}").s("fmt", self.fmt.value()),
            )),
        }
    }

    fn parse_pk_plus(&self, s: &str, previous: Option<&Station>) -> Got {
        if let Some(m) = RE_REL.captures(s) {
            let Some(prev) = previous else {
                return Got::Err(CoreError::new(
                    ErrorCode::StationParseFailed,
                    Msg::new("«{s}»: относительная запись без предыдущего пикета").s("s", s),
                ));
            };
            let pk = (prev.value / self.picket_length + 1e-9).floor();
            return match num(&m["plus"]) {
                Some(plus) => Got::Value(pk * self.picket_length + plus),
                None => Got::NoMatch,
            };
        }
        let Some(m) = RE_PK_PLUS.captures(s) else {
            return Got::NoMatch;
        };
        // '1535' без 'ПК' и без '+' неоднозначно
        if m.name("plus").is_none() && !(s.starts_with("пк") || s.starts_with("pk")) {
            return Got::NoMatch;
        }
        let (Some(mut pk), plus) = (
            num(&m["pk"]),
            m.name("plus").map_or(Some(0.0), |p| num(p.as_str())),
        ) else {
            return Got::NoMatch;
        };
        let Some(plus) = plus else {
            return Got::NoMatch;
        };
        if m.name("sign").is_some() {
            pk = -pk;
        }
        Got::Value(pk * self.picket_length + plus)
    }

    fn fail(&self, raw: &J, why: Msg) -> CoreError {
        CoreError::new(
            ErrorCode::StationParseFailed,
            Msg::new("«{raw}»: {why}")
                .s("raw", py_str(raw))
                .m("why", why),
        )
    }

    /// Запись пикета в выбранном режиме.
    pub fn format(&self, value: f64) -> String {
        let d = self.decimals;
        let w = |base: usize| if d > 0 { base + d } else { base - 1 };
        match self.fmt {
            StationFormat::PkPlus => {
                let (pk, plus) = self.split(value, self.picket_length);
                format!("ПК {pk}+{}", pyfmt::fixed_width(plus, w(3), d))
            }
            StationFormat::KmPlus => {
                let (km, plus) = self.split(value, 1000.0);
                format!("км {km}+{}", pyfmt::fixed_width(plus, w(4), d))
            }
            StationFormat::Rail => {
                let (km, rest) = self.split(value, 1000.0);
                let (pk, plus) = self.split(rest, 100.0);
                format!(
                    "км {} ПК {}+{}",
                    km + 1,
                    pk + 1,
                    pyfmt::fixed_width(plus, w(3), d)
                )
            }
            StationFormat::Km => pyfmt::fixed(value / 1000.0, d + 3),
            StationFormat::Meters => pyfmt::fixed(value, d),
        }
    }

    /// Целая часть вниз (ПК -1+50 = -50 м), остаток неотрицателен.
    /// Округление остатка не даёт `+100.00`.
    fn split(&self, value: f64, unit: f64) -> (i64, f64) {
        let q = pyfmt::round(value, self.decimals);
        let mut n = (q / unit + 1e-12).floor();
        let mut rest = pyfmt::round(q - n * unit, self.decimals);
        if rest >= unit {
            n += 1.0;
            rest = pyfmt::round(rest - unit, self.decimals);
        }
        (n as i64, rest)
    }
}

fn parse_km_plus(s: &str) -> Got {
    let Some(m) = RE_KM_PLUS.captures(s) else {
        return Got::NoMatch;
    };
    let (Some(mut km), Some(plus)) = (num(&m["km"]), num(&m["plus"])) else {
        return Got::NoMatch;
    };
    if m.name("sign").is_some() {
        km = -km;
    }
    Got::Value(km * 1000.0 + plus)
}

fn parse_rail(s: &str) -> Got {
    let Some(m) = RE_RAIL.captures(s) else {
        return Got::NoMatch;
    };
    let (Some(km), Some(pk)) = (num(&m["km"]), num(&m["pk"])) else {
        return Got::NoMatch;
    };
    if km < 1.0 || !(1.0..=10.0).contains(&pk) {
        return Got::Err(CoreError::new(
            ErrorCode::StationParseFailed,
            Msg::new("«{s}»: километр с 1, пикет от 1 до 10").s("s", s),
        ));
    }
    let plus = match m.name("plus") {
        Some(p) => match num(p.as_str()) {
            Some(v) => v,
            None => return Got::NoMatch,
        },
        None => 0.0,
    };
    Got::Value((km - 1.0) * 1000.0 + (pk - 1.0) * 100.0 + plus)
}

fn parse_meters(s: &str) -> Got {
    let s = s.replace(' ', "");
    let Some(m) = RE_NUM.captures(&s) else {
        return Got::NoMatch;
    };
    match num(&m["v"]) {
        Some(v) if m.name("sign").is_some() => Got::Value(-v),
        Some(v) => Got::Value(v),
        None => Got::NoMatch,
    }
}

fn parse_km(s: &str) -> Got {
    match parse_meters(&s.replace("км", "").replace("km", "")) {
        Got::Value(v) => Got::Value(v * 1000.0),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn p(fmt: StationFormat) -> StationParser {
        StationParser::new(fmt, 100.0).unwrap()
    }

    #[test]
    fn parse_like_the_module() {
        let pk = p(StationFormat::PkPlus);
        let v = |s: &str| pk.parse(&json!(s), None).map(|x| x.value);
        assert_eq!(v("ПК 15+35"), Ok(1535.0));
        assert_eq!(v("15+35,5"), Ok(1535.5));
        assert_eq!(v("ПК15+35.5"), Ok(1535.5));
        assert_eq!(v("ПК -1+50"), Ok(-50.0));
        assert_eq!(v("пк 15"), Ok(1500.0));
        assert_eq!(v("PK 3+99.60"), Ok(399.6));
        assert!(v("1535").is_err());
        let prev = Station { value: 1535.0 };
        assert_eq!(
            pk.parse(&json!("+40"), Some(&prev)).map(|x| x.value),
            Ok(1540.0)
        );
        let e = pk.parse(&json!("+40"), None).unwrap_err();
        assert_eq!(
            e.msg.render(&|s| s.to_string()),
            "«+40»: относительная запись без предыдущего пикета"
        );
        let e = pk.parse(&json!(1535.5), None).unwrap_err();
        assert_eq!(
            e.msg.render(&|s| s.to_string()),
            "«1535.5»: число без «+» в режиме записи с плюсом"
        );
        let km = p(StationFormat::KmPlus);
        assert_eq!(
            km.parse(&json!("км 1+535"), None).map(|x| x.value),
            Ok(1535.0)
        );
        let rail = p(StationFormat::Rail);
        assert_eq!(
            rail.parse(&json!("км 12 ПК 3+45"), None).map(|x| x.value),
            Ok(11245.0)
        );
        assert!(rail.parse(&json!("км 12 ПК 11"), None).is_err());
        let m = p(StationFormat::Meters);
        assert_eq!(
            m.parse(&json!("1 535,5"), None).map(|x| x.value),
            Ok(1535.5)
        );
        assert_eq!(m.parse(&json!(12.5), None).map(|x| x.value), Ok(12.5));
        let k = p(StationFormat::Km);
        assert_eq!(k.parse(&json!("1,535"), None).map(|x| x.value), Ok(1535.0));
    }

    #[test]
    fn format_like_the_module() {
        assert_eq!(p(StationFormat::PkPlus).format(1535.0), "ПК 15+35.00");
        assert_eq!(p(StationFormat::PkPlus).format(-50.0), "ПК -1+50.00");
        assert_eq!(p(StationFormat::PkPlus).format(1471.237), "ПК 14+71.24");
        assert_eq!(p(StationFormat::PkPlus).format(99.999), "ПК 1+00.00");
        assert_eq!(p(StationFormat::KmPlus).format(1535.0), "км 1+535.00");
        assert_eq!(p(StationFormat::Rail).format(11245.0), "км 12 ПК 3+45.00");
        assert_eq!(p(StationFormat::Km).format(1535.0), "1.53500");
        assert_eq!(p(StationFormat::Meters).format(1535.0), "1535.00");
    }
}
