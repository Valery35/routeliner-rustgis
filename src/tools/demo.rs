// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 3.0 или новее.
// SPDX-License-Identifier: GPL-3.0-or-later

//! 1.01 «Демонстрационный пример» (`processing/alg_demo.py`).
//!
//! Модуль пишет пример в GeoPackage с растром рельефа и сразу прогоняет
//! его через свои инструменты. У расширений RustGIS нет поля папки,
//! поэтому пример уходит в проект слоями и таблицами: маршруты, дефекты,
//! ведомость, таблицы точечных и линейных событий. Самопроверка идёт тем
//! же порядком, что у модуля, теми же инструментами расширения, и её
//! итоги ложатся в проект рядом. Растр рельефа в проект не идёт, растров
//! у расширений нет. Для проверки профиля он строится в памяти, как
//! GeoTIFF модуля, и снимается билинейно, как растр в QGIS.

use std::collections::HashMap;
use std::sync::LazyLock;

use rustgis_kit::{Catalog, Failure, Lang};
use serde_json::{Map, Value as J, json};

use super::common::{Cx, rm};
use crate::core::demo::{self, Dem, Demo};
use crate::core::route::P3;
use crate::pyfmt::Msg;

/// Допуск сравнения с эталоном, м.
const TOL: f64 = 0.01;
/// Имя поля рельефа в таблице профиля: `z_` и имя растра модуля.
pub const ZF: &str = "z_routeliner_demo_dem";

static CATALOG: LazyLock<Result<Catalog, String>> =
    LazyLock::new(|| Catalog::parse(crate::product::TOOLS_JSON));

/// Имя слоя на языке показа: имена, которых нет в сообщениях 1.01.
fn layer_name(cx: &Cx<'_>, ru: &str) -> String {
    let en = match ru {
        "Маршруты" => "Routes",
        "Дефекты" => "Defects",
        "Ведомость" => "Ledger",
        "Точечные события" => "Point events",
        "Линейные события" => "Line events",
        "Профиль, линии" => "Profile, lines",
        "Профиль, подписи" => "Profile, labels",
        _ => return cx.tr(ru),
    };
    match cx.lang {
        Lang::Ru => ru.to_string(),
        Lang::En => en.to_string(),
    }
}

/// Число записи, как его пишет модуль: до миллиметра.
fn r3(v: f64) -> J {
    rm(v)
}

fn opt3(v: Option<f64>) -> J {
    v.map_or(J::Null, r3)
}

/// Строка таблицы: поля по порядку.
type Row = Vec<(&'static str, J)>;

/// Слой примера: значения прогона и выход для RustGIS.
struct Layer {
    /// `[{"id", "parts"|"xyz", "attrs"}]` - вход инструментов.
    value: J,
    /// `@КЛЮЧ` - сведения о слое.
    info: J,
    /// Выход `{"name", "geometry", "fields", "features"}`.
    out: J,
}

/// Слой из строк: `geom` - геометрия выхода каждой строки, `input` -
/// геометрия входа (`parts` или `xyz`).
fn layer(name: &str, kind: &str, rows: &[Row], geoms: &[(J, J)], types: &[&str]) -> Layer {
    let fields: Vec<&str> = rows
        .first()
        .map_or_else(Vec::new, |r| r.iter().map(|c| c.0).collect());
    let numeric: Vec<&str> = fields
        .iter()
        .zip(types)
        .filter(|(_, t)| **t != "string")
        .map(|(f, _)| *f)
        .collect();
    let mut value = Vec::new();
    let mut feats = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let attrs: Map<String, J> = row
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect();
        let mut v = json!({"id": i as i64 + 1, "attrs": attrs});
        let (input, output) = geoms.get(i).cloned().unwrap_or((J::Null, J::Null));
        if kind == "line" {
            v["parts"] = input;
        } else {
            v["xyz"] = input;
        }
        value.push(v);
        let vals: Vec<J> = row.iter().map(|c| c.1.clone()).collect();
        feats.push(json!({"values": vals, "geometry": output}));
    }
    let hasz = kind == "line";
    Layer {
        value: J::Array(value),
        info: json!({"name": name, "geometry": kind, "hasz": hasz, "fields": fields, "numeric": numeric}),
        out: json!({
            "name": name,
            "geometry": kind,
            "fields": fields.iter().zip(types).map(|(f, t)| json!([f, t])).collect::<Vec<_>>(),
            "features": feats,
        }),
    }
}

fn xyz(p: &P3) -> J {
    json!([
        p[0],
        p[1],
        if p[2].is_finite() {
            json!(p[2])
        } else {
            J::Null
        }
    ])
}

/// Слои примера.
struct Layers {
    routes: Layer,
    ledger: Layer,
    points: Layer,
    lines: Layer,
    defects: Layer,
}

fn layers(cx: &Cx<'_>, d: &Demo) -> Layers {
    let rows: Vec<Row> = d
        .routes
        .iter()
        .map(|r| {
            vec![
                ("route_id", json!(r.route_id)),
                ("name", json!(cx.tr(r.name))),
            ]
        })
        .collect();
    let geoms: Vec<(J, J)> = d
        .routes
        .iter()
        .map(|r| {
            let input: Vec<Vec<J>> = r
                .parts
                .iter()
                .map(|p| p.iter().map(xyz).collect())
                .collect();
            // у слоя RustGIS высот нет: выход без Z
            let plan: Vec<Vec<J>> = r
                .parts
                .iter()
                .map(|p| p.iter().map(|v| json!([v[0], v[1], J::Null])).collect())
                .collect();
            (json!(input), json!({"parts": plan}))
        })
        .collect();
    let routes = layer(
        &layer_name(cx, "Маршруты"),
        "line",
        &rows,
        &geoms,
        &["string", "string"],
    );

    let rows: Vec<Row> = d
        .ledger
        .iter()
        .map(|l| {
            vec![
                ("route_id", json!(l.route_id)),
                ("system", json!(l.system)),
                ("station", json!(l.station)),
                ("measure", r3(l.measure)),
                (
                    "station_ahead",
                    l.station_ahead.map_or(J::Null, |s| json!(s)),
                ),
                ("note", json!(cx.tr(l.note))),
            ]
        })
        .collect();
    let ledger = layer(
        &layer_name(cx, "Ведомость"),
        "none",
        &rows,
        &[],
        &["string", "string", "string", "double", "string", "string"],
    );

    let rows: Vec<Row> = d
        .points
        .iter()
        .map(|p| {
            vec![
                ("eid", json!(p.eid)),
                ("route_id", json!(p.route_id)),
                ("pk", json!(p.pk)),
                ("offset", r3(p.offset)),
                ("section", p.section.map_or(J::Null, |s| json!(s))),
                ("exp_m", opt3(p.exp_m)),
                ("exp_x", opt3(p.exp_x)),
                ("exp_y", opt3(p.exp_y)),
                ("exp_error", json!(p.exp_error)),
                (
                    "note",
                    json!(if p.note.is_empty() {
                        String::new()
                    } else {
                        cx.tr(p.note)
                    }),
                ),
            ]
        })
        .collect();
    let points = layer(
        &layer_name(cx, "Точечные события"),
        "none",
        &rows,
        &[],
        &[
            "int", "string", "string", "double", "int", "double", "double", "double", "string",
            "string",
        ],
    );

    let rows: Vec<Row> = d
        .lines
        .iter()
        .map(|l| {
            vec![
                ("eid", json!(l.eid)),
                ("route_id", json!(l.route_id)),
                ("pk_from", json!(l.pk_from)),
                ("pk_to", json!(l.pk_to)),
                ("offset", r3(l.offset)),
                ("exp_m_from", opt3(l.exp_m_from)),
                ("exp_m_to", opt3(l.exp_m_to)),
                ("exp_error", json!(l.exp_error)),
                ("note", json!(cx.tr(l.note))),
            ]
        })
        .collect();
    let lines = layer(
        &layer_name(cx, "Линейные события"),
        "none",
        &rows,
        &[],
        &[
            "int", "string", "string", "string", "double", "double", "double", "string", "string",
        ],
    );

    let rows: Vec<Row> = d
        .defects
        .iter()
        .map(|f| {
            vec![
                ("did", json!(f.did)),
                ("exp_route", json!(f.exp_route)),
                ("exp_m", r3(f.exp_m)),
                ("exp_station", r3(f.exp_station)),
                ("exp_offset", r3(f.exp_offset)),
            ]
        })
        .collect();
    let geoms: Vec<(J, J)> = d
        .defects
        .iter()
        .map(|f| {
            (
                json!([f.x, f.y, J::Null]),
                json!({"xyz": [f.x, f.y, J::Null]}),
            )
        })
        .collect();
    let defects = layer(
        &layer_name(cx, "Дефекты"),
        "point",
        &rows,
        &geoms,
        &["int", "string", "double", "double", "double"],
    );
    Layers {
        routes,
        ledger,
        points,
        lines,
        defects,
    }
}

/// 1.01: пример слоями и, по флажку RUN, самопроверка.
pub fn demo(cx: &mut Cx<'_>) -> Result<J, Failure> {
    let n = (cx.number("N_POINTS").round() as i64).clamp(3, 200_000) as usize;
    let seed = cx.number("SEED").round() as i64;
    let d = demo::build(n, (n / 2).max(30), seed);
    let l = layers(cx, &d);
    let mut out = Map::new();
    out.insert("ROUTES".into(), l.routes.out.clone());
    out.insert("DEFECTS".into(), l.defects.out.clone());
    out.insert("LEDGER".into(), l.ledger.out.clone());
    out.insert("EVENTS_POINTS".into(), l.points.out.clone());
    out.insert("EVENTS_LINES".into(), l.lines.out.clone());
    cx.say(
        Msg::new("Событий: точечных {a}, линейных {b}, дефектов {c}")
            .i("a", d.points.len() as i64)
            .i("b", d.lines.len() as i64)
            .i("c", d.defects.len() as i64),
    );
    if cx.flag("RUN") {
        check(cx, &d, &l, &mut out)?;
    }
    Ok(J::Object(out))
}

// ------------------------------------------------------------ проверка
/// Запуск инструмента расширения по имени Processing.
fn run(cx: &Cx<'_>, name: &str, values: J) -> Result<J, Failure> {
    let cat = CATALOG.as_ref().map_err(|e| Failure::Run(e.clone()))?;
    let tool = cat
        .tool(name)
        .ok_or_else(|| Failure::Run(name.to_string()))?;
    let mut sub = Cx::new(tool, &values, cx.lang);
    if name == "profile_drawing" {
        let (out, width) = super::profile::drawing(&mut sub)?;
        return Ok(json!({"out": out, "width": width}));
    }
    let (out, _) = super::run(tool, &values, cx.lang)?;
    Ok(out)
}

/// Общие значения прогонов самопроверки (`common` в `alg_demo.py`).
fn common(l: &Layers) -> Map<String, J> {
    let mut v = Map::new();
    v.insert("ROUTES".into(), l.routes.value.clone());
    v.insert("@ROUTES".into(), l.routes.info.clone());
    v.insert("ROUTE_ID".into(), json!("route_id"));
    v.insert("SNAP".into(), json!(0.01));
    v.insert("FORMAT".into(), json!(0));
    v.insert("PICKET".into(), json!(100.0));
    v.insert("START".into(), json!(0.0));
    v.insert("LEDGER".into(), l.ledger.value.clone());
    v.insert("@LEDGER".into(), l.ledger.info.clone());
    for (k, f) in [
        ("LG_ROUTE", "route_id"),
        ("LG_STATION", "station"),
        ("LG_MEASURE", "measure"),
        ("LG_AHEAD", "station_ahead"),
        ("LG_SYSTEM", "system"),
    ] {
        v.insert(k.into(), json!(f));
    }
    v.insert("SYSTEM".into(), json!(demo::SYSTEM));
    v
}

fn with(base: &Map<String, J>, extra: &[(&str, J)]) -> J {
    let mut v = base.clone();
    for (k, x) in extra {
        v.insert((*k).to_string(), x.clone());
    }
    J::Object(v)
}

/// Имена полей выхода.
fn field_names(o: &J) -> Vec<String> {
    o["fields"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|f| f[0].as_str().unwrap_or("").to_string())
        .collect()
}

/// Объекты выхода: поля по имени и номер записи источника.
fn records(o: &J) -> Vec<(Map<String, J>, Option<i64>, J)> {
    let names = field_names(o);
    o["features"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|f| {
            let m: Map<String, J> = names
                .iter()
                .cloned()
                .zip(f["values"].as_array().cloned().unwrap_or_default())
                .collect();
            (m, f["source"].as_i64(), f["geometry"].clone())
        })
        .collect()
}

fn n(v: &J) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

/// Выход с `keep_source` - в самостоятельный слой: поля записи источника
/// впереди, геометрия источника - точкой (у самопроверки нет объектов
/// входа RustGIS, крейт их не подставит).
fn materialize(o: &J, src: &Layer, name: String) -> J {
    let mut out = o.clone();
    out["name"] = json!(name);
    let Some(obj) = out.as_object_mut() else {
        return out;
    };
    if obj.remove("keep_source").is_none() {
        return out;
    }
    let src_fields: Vec<J> = src.out["fields"].as_array().cloned().unwrap_or_default();
    let by_id: HashMap<i64, &J> = src
        .value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| Some((f["id"].as_i64()?, f)))
        .collect();
    let names: Vec<String> = src_fields
        .iter()
        .map(|f| f[0].as_str().unwrap_or("").to_string())
        .collect();
    let mut fields = src_fields.clone();
    fields.extend(o["fields"].as_array().cloned().unwrap_or_default());
    let feats: Vec<J> = o["features"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|f| {
            let s = f["source"].as_i64().and_then(|id| by_id.get(&id));
            let mut vals: Vec<J> = names
                .iter()
                .map(|k| s.map_or(J::Null, |s| s["attrs"][k].clone()))
                .collect();
            vals.extend(f["values"].as_array().cloned().unwrap_or_default());
            let geom = if f["geometry"].get("source").is_some() {
                s.map_or(J::Null, |s| json!({"xyz": s["xyz"]}))
            } else {
                f["geometry"].clone()
            };
            json!({"values": vals, "geometry": geom})
        })
        .collect();
    out["fields"] = json!(fields);
    out["features"] = json!(feats);
    if out["geometry"] == "source" {
        out["geometry"] = json!("point");
    }
    out
}

/// Выход-линии 1.03 - во вход инструмента: части с M вершин.
fn as_routes(o: &J) -> J {
    let names = field_names(o);
    let feats: Vec<J> = o["features"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(i, f)| {
            let g = &f["geometry"];
            let (parts, ms): (Vec<J>, Vec<J>) = if let Some(p) = g.get("points") {
                (vec![p.clone()], vec![g["measure"].clone()])
            } else {
                let ms = g["measure"].as_array().cloned().unwrap_or_default();
                (g["parts"].as_array().cloned().unwrap_or_default(), ms)
            };
            let parts: Vec<Vec<J>> = parts
                .iter()
                .zip(ms.iter().chain(std::iter::repeat(&J::Null)))
                .map(|(p, m)| {
                    p.as_array()
                        .into_iter()
                        .flatten()
                        .enumerate()
                        .map(|(k, v)| {
                            json!([v[0], v[1], v[2], m.get(k).cloned().unwrap_or(J::Null)])
                        })
                        .collect()
                })
                .collect();
            let attrs: Map<String, J> = names
                .iter()
                .cloned()
                .zip(f["values"].as_array().cloned().unwrap_or_default())
                .collect();
            json!({"id": i as i64 + 1, "parts": parts, "attrs": attrs})
        })
        .collect();
    J::Array(feats)
}

/// Выход-точки - во вход инструмента (`xyz` и поля).
fn as_points(o: &J) -> (J, J) {
    let names = field_names(o);
    let feats: Vec<J> = o["features"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(i, f)| {
            let attrs: Map<String, J> = names
                .iter()
                .cloned()
                .zip(f["values"].as_array().cloned().unwrap_or_default())
                .collect();
            json!({"id": i as i64 + 1, "xyz": f["geometry"]["xyz"], "attrs": attrs})
        })
        .collect();
    let numeric: Vec<&String> = names
        .iter()
        .zip(o["fields"].as_array().into_iter().flatten())
        .filter(|(_, f)| f[1] != "string")
        .map(|(n, _)| n)
        .collect();
    (
        J::Array(feats),
        json!({"name": o["name"], "geometry": "point", "hasz": false, "fields": names, "numeric": numeric}),
    )
}

fn yes_no(ok: bool) -> Msg {
    Msg::new(if ok { "да" } else { "нет" })
}

/// Самопроверка: прогон примера инструментами расширения и сверка
/// с эталоном (`DemoAlgorithm._check`).
#[allow(clippy::too_many_lines)]
fn check(cx: &mut Cx<'_>, d: &Demo, l: &Layers, out: &mut Map<String, J>) -> Result<(), Failure> {
    let base = common(l);
    let mut add = |key: &str, o: J| {
        out.insert(key.to_string(), o);
    };

    // маршруты
    let res = run(cx, "check_routes", with(&base, &[("ERRORS", json!(true))]))?;
    let bad: Vec<String> = records(&res["ERRORS"])
        .iter()
        .map(|r| r.0["rl_route"].as_str().unwrap_or("").to_string())
        .collect();
    let ok_routes = bad == ["R4"];
    add(
        "CHECK_ROUTES",
        materialize(&res["OUTPUT"], &l.routes, cx.tr("Собранные маршруты")),
    );

    // точечные события
    let ev = [
        ("EVENTS", l.points.value.clone()),
        ("@EVENTS", l.points.info.clone()),
        ("EV_ROUTE", json!("route_id")),
        ("EV_FROM", json!("pk")),
        ("EV_OFFSET", json!("offset")),
        ("EV_SECTION", json!("section")),
        ("ERRORS", json!(true)),
    ];
    let res = run(cx, "point_events", with(&base, &ev))?;
    let pts: HashMap<i64, &demo::PointEvent> = d.points.iter().map(|p| (p.eid, p)).collect();
    let (mut p_ok, mut p_total, mut worst) = (0, 0, 0.0f64);
    for (f, src, _) in records(&res["OUTPUT"]) {
        let Some(p) = src.and_then(|s| pts.get(&s)) else {
            continue;
        };
        if !p.exp_error.is_empty() {
            continue;
        }
        p_total += 1;
        let ex = n(&opt3(p.exp_x));
        let ey = n(&opt3(p.exp_y));
        let dxy = (n(&f["rl_x"]) - ex).hypot(n(&f["rl_y"]) - ey);
        worst = worst.max(dxy);
        p_ok += i32::from(dxy <= TOL);
    }
    let exp_err = d.points.iter().filter(|p| !p.exp_error.is_empty()).count();
    let e_ok = records(&res["ERRORS"])
        .iter()
        .filter(|(f, src, _)| {
            src.and_then(|s| pts.get(&s))
                .is_some_and(|p| f["rl_error"] == p.exp_error)
        })
        .count();
    let p_total_all = d.points.len() - exp_err;
    add(
        "POINTS_OUT",
        materialize(
            &res["OUTPUT"],
            &l.points,
            cx.tr("Точечные события, результат"),
        ),
    );
    add(
        "POINTS_ERR",
        materialize(&res["ERRORS"], &l.points, cx.tr("Точечные события, ошибки")),
    );

    // калибровка: пикетаж ведомости уходит в M, и те же события ставятся
    // по M без ведомости. Записи с номером участка пропускаются: участки
    // пикетажа из M нумеруются по изломам масштаба, а не по реперам.
    let res = run(
        cx,
        "calibrate_routes",
        with(&base, &[("OUT_M", json!(2)), ("ERRORS", json!(true))]),
    )?;
    let got: HashMap<String, (String, i64)> = records(&res["OUTPUT"])
        .iter()
        .map(|(f, _, _)| {
            (
                f["route_id"].as_str().unwrap_or("").to_string(),
                (
                    f["source"].as_str().unwrap_or("").to_string(),
                    f["equations"].as_i64().unwrap_or(-1),
                ),
            )
        })
        .collect();
    let want: HashMap<String, (String, i64)> = [
        ("R1", ("length", 0)),
        ("R2", ("length", 0)),
        ("R3", ("ledger", 2)),
    ]
    .iter()
    .map(|(r, (s, e))| ((*r).to_string(), ((*s).to_string(), *e)))
    .collect();
    let cal_ok = got == want;
    let cal = as_routes(&res["OUTPUT"]);
    add("CALIBRATED", {
        let mut o = res["OUTPUT"].clone();
        o["name"] = json!(cx.tr("Калиброванные маршруты"));
        o
    });
    let mut m_base = base.clone();
    m_base.remove("LEDGER");
    m_base.remove("@LEDGER");
    m_base.insert("ROUTES".into(), cal);
    let m_ev = [
        ("USE_M", json!(true)),
        ("EVENTS", l.points.value.clone()),
        ("@EVENTS", l.points.info.clone()),
        ("EV_ROUTE", json!("route_id")),
        ("EV_FROM", json!("pk")),
        ("EV_OFFSET", json!("offset")),
        ("ERRORS", json!(true)),
    ];
    let res = run(cx, "point_events", with(&m_base, &m_ev))?;
    let plain = |p: &demo::PointEvent| p.exp_error.is_empty() && p.section.is_none();
    let (mut m_ok, mut m_total) = (0, 0);
    for (f, src, _) in records(&res["OUTPUT"]) {
        let Some(p) = src.and_then(|s| pts.get(&s)) else {
            continue;
        };
        if !plain(p) {
            continue;
        }
        m_total += 1;
        let dxy = (n(&f["rl_x"]) - n(&opt3(p.exp_x))).hypot(n(&f["rl_y"]) - n(&opt3(p.exp_y)));
        m_ok += i32::from(dxy <= TOL);
    }
    let m_exp = d.points.iter().filter(|p| plain(p)).count() as i32;

    // линейные события
    let le = [
        ("EVENTS", l.lines.value.clone()),
        ("@EVENTS", l.lines.info.clone()),
        ("EV_ROUTE", json!("route_id")),
        ("EV_FROM", json!("pk_from")),
        ("EV_TO", json!("pk_to")),
        ("EV_OFFSET", json!("offset")),
        ("ERRORS", json!(true)),
    ];
    let res = run(cx, "line_events", with(&base, &le))?;
    let lns: HashMap<i64, &demo::LineEvent> = d.lines.iter().map(|x| (x.eid, x)).collect();
    let (mut l_ok, mut l_total) = (0, 0);
    for (f, src, _) in records(&res["OUTPUT"]) {
        l_total += 1;
        let Some(e) = src.and_then(|s| lns.get(&s)) else {
            continue;
        };
        let near = |a: &J, b: Option<f64>| (n(a) - n(&opt3(b))).abs() <= TOL;
        l_ok += i32::from(near(&f["rl_m_from"], e.exp_m_from) && near(&f["rl_m_to"], e.exp_m_to));
    }
    let l_err = records(&res["ERRORS"])
        .iter()
        .filter(|(f, src, _)| {
            src.and_then(|s| lns.get(&s))
                .is_some_and(|e| f["rl_error"] == e.exp_error)
        })
        .count();
    let sections = materialize(&res["OUTPUT"], &l.lines, cx.tr("Участки, результат"));
    add("SECTIONS", sections);

    // дефекты: обратная привязка
    let res = run(
        cx,
        "locate_points",
        with(
            &base,
            &[
                ("POINTS", l.defects.value.clone()),
                ("@POINTS", l.defects.info.clone()),
                ("RADIUS", json!(50.0)),
                ("ERRORS", json!(true)),
            ],
        ),
    )?;
    let defs: HashMap<i64, &demo::Defect> = d.defects.iter().map(|x| (x.did, x)).collect();
    let (mut d_ok, mut d_total) = (0, 0);
    for (f, src, _) in records(&res["OUTPUT"]) {
        d_total += 1;
        let Some(e) = src.and_then(|s| defs.get(&s)) else {
            continue;
        };
        d_ok += i32::from(
            f["rl_route"] == e.exp_route
                && (n(&f["rl_m"]) - n(&r3(e.exp_m))).abs() <= TOL
                && (n(&f["rl_offset"]) - n(&r3(e.exp_offset))).abs() <= TOL,
        );
    }
    let located = materialize(&res["OUTPUT"], &l.defects, cx.tr("Дефекты, привязка"));
    add("LOCATED", located.clone());

    let res = run(cx, "pickets", with(&base, &[("STEP", json!(100.0))]))?;
    add("PICKETS", {
        let mut o = res["OUTPUT"].clone();
        o["name"] = json!(cx.tr("Пикеты"));
        o
    });

    // профиль: отметки рельефа и оси сверяются с формулами
    let res = run(
        cx,
        "profile_table",
        with(
            &base,
            &[
                ("PICKET_STEP", json!(100.0)),
                ("STEP", json!(50.0)),
                ("VERTICES", json!(0)),
                ("POIS", l.defects.value.clone()),
                ("@POIS", l.defects.info.clone()),
                ("POI_LABEL", json!("did")),
                ("CORRIDOR", json!(5.0)),
            ],
        ),
    )?;
    let dem = Dem::new();
    let mut table = res["OUTPUT"].clone();
    table["name"] = json!(cx.tr("Таблица профиля"));
    if let Some(fs) = table["fields"].as_array_mut() {
        fs.push(json!([ZF, "double"]));
    }
    if let Some(feats) = table["features"].as_array_mut() {
        for f in feats {
            let (x, y) = (n(&f["geometry"]["xyz"][0]), n(&f["geometry"]["xyz"][1]));
            let z = dem.sample(x, y);
            if let Some(v) = f["values"].as_array_mut() {
                v.push(rm(z));
            }
        }
    }
    let (mut g_ok, mut g_total, mut a_ok, mut a_total, mut n_eq) = (0, 0, 0, 0, 0);
    let mut g_worst = 0.0f64;
    let rows = records(&table);
    for (f, _, _) in &rows {
        n_eq += i32::from(f["route_id"] == "R3" && f["kind"] == "equation");
        if !f[ZF].is_null() {
            g_total += 1;
            let dz = (n(&f[ZF]) - demo::dem_z(n(&f["x"]), n(&f["y"]))).abs();
            g_worst = g_worst.max(dz);
            g_ok += i32::from(dz <= TOL);
        }
        if f["route_id"] == "R1" {
            a_total += 1;
            a_ok += i32::from(
                !f["z_axis"].is_null() && (n(&f["z_axis"]) - demo::r1_z(n(&f["m"]))).abs() <= TOL,
            );
        }
    }
    let n_table = rows.len() as i32;
    let (t_value, t_info) = as_points(&table);
    let (p_value, p_info) = as_points(&located);
    let res = run(
        cx,
        "profile_drawing",
        json!({
            "TABLE": t_value, "@TABLE": t_info, "ROUTE": "R1", "GROUND": ZF,
            "PLAN": p_value, "@PLAN": p_info, "PLAN_LABEL": "did",
        }),
    )?;
    let width = n(&res["width"]);
    let drawing = &res["out"];
    let count = |o: &J| o["features"].as_array().map_or(0, Vec::len);
    let drawn = (width - 2000.0 / 500.0 * 1000.0).abs() <= TOL
        && count(&drawing["LINES"]) > 0
        && count(&drawing["TEXTS"]) > 0;
    add("TABLE", table);
    add("PROFILE_LINES", {
        let mut o = drawing["LINES"].clone();
        o["name"] = json!(layer_name(cx, "Профиль, линии"));
        o
    });
    add("PROFILE_TEXTS", {
        let mut o = drawing["TEXTS"].clone();
        o["name"] = json!(layer_name(cx, "Профиль, подписи"));
        o
    });

    let n_def = d.defects.len() as i32;
    let p_total_all = p_total_all as i32;
    let passed = ok_routes
        && p_ok == p_total
        && p_total == p_total_all
        && e_ok == exp_err
        && l_ok == l_total
        && l_err == 1
        && d_ok == d_total
        && d_total == n_def
        && g_total == n_table
        && g_ok == g_total
        && a_ok == a_total
        && a_total > 0
        && n_eq == 2
        && drawn
        && cal_ok
        && m_ok == m_total
        && m_total == m_exp
        && m_exp > 0;
    let routes_v = if ok_routes {
        yes_no(true)
    } else {
        Msg::new("{no} {list}").m("no", yes_no(false)).s(
            "list",
            format!(
                "[{}]",
                bad.iter()
                    .map(|b| format!("'{b}'"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )
    };
    let lines = [
        Msg::new("Маршруты: не собран только R4 - {v}").m("v", routes_v),
        Msg::new(
            "Точечные: совпало {a} из {n} (наибольшее расхождение {w:.1f} мм), ожидаемых ошибок распознано {e} из {en}",
        )
        .i("a", i64::from(p_ok))
        .i("n", p_total_all.into())
        .f("w", worst * 1000.0)
        .i("e", e_ok as i64)
        .i("en", exp_err as i64),
        Msg::new(
            "Калибровка: источники и уравнения маршрутов верны - {v}, точечные события по M совпали {a} из {n}",
        )
        .m("v", yes_no(cal_ok))
        .i("a", m_ok.into())
        .i("n", m_exp.into()),
        Msg::new("Участки: совпало {a} из {n}, ошибка нулевой длины распознана: {v}")
            .i("a", l_ok.into())
            .i("n", l_total.into())
            .m("v", yes_no(l_err == 1)),
        Msg::new("Дефекты: привязано верно {a} из {n}")
            .i("a", d_ok.into())
            .i("n", n_def.into()),
        Msg::new(
            "Профиль: отметки рельефа совпали {a} из {n} (наибольшее расхождение {w:.1f} мм), отметки оси R1 {b} из {m}, уравнений R3 {e} из 2, чертёж построен: {v}",
        )
        .i("a", g_ok.into())
        .i("n", n_table.into())
        .f("w", g_worst * 1000.0)
        .i("b", a_ok.into())
        .i("m", a_total.into())
        .i("e", n_eq.into())
        .m("v", yes_no(drawn)),
        Msg::new(if passed {
            "ПРОВЕРКА ПРОЙДЕНА"
        } else {
            "ПРОВЕРКА НЕ ПРОЙДЕНА"
        }),
    ];
    for m in lines {
        cx.say(m);
    }
    Ok(())
}
