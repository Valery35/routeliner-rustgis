// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 3.0 или новее.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Общее у инструментов: значения прогона, чтение маршрутов, ведомости
//! и событий, системы пикетажа, таблица ошибок, округление полей
//! (`processing/common.py`, `layers/tables.py`, `layers/route_source.py`).

use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;
use rustgis_kit::{Failure, Lang, Tool};
use serde_json::{Value as J, json};

use crate::core::assembler::RouteAssembler;
use crate::core::chainage::{ChainageSystem, SystemName};
use crate::core::errors::{CoreError, rid_text};
use crate::core::events::{EventLocator, Routes};
use crate::core::route::{P3, P4};
use crate::core::stations::{StationFormat, StationParser, py_str};
use crate::pyfmt::{self, Msg};

/// Знаков у метров и у градусов в полях результата.
pub const DEC_M: usize = 3;
pub const DEC_DEG: usize = 2;

/// Метры до миллиметра, пустое остаётся пустым.
pub fn rm(v: f64) -> J {
    if v.is_finite() {
        json!(pyfmt::round(v, DEC_M))
    } else {
        J::Null
    }
}

/// Градусы до сотой.
pub fn rdeg(v: f64) -> J {
    if v.is_finite() {
        json!(pyfmt::round(v, DEC_DEG))
    } else {
        J::Null
    }
}

/// Значение пусто: нет значения или текст `NULL`.
pub fn is_null(v: &J) -> bool {
    v.is_null() || py_str(v) == "NULL"
}

/// Число из ячейки (`tables._num`): пусто или не число - None.
pub fn num(v: &J) -> Option<f64> {
    if is_null(v) {
        return None;
    }
    if let Some(f) = v.as_f64() {
        return Some(f);
    }
    let s = py_str(v);
    if s.trim().is_empty() {
        return None;
    }
    s.replace(',', ".").replace(' ', "").trim().parse().ok()
}

/// ID маршрута как ключ (`tables._key`): `12`, 12 и 12.0 совпадают.
pub fn route_key(v: &J) -> Option<String> {
    if is_null(v) {
        return None;
    }
    if let Some(f) = v.as_f64()
        && !v.is_i64()
        && !v.is_u64()
        && f.fract() == 0.0
        && f.is_finite()
    {
        return Some(format!("{}", f as i64));
    }
    Some(py_str(v).trim().to_string())
}

/// Расчёт одного запуска: значения, язык, сообщения журнала.
pub struct Cx<'a> {
    pub tool: &'a Tool,
    pub values: &'a J,
    pub lang: Lang,
    log: Vec<Msg>,
}

impl<'a> Cx<'a> {
    pub fn new(tool: &'a Tool, values: &'a J, lang: Lang) -> Self {
        Self {
            tool,
            values,
            lang,
            log: Vec::new(),
        }
    }

    /// Значение поля или значение по умолчанию из `tools.json`.
    pub fn raw(&self, key: &str) -> J {
        match self.values.get(key) {
            Some(v) if !v.is_null() => v.clone(),
            _ => self
                .tool
                .param(key)
                .and_then(|p| p.default.clone())
                .unwrap_or(J::Null),
        }
    }

    pub fn number(&self, key: &str) -> f64 {
        rustgis_kit::tool::number(&self.raw(key)).unwrap_or(0.0)
    }

    pub fn flag(&self, key: &str) -> bool {
        match self.raw(key) {
            J::Bool(b) => b,
            J::Number(n) => n.as_f64().is_some_and(|v| v != 0.0),
            J::String(s) => matches!(s.trim().to_lowercase().as_str(), "true" | "1" | "yes"),
            _ => false,
        }
    }

    /// Текст поля, пустой - None.
    pub fn text(&self, key: &str) -> Option<String> {
        match self.raw(key) {
            J::String(s) if !s.trim().is_empty() => Some(s),
            _ => None,
        }
    }

    pub fn index(&self, key: &str) -> usize {
        self.raw(key)
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
            .unwrap_or(0)
    }

    /// Объекты поля-слоя, слоя нет - None.
    pub fn layer(&self, key: &str) -> Option<&'a Vec<J>> {
        self.values.get(key).and_then(J::as_array)
    }

    /// Выход нужен: необязательный выход по флажку, без флажка - нужен.
    pub fn wants(&self, key: &str) -> bool {
        !matches!(self.values.get(key), Some(J::Bool(false)))
    }

    /// Строка журнала: в ответ RustGIS на языке показа.
    pub fn say(&mut self, m: Msg) {
        self.log.push(m);
    }

    /// Перевод русского шаблона на язык показа.
    pub fn tr(&self, s: &str) -> String {
        self.tool.translate(s, self.lang).to_string()
    }

    pub fn text_of(&self, m: &Msg) -> String {
        m.render(&|s| self.tr(s))
    }

    pub fn messages(&self) -> Vec<String> {
        self.log.iter().map(|m| self.text_of(m)).collect()
    }

    pub fn fail(&self, m: Msg) -> Failure {
        Failure::Run(self.text_of(&m))
    }

    /// Разбор пикетов по полям FORMAT и PICKET.
    pub fn parser(&self) -> Result<StationParser, Failure> {
        let fmt = if self.tool.param("FORMAT").is_some() {
            StationFormat::ALL
                .get(self.index("FORMAT"))
                .copied()
                .unwrap_or(StationFormat::PkPlus)
        } else {
            StationFormat::PkPlus
        };
        StationParser::new(fmt, self.number("PICKET"))
            .ok_or_else(|| Failure::Values("PICKET".into()))
    }

    /// Таблица ошибок: маршрут, код, пояснение на языке показа. `keep` -
    /// поля записи источника (ключ поля-слоя).
    /// Поля записи источника (`source`) из слоя `layer` - членом `named`:
    /// крейт кладёт их в свойства объекта итога, как модуль повторяет поля
    /// исходной записи. Поля итога с тем же именем не перекрываются.
    pub fn keep_fields(&self, feats: &mut [J], layer: &str) {
        let by_id: HashMap<i64, &J> = self
            .layer(layer)
            .into_iter()
            .flatten()
            .filter_map(|r| Some((r["id"].as_i64()?, &r["attrs"])))
            .collect();
        for f in feats {
            let Some(attrs) = f["source"].as_i64().and_then(|id| by_id.get(&id)) else {
                continue;
            };
            let named: serde_json::Map<String, J> = attrs
                .as_object()
                .into_iter()
                .flatten()
                .filter(|(k, _)| !k.starts_with("rl_"))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            f["named"] = J::Object(named);
        }
    }

    pub fn errors_out(&self, errors: &[CoreError], keep: Option<&str>) -> J {
        let mut feats: Vec<J> = errors
            .iter()
            .map(|e| {
                let mut f = json!({
                    "values": [e.route_id, e.code.as_str(), self.text_of(&e.msg)],
                    "geometry": J::Null,
                });
                if let (Some(_), Some(k)) = (keep, e.key) {
                    f["source"] = json!(k);
                }
                f
            })
            .collect();
        if let Some(layer) = keep {
            self.keep_fields(&mut feats, layer);
        }
        let mut out = json!({
            "geometry": "none",
            "fields": [["rl_route", "string"], ["rl_error", "string"], ["rl_message", "string"]],
            "features": feats,
        });
        if let Some(k) = keep {
            out["keep_source"] = json!(k);
        }
        out
    }
}

/// Вершина значения прогона `[x, y, z|null]`.
fn vertex(v: &J) -> Option<P4> {
    let a = v.as_array()?;
    Some([
        a.first()?.as_f64()?,
        a.get(1)?.as_f64()?,
        a.get(2).and_then(J::as_f64).unwrap_or(f64::NAN),
        a.get(3).and_then(J::as_f64).unwrap_or(f64::NAN),
    ])
}

/// Части объекта-линии.
pub fn parts_of(f: &J) -> Vec<Vec<P4>> {
    f["parts"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|p| {
            p.as_array()
                .into_iter()
                .flatten()
                .filter_map(vertex)
                .collect()
        })
        .collect()
}

/// Маршруты слоя ROUTES по полю ROUTE_ID и ошибки сборки.
pub fn routes(cx: &mut Cx<'_>) -> Result<(Routes, Vec<CoreError>), Failure> {
    let Some(feats) = cx.layer("ROUTES") else {
        return Err(cx.fail(Msg::new("Не задан слой маршрутов")));
    };
    let id_field = cx.text("ROUTE_ID").unwrap_or_default();
    let asm = RouteAssembler {
        tol: cx.number("SNAP"),
        allow_gaps: cx.flag("ALLOW_GAPS"),
        use_z: cx.flag("USE_Z"),
        reverse: cx.flag("REVERSE"),
    };
    let mut by_id: IndexMap<Option<String>, Vec<Vec<P4>>> = IndexMap::new();
    for f in feats {
        let rid = route_key(&f["attrs"][&id_field]);
        by_id.entry(rid).or_default().extend(parts_of(f));
    }
    let mut routes = Routes::new();
    let mut errors = Vec::new();
    for (rid, parts) in by_id {
        let Some(rid) = rid else {
            continue;
        };
        // RustGIS отдаёт слои в метрах зоны UTM: географической СК здесь нет
        match asm.build(&rid, &parts, false) {
            Ok(r) => {
                routes.insert(rid, r);
            }
            Err(e) => errors.push(e),
        }
    }
    cx.say(
        Msg::new("Маршрутов собрано {a}, не собрано {b}")
            .i("a", routes.len() as i64)
            .i("b", errors.len() as i64),
    );
    for e in &errors {
        cx.say(
            Msg::new("Маршрут {rid}: {msg}")
                .s("rid", rid_text(e.route_id.as_deref()))
                .m("msg", e.msg.clone()),
        );
    }
    Ok((routes, errors))
}

/// Системы пикетажа и их источник.
pub struct Systems<'a> {
    pub loc: EventLocator<'a>,
    /// Маршруты с системой из ведомости.
    pub ledger_ids: HashSet<String>,
    /// Маршруты с системой из M геометрии.
    pub m_ids: HashSet<String>,
    pub errors: Vec<CoreError>,
}

/// Разбор пикетов, ведомость и M маршрутов (`RoutelinerAlgorithm.locator`).
pub fn locator<'a>(cx: &mut Cx<'_>, routes: &'a Routes) -> Result<Systems<'a>, Failure> {
    let parser = cx.parser()?;
    let mut systems = HashMap::new();
    let mut errors = Vec::new();
    let mut ledger_ids = HashSet::new();
    if let Some(ledger) = cx.layer("LEDGER") {
        let (Some(route_f), Some(station_f)) = (cx.text("LG_ROUTE"), cx.text("LG_STATION")) else {
            return Err(cx.fail(Msg::new("Для ведомости нужны поля ID маршрута и пикета")));
        };
        let lengths: HashMap<&str, f64> =
            routes.iter().map(|(k, r)| (k.as_str(), r.length)).collect();
        let (sys, errs) = read_ledger(
            ledger,
            &parser,
            &route_f,
            &station_f,
            cx.text("LG_MEASURE").as_deref(),
            cx.text("LG_AHEAD").as_deref(),
            cx.text("LG_SYSTEM").as_deref(),
            cx.text("SYSTEM").as_deref(),
            &lengths,
        );
        systems = sys
            .into_iter()
            .filter(|(k, _)| routes.contains_key(k))
            .collect();
        ledger_ids = systems.keys().cloned().collect();
        cx.say(
            Msg::new("Ведомость: систем пикетажа {a}, ошибок {b}")
                .i("a", systems.len() as i64)
                .i("b", errs.len() as i64),
        );
        for e in &errs {
            cx.say(
                Msg::new("Ведомость, маршрут {rid}: {msg}")
                    .s("rid", rid_text(e.route_id.as_deref()))
                    .m("msg", e.msg.clone()),
            );
        }
        errors = errs;
    }
    let (m_sys, m_err) = m_systems(cx, routes, &ledger_ids);
    let m_ids: HashSet<String> = m_sys.keys().cloned().collect();
    systems.extend(m_sys);
    errors.extend(m_err);
    let mut loc = EventLocator::new(routes, parser);
    loc.systems = systems;
    loc.start_station = cx.number("START");
    Ok(Systems {
        loc,
        ledger_ids,
        m_ids,
        errors,
    })
}

/// Системы пикетажа из M маршрутов без ведомости (`m_systems`).
fn m_systems(
    cx: &mut Cx<'_>,
    routes: &Routes,
    skip: &HashSet<String>,
) -> (HashMap<String, ChainageSystem>, Vec<CoreError>) {
    if cx.tool.param("USE_M").is_none() || !cx.flag("USE_M") {
        return (HashMap::new(), Vec::new());
    }
    let factor = match cx.number("M_FACTOR") {
        f if f != 0.0 => f,
        _ => 1.0,
    };
    let mut out = HashMap::new();
    let mut errors = Vec::new();
    let mut plain = 0;
    for (rid, r) in routes {
        if skip.contains(rid) {
            continue;
        }
        if !r.has_m() {
            plain += 1;
            continue;
        }
        match ChainageSystem::from_measures(
            SystemName::Word("M геометрии"),
            &r.vertex_measures(),
            Some(r.length),
            factor,
        ) {
            Ok(s) => {
                out.insert(rid.clone(), s);
            }
            Err(e) => {
                cx.say(
                    Msg::new("M маршрута {rid}: {msg}")
                        .s("rid", rid.clone())
                        .m("msg", e.msg.clone()),
                );
                errors.push(e.with_key(None, Some(rid)));
            }
        }
    }
    cx.say(
        Msg::new("Пикетаж из M: маршрутов {a}, без M {b}, ошибок {c}")
            .i("a", out.len() as i64)
            .i("b", plain)
            .i("c", errors.len() as i64),
    );
    (out, errors)
}

/// Системы пикетажа по маршрутам из ведомости (`layers/tables.read_ledger`).
#[allow(clippy::too_many_arguments)]
pub fn read_ledger(
    rows: &[J],
    parser: &StationParser,
    route_field: &str,
    station_field: &str,
    measure_field: Option<&str>,
    ahead_field: Option<&str>,
    system_field: Option<&str>,
    system_name: Option<&str>,
    route_lengths: &HashMap<&str, f64>,
) -> (IndexMap<String, ChainageSystem>, Vec<CoreError>) {
    type Row = (f64, Option<f64>, Option<f64>);
    let mut by_rid: IndexMap<Option<String>, Vec<Row>> = IndexMap::new();
    let mut errors = Vec::new();
    for f in rows {
        let a = &f["attrs"];
        let key = f["id"].as_i64();
        if let (Some(sf), Some(name)) = (system_field, system_name)
            && py_str(&a[sf]).trim() != name
        {
            continue;
        }
        let rid = route_key(&a[route_field]);
        let st = match parser.parse(&a[station_field], None) {
            Ok(s) => s,
            Err(e) => {
                errors.push(e.with_key(key, rid.as_deref()));
                continue;
            }
        };
        let mut ahead = None;
        if let Some(af) = ahead_field
            && !is_null(&a[af])
            && !py_str(&a[af]).trim().is_empty()
        {
            match parser.parse(&a[af], None) {
                Ok(v) => ahead = Some(v.value),
                Err(e) => {
                    errors.push(e.with_key(key, rid.as_deref()));
                    continue;
                }
            }
        }
        let m = measure_field.and_then(|mf| num(&a[mf]));
        by_rid.entry(rid).or_default().push((st.value, m, ahead));
    }
    let mut systems = IndexMap::new();
    for (rid, rr) in by_rid {
        let length = rid.as_deref().and_then(|r| route_lengths.get(r)).copied();
        let name = SystemName::Text(system_name.unwrap_or("ведомость").to_string());
        match ChainageSystem::from_table(name, &rr, length) {
            Ok(s) => {
                if let Some(r) = rid {
                    systems.insert(r, s);
                }
            }
            Err(e) => errors.push(e.with_key(None, rid.as_deref())),
        }
    }
    (systems, errors)
}

/// Геометрия линии выхода: один кусок - `{"points"}`, несколько -
/// `{"parts"}`. Высоты нет: выходы модуля пишутся без Z. `ms` -
/// M-значения вершин (`OUT_M`), они идут рядом членом `measure`.
pub fn line_geometry(pieces: &[Vec<P3>], ms: Option<&[Vec<f64>]>) -> J {
    let pts: Vec<Vec<J>> = pieces
        .iter()
        .map(|c| c.iter().map(|p| json!([p[0], p[1], J::Null])).collect())
        .collect();
    let mut g = if pts.len() == 1 {
        json!({"points": pts[0]})
    } else {
        json!({"parts": pts})
    };
    if let Some(ms) = ms {
        g["measure"] = if ms.len() == 1 {
            json!(ms[0])
        } else {
            json!(ms)
        };
    }
    g
}

/// Куски участка и M их вершин (`_pieces_m`). Для пикетажа (2) внутрь
/// куска вставляются вершины уравнений, чтобы M скачком повторял пикетаж.
pub fn pieces_m(
    pieces: Vec<Vec<P3>>,
    piece_m: Vec<Vec<f64>>,
    out_m: usize,
    system: &ChainageSystem,
) -> (Vec<Vec<P3>>, Option<Vec<Vec<f64>>>) {
    match out_m {
        0 => (pieces, None),
        1 => (pieces, Some(piece_m)),
        _ => {
            let mut cs = Vec::new();
            let mut ms = Vec::new();
            for (c, m) in pieces.iter().zip(&piece_m) {
                let (c2, _, st) = system.stations_along(c, m);
                cs.push(c2);
                ms.push(st);
            }
            (cs, Some(ms))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_and_numbers_like_the_module() {
        assert_eq!(route_key(&json!("R1 ")), Some("R1".into()));
        assert_eq!(route_key(&json!(12)), Some("12".into()));
        assert_eq!(route_key(&json!(12.0)), Some("12".into()));
        assert_eq!(route_key(&json!(12.5)), Some("12.5".into()));
        assert_eq!(route_key(&J::Null), None);
        assert_eq!(num(&json!("1 535,5")), Some(1535.5));
        assert_eq!(num(&json!("")), None);
        assert_eq!(num(&json!(400.0)), Some(400.0));
        assert_eq!(rm(1471.2365), json!(1471.236));
    }
}
