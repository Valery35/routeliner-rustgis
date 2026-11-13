// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 2.0 или новее.
// SPDX-License-Identifier: GPL-2.0-or-later

//! Группа 1, подготовка: 1.02 «Проверка маршрутов» и 1.03 «Калибровка
//! маршрутов» (`processing/algorithms.py`, `CheckRoutes`,
//! `CalibrateRoutes`). 1.01 «Пример» пишет папку с GeoPackage и растром,
//! в RustGIS его нет.

use std::collections::HashMap;

use indexmap::IndexMap;
use rustgis_kit::Failure;
use serde_json::{Value as J, json};

use super::common::{self, Cx, line_geometry, pieces_m, rm, route_key};
use crate::core::chainage::{ChainageSystem, SystemName, TableRow};
use crate::core::errors::CoreError;
use crate::core::events::{EventLocator, Routes};
use crate::core::stations::StationParser;
use crate::pyfmt::Msg;

/// 1.02: маршруты в сборе и ошибки сборки.
pub fn check_routes(cx: &mut Cx<'_>) -> Result<J, Failure> {
    let (routes, errors) = common::routes(cx)?;
    let feats: Vec<J> = routes
        .iter()
        .map(|(rid, r)| {
            let gap_max = r.gaps.iter().map(|g| g.distance).fold(0.0, f64::max);
            json!({
                "values": [rid, rm(r.length), r.parts.len(), r.gaps.len(), rm(gap_max)],
                "geometry": line_geometry(&r.parts, None),
            })
        })
        .collect();
    let mut out = serde_json::Map::new();
    out.insert(
        "OUTPUT".into(),
        json!({
            "geometry": "line",
            "fields": [["route_id", "string"], ["length", "double"], ["parts", "int"],
                       ["gaps", "int"], ["gap_max", "double"]],
            "features": feats,
        }),
    );
    if cx.wants("ERRORS") {
        out.insert("ERRORS".into(), cx.errors_out(&errors, None));
    }
    cx.say(
        Msg::new("Итого: собрано {a}, ошибок {b}")
            .i("a", routes.len() as i64)
            .i("b", errors.len() as i64),
    );
    Ok(J::Object(out))
}

/// Системы по маршрутам и ошибки их построения.
type Built = (HashMap<String, ChainageSystem>, Vec<CoreError>);

/// Системы пикетажа по контрольным точкам (`control_systems`).
fn control_systems(
    cx: &mut Cx<'_>,
    routes: &Routes,
    parser: StationParser,
) -> Result<Built, Failure> {
    let Some(pts) = cx.layer("CONTROL") else {
        return Ok((HashMap::new(), Vec::new()));
    };
    let Some(sf) = cx.text("CT_STATION") else {
        return Err(cx.fail(Msg::new("Для контрольных точек нужно поле пикета")));
    };
    let rf = cx.text("CT_ROUTE");
    let radius = Some(cx.number("CT_RADIUS")).filter(|r| *r != 0.0);
    let mut plain = EventLocator::new(routes, parser);
    let mut rows: IndexMap<String, Vec<TableRow>> = IndexMap::new();
    let mut errors = Vec::new();
    for f in pts {
        let key = f["id"].as_i64();
        let (Some(x), Some(y)) = (f["xyz"][0].as_f64(), f["xyz"][1].as_f64()) else {
            continue; // пустая геометрия
        };
        let st = match parser.parse(&f["attrs"][&sf], None) {
            Ok(s) => s,
            Err(e) => {
                errors.push(e.with_key(key, None));
                continue;
            }
        };
        let only: Option<Vec<String>> = rf
            .as_ref()
            .map(|r| route_key(&f["attrs"][r]).into_iter().collect());
        match plain.locate_xy(key, x, y, only.as_deref(), radius) {
            Ok(r) => rows
                .entry(r.route_id)
                .or_default()
                .push((st.value, Some(r.m), None)),
            Err(e) => errors.push(e),
        }
    }
    let mut systems = HashMap::new();
    for (rid, rr) in rows {
        let length = routes.get(&rid).map(|r| r.length);
        match ChainageSystem::from_table(SystemName::Word("контрольные точки"), &rr, length)
        {
            Ok(s) => {
                systems.insert(rid, s);
            }
            Err(e) => errors.push(e.with_key(None, Some(&rid))),
        }
    }
    cx.say(
        Msg::new("Контрольные точки: маршрутов {a}, ошибок {b}")
            .i("a", systems.len() as i64)
            .i("b", errors.len() as i64),
    );
    Ok((systems, errors))
}

/// 1.03: маршруты с M-значениями пикетажа или меры.
pub fn calibrate_routes(cx: &mut Cx<'_>) -> Result<J, Failure> {
    let (routes, mut route_errors) = common::routes(cx)?;
    let common::Systems {
        mut loc,
        ledger_ids,
        m_ids,
        errors: lerr,
    } = common::locator(cx, &routes)?;
    let (ct, ct_err) = control_systems(cx, &routes, loc.parser)?;
    let ct_ids: Vec<String> = ct.keys().cloned().collect();
    loc.systems.extend(ct);
    let out_m = match cx.index("OUT_M") {
        0 => 2,
        v => v,
    };
    let parser = loc.parser;
    let mut feats = Vec::new();
    for (rid, r) in &routes {
        let Some(system) = loc.system_for(rid).cloned() else {
            continue;
        };
        let pieces = match r.substring_m(0.0, r.length, 0.0) {
            Ok(p) => p,
            Err(e) => {
                route_errors.push(e.with_key(None, Some(rid)));
                continue;
            }
        };
        let (cs, ms): (Vec<_>, Vec<_>) = pieces.into_iter().unzip();
        let (cs, ms) = pieces_m(cs, ms, out_m, &system);
        let source = if ct_ids.contains(rid) {
            "points"
        } else if ledger_ids.contains(rid) {
            "ledger"
        } else if m_ids.contains(rid) {
            "m"
        } else {
            "length"
        };
        let s0 = system.to_station(0.0).unwrap_or(f64::NAN);
        let s1 = system.to_station_back(r.length).unwrap_or(f64::NAN);
        feats.push(json!({
            "values": [rid, rm(r.length), rm(s0), rm(s1), parser.format(s0), parser.format(s1),
                       system.sections.len(), system.equations().len(), source],
            "geometry": line_geometry(&cs, ms.as_deref()),
        }));
    }
    let errors: Vec<CoreError> = route_errors.into_iter().chain(lerr).chain(ct_err).collect();
    let mut out = serde_json::Map::new();
    out.insert(
        "OUTPUT".into(),
        json!({
            "geometry": "line",
            "fields": [["route_id", "string"], ["length", "double"], ["st_from", "double"],
                       ["st_to", "double"], ["pk_from", "string"], ["pk_to", "string"],
                       ["sections", "int"], ["equations", "int"], ["source", "string"]],
            "features": feats,
        }),
    );
    if cx.wants("ERRORS") {
        out.insert("ERRORS".into(), cx.errors_out(&errors, None));
    }
    cx.say(
        Msg::new("Итого: маршрутов {a}, ошибок {b}")
            .i("a", routes.len() as i64)
            .i("b", errors.len() as i64),
    );
    Ok(J::Object(out))
}
