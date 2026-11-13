// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 3.0 или новее.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Группы 2 и 4: 2.01 «Точечные события», 2.02 «Участки (линейные
//! события)», 4.01 «Пикетная разбивка», 4.02 «Привязка точек к маршрутам»
//! (`processing/algorithms.py`, `_EventsBase`, `Pickets`, `LocatePoints`).
//!
//! Итог событий и привязки - объекты источника: поля записи берёт крейт
//! (`keep_source`), расчёт добавляет поля `rl_*`.

use std::collections::HashMap;

use rustgis_kit::Failure;
use serde_json::{Value as J, json};

use super::common::{self, Cx, line_geometry, num, pieces_m, rdeg, rm, route_key};
use crate::core::errors::CoreError;
use crate::core::events::EventRecord;
use crate::pyfmt::Msg;

/// Записи событий по полям EV_* (`layers/tables.read_events`).
fn read_events(cx: &Cx<'_>, line: bool) -> Result<Vec<EventRecord>, Failure> {
    let Some(rows) = cx.layer("EVENTS") else {
        return Err(Failure::Values("EVENTS".into()));
    };
    let route_f = cx.text("EV_ROUTE").unwrap_or_default();
    let from_f = cx.text("EV_FROM").unwrap_or_default();
    let to_f = if line { cx.text("EV_TO") } else { None };
    let off_f = cx.text("EV_OFFSET");
    let sec_f = cx.text("EV_SECTION");
    let sign = if cx.flag("OFFSET_RIGHT") { -1.0 } else { 1.0 };
    let raw = |v: &J| {
        if common::is_null(v) {
            J::Null
        } else {
            v.clone()
        }
    };
    Ok(rows
        .iter()
        .map(|f| {
            let a = &f["attrs"];
            let off = off_f.as_ref().and_then(|k| num(&a[k]));
            let sec = sec_f.as_ref().and_then(|k| num(&a[k]));
            EventRecord {
                key: f["id"].as_i64(),
                route_id: route_key(&a[&route_f]),
                raw_from: raw(&a[&from_f]),
                raw_to: to_f.as_ref().map_or(J::Null, |k| raw(&a[k])),
                offset: sign * off.unwrap_or(0.0),
                section: sec.map(|s| s.trunc() as i64),
            }
        })
        .collect())
}

/// 2.01 и 2.02: события на маршрутах и ошибки постановки.
pub fn events(cx: &mut Cx<'_>, line: bool) -> Result<J, Failure> {
    let (routes, route_errors) = common::routes(cx)?;
    let mut sys = common::locator(cx, &routes)?;
    let records = read_events(cx, line)?;
    // маршрут не собран - своё пояснение вместо «не найден»
    let broken: HashMap<String, &CoreError> = route_errors
        .iter()
        .filter_map(|e| e.route_id.clone().map(|r| (r, e)))
        .collect();
    let out_m = cx.index("OUT_M");
    let parser = sys.loc.parser;
    let (feats, bad, placed): (Vec<J>, Vec<CoreError>, usize) = if line {
        let (ok, bad) = sys.loc.locate_lines(&records);
        let feats = ok
            .into_iter()
            .map(|r| {
                let system = sys.loc.system_for(&r.route_id).cloned();
                let (cs, ms) = match &system {
                    Some(s) => pieces_m(r.pieces, r.piece_m, out_m, s),
                    None => (r.pieces, None),
                };
                json!({
                    "source": r.key,
                    "values": [rm(r.m_from), rm(r.m_to), rm(r.m_to - r.m_from),
                               parser.format(r.st_from), parser.format(r.st_to), i64::from(r.swapped)],
                    "geometry": line_geometry(&cs, ms.as_deref()),
                })
            })
            .collect::<Vec<_>>();
        let n = feats.len();
        (feats, bad, n)
    } else {
        let (ok, bad) = sys.loc.locate_points(&records);
        let feats = ok
            .iter()
            .map(|r| {
                let mut g = json!({"xyz": [r.x, r.y, J::Null]});
                if out_m > 0 {
                    g["measure"] = json!(if out_m == 1 { r.m } else { r.station });
                }
                json!({
                    "source": r.key,
                    "values": [rm(r.m), parser.format(r.station), rm(r.x), rm(r.y), rdeg(r.azimuth)],
                    "geometry": g,
                })
            })
            .collect::<Vec<_>>();
        let n = feats.len();
        (feats, bad, n)
    };
    let bad: Vec<CoreError> = bad
        .into_iter()
        .map(|e| match e.route_id.as_ref().and_then(|r| broken.get(r)) {
            Some(b) => CoreError {
                msg: Msg::new("маршрут не собран: {msg}").m("msg", b.msg.clone()),
                ..e
            },
            None => e,
        })
        .collect();
    let fields = if line {
        json!([
            ["rl_m_from", "double"],
            ["rl_m_to", "double"],
            ["rl_length", "double"],
            ["rl_pk_from", "string"],
            ["rl_pk_to", "string"],
            ["rl_swapped", "int"]
        ])
    } else {
        json!([
            ["rl_m", "double"],
            ["rl_pk", "string"],
            ["rl_x", "double"],
            ["rl_y", "double"],
            ["rl_azimuth", "double"]
        ])
    };
    let mut out = serde_json::Map::new();
    out.insert(
        "OUTPUT".into(),
        json!({
            "keep_source": "EVENTS",
            "geometry": if line { "line" } else { "point" },
            "fields": fields,
            "features": feats,
        }),
    );
    if cx.wants("ERRORS") {
        out.insert("ERRORS".into(), cx.errors_out(&bad, Some("EVENTS")));
    }
    cx.say(
        Msg::new("Итого: поставлено {a} из {n}, ошибок {b}")
            .i("a", placed as i64)
            .i("n", records.len() as i64)
            .i("b", bad.len() as i64),
    );
    Ok(J::Object(out))
}

/// 4.01: точки целых пикетов по каждому маршруту.
pub fn pickets(cx: &mut Cx<'_>) -> Result<J, Failure> {
    let (routes, _) = common::routes(cx)?;
    let mut sys = common::locator(cx, &routes)?;
    let step = cx.number("STEP");
    if step <= 0.0 {
        return Err(Failure::Values("STEP".into()));
    }
    let out_m = cx.index("OUT_M");
    let parser = sys.loc.parser;
    let mut feats = Vec::new();
    for (rid, r) in &routes {
        let Some(items) = sys.loc.system_for(rid).map(|s| s.whole_stations(step)) else {
            continue;
        };
        if items.is_empty() {
            continue;
        }
        let ms: Vec<f64> = items.iter().map(|(m, _, _)| *m).collect();
        let at = r.points_at(&ms, &[0.0]);
        for ((m, st, sec), p) in items.into_iter().zip(at) {
            let mut g = json!({"xyz": [p.x, p.y, J::Null]});
            if out_m > 0 {
                g["measure"] = json!(if out_m == 1 { m } else { st });
            }
            feats.push(json!({
                "values": [rid, parser.format(st), rm(st), rm(m), sec, rdeg(p.azimuth),
                           i64::from(st.abs() % 1000.0 < 1e-6)],
                "geometry": g,
            }));
        }
    }
    cx.say(Msg::new("Итого: пикетов {n}").i("n", feats.len() as i64));
    Ok(json!({"OUTPUT": {
        "geometry": "point",
        "fields": [["route_id", "string"], ["pk", "string"], ["station", "double"], ["m", "double"],
                   ["section", "int"], ["azimuth", "double"], ["km", "int"]],
        "features": feats,
    }}))
}

/// 4.02: ближайший маршрут, мера, пикет и смещение каждой точки.
pub fn locate_points(cx: &mut Cx<'_>) -> Result<J, Failure> {
    let (routes, _) = common::routes(cx)?;
    let mut sys = common::locator(cx, &routes)?;
    let Some(pts) = cx.layer("POINTS") else {
        return Err(Failure::Values("POINTS".into()));
    };
    let rf = cx.text("PT_ROUTE");
    let radius = Some(cx.number("RADIUS")).filter(|r| *r != 0.0);
    let parser = sys.loc.parser;
    let mut feats = Vec::new();
    let mut errors = Vec::new();
    for f in pts {
        let key = f["id"].as_i64();
        let (Some(x), Some(y)) = (f["xyz"][0].as_f64(), f["xyz"][1].as_f64()) else {
            continue; // пустая геометрия
        };
        let only: Option<Vec<String>> = rf
            .as_ref()
            .map(|r| route_key(&f["attrs"][r]).into_iter().collect());
        match sys.loc.locate_xy(key, x, y, only.as_deref(), radius) {
            Ok(r) => {
                // сторона - код, а не слово: значение не зависит от языка
                let off = rm(r.offset);
                let o = off.as_f64().unwrap_or(0.0);
                let side = if o > 0.0 {
                    "left"
                } else if o < 0.0 {
                    "right"
                } else {
                    "axis"
                };
                feats.push(json!({
                    "source": key,
                    "values": [r.route_id, rm(r.m), parser.format(r.station), off, side],
                    "geometry": {"source": key},
                }));
            }
            Err(e) => errors.push(e),
        }
    }
    let mut out = serde_json::Map::new();
    out.insert(
        "OUTPUT".into(),
        json!({
            "keep_source": "POINTS",
            "geometry": "source",
            "fields": [["rl_route", "string"], ["rl_m", "double"], ["rl_pk", "string"],
                       ["rl_offset", "double"], ["rl_side", "string"]],
            "features": feats,
        }),
    );
    if cx.wants("ERRORS") {
        out.insert("ERRORS".into(), cx.errors_out(&errors, Some("POINTS")));
    }
    cx.say(
        Msg::new("Итого: привязано {a}, ошибок {b}")
            .i("a", feats.len() as i64)
            .i("b", errors.len() as i64),
    );
    Ok(J::Object(out))
}
