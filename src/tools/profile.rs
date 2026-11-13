// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 3.0 или новее.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Группа 5, профиль: 5.01 «Таблица профиля» и 5.02 «Чертёж профиля»
//! (`processing/alg_profile.py`). Растров у расширений RustGIS нет, поэтому
//! 5.01 считается без полей `z_*`, а у 5.02 нет поля ORIGIN и матрицы ROWS
//! в окне: чертёж ставится под таблицей, строки сетки - шаблон
//! «Трубопровод» из `tools.json`.

use std::collections::HashMap;
use std::sync::LazyLock;

use indexmap::IndexMap;
use regex::Regex;
use rustgis_kit::Failure;
use serde_json::{Value as J, json};

use super::common::{self, Cx, num, rdeg, rm, route_key};
use super::expr::Expr;
use crate::core::profile::{ProfileDrawer, ProfilePoint, ROW_TYPES, Row, Vertices, collect_points};
use crate::core::stations::py_str;
use crate::pyfmt::{self, Msg};

static DECIMALS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+)\.(\d+)").unwrap_or_else(|e| unreachable!("{e}")));

/// Подпись пикета для сетки: без лишних нулей после точки
/// (ПК 6+20.00 - ПК 6+20, ПК 6+79.30 - ПК 6+79.3).
pub fn compact_station(text: &str) -> String {
    DECIMALS
        .replace_all(text, |c: &regex::Captures<'_>| {
            let frac = c[2].trim_end_matches('0');
            if frac.is_empty() {
                c[1].to_string()
            } else {
                format!("{}.{frac}", &c[1])
            }
        })
        .into_owned()
}

/// 5.01: точки продольного профиля по маршрутам.
pub fn profile_table(cx: &mut Cx<'_>) -> Result<J, Failure> {
    let (routes, _) = common::routes(cx)?;
    let mut sys = common::locator(cx, &routes)?;
    let wanted: Vec<String> = cx
        .text("ROUTE_LIST")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    let mut ids: Vec<String> = routes.keys().cloned().collect();
    if !wanted.is_empty() {
        for w in &wanted {
            if !routes.contains_key(w) {
                cx.say(Msg::new("маршрут «{rid}» не найден").s("rid", w.clone()));
            }
        }
        ids = wanted
            .into_iter()
            .filter(|w| routes.contains_key(w))
            .collect();
    }
    let mode = match cx.index("VERTICES") {
        1 => Vertices::All,
        2 => Vertices::None,
        _ => Vertices::Significant,
    };
    // события на профиль: проекция на ближайший маршрут в коридоре от оси
    let mut events: HashMap<String, Vec<(f64, String)>> = HashMap::new();
    if let Some(pois) = cx.layer("POIS") {
        let lab = cx.text("POI_LABEL");
        let corridor = cx.number("CORRIDOR");
        for f in pois {
            let (Some(x), Some(y)) = (f["xyz"][0].as_f64(), f["xyz"][1].as_f64()) else {
                continue;
            };
            let mut best: Option<(&String, crate::core::route::Projection)> = None;
            for rid in &ids {
                if let Ok(pr) = routes[rid].locate(x, y)
                    && best
                        .as_ref()
                        .is_none_or(|(_, b)| pr.offset.abs() < b.offset.abs())
                {
                    best = Some((rid, pr));
                }
            }
            if let Some((rid, pr)) = best
                && pr.offset.abs() <= corridor
            {
                let v = lab.as_ref().map_or(J::Null, |l| f["attrs"][l].clone());
                let text = if v.is_null() {
                    String::new()
                } else {
                    py_str(&v)
                };
                events.entry(rid.clone()).or_default().push((pr.m, text));
            }
        }
    }
    let parser = sys.loc.parser;
    let (picket_step, step) = (cx.number("PICKET_STEP"), cx.number("STEP"));
    let mut feats = Vec::new();
    for rid in &ids {
        let r = &routes[rid];
        let Some(system) = sys.loc.system_for(rid) else {
            continue;
        };
        let ev = events.get(rid).cloned().unwrap_or_default();
        let pts = collect_points(r, system, picket_step, step, mode, &ev);
        let ms: Vec<f64> = pts.iter().map(|p| p.m).collect();
        let at = r.points_at(&ms, &[0.0]);
        // Z нет или все нули - отметки оси нет
        let no_z = at.iter().all(|a| a.z.is_nan() || a.z == 0.0);
        for (i, (p, a)) in pts.iter().zip(&at).enumerate() {
            let pk = if p.kind == "equation" {
                format!(
                    "{} = {}",
                    parser.format(p.station),
                    parser.format(p.station_ahead)
                )
            } else {
                parser.format(p.station)
            };
            let z = if no_z { f64::NAN } else { a.z };
            feats.push(json!({
                "values": [rid, i + 1, p.kind, rm(p.m), rm(p.station), rm(p.station_ahead), pk,
                           p.section, rm(a.x), rm(a.y), rm(z),
                           if p.turn != 0.0 { rdeg(p.turn) } else { J::Null },
                           if p.label.is_empty() { J::Null } else { json!(p.label) }],
                "geometry": {"xyz": [a.x, a.y, J::Null]},
            }));
        }
    }
    cx.say(
        Msg::new("Итого: точек профиля {n}, растров {r}")
            .i("n", feats.len() as i64)
            .i("r", 0),
    );
    Ok(json!({"OUTPUT": {
        "geometry": "point",
        "fields": [["route_id", "string"], ["n", "int"], ["kind", "string"], ["m", "double"],
                   ["station", "double"], ["station_ahead", "double"], ["pk", "string"],
                   ["section", "int"], ["x", "double"], ["y", "double"], ["z_axis", "double"],
                   ["turn", "double"], ["label", "string"]],
        "features": feats,
    }}))
}

/// Ячейка матрицы строк как текст.
fn cell(v: &J) -> String {
    match v {
        J::Null => String::new(),
        J::String(s) => s.trim().to_string(),
        other => py_str(other).trim().to_string(),
    }
}

/// Поле в кавычках для выражения (`QgsExpression.quotedColumnRef`).
fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// 5.02: чертёж профиля одного маршрута по таблице 5.01.
pub fn profile_drawing(cx: &mut Cx<'_>) -> Result<J, Failure> {
    let Some(table) = cx.layer("TABLE") else {
        return Err(cx.fail(Msg::new("Не задана таблица профиля")));
    };
    let fields: Vec<String> = cx.values["@TABLE"]["fields"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| f.as_str().map(str::to_string))
        .collect();
    for need in ["route_id", "m", "kind", "pk"] {
        if !fields.iter().any(|f| f == need) {
            return Err(cx.fail(
                Msg::new("В таблице нет поля «{f}», нужна таблица инструмента 5.01").s("f", need),
            ));
        }
    }
    // точки маршрута: заданного или первого в таблице, по мере
    let rid_of = |f: &J| py_str(&f["attrs"]["route_id"]);
    let rid = match cx.text("ROUTE") {
        Some(r) => r,
        None => table.first().map(rid_of).unwrap_or_default(),
    };
    let mut feats: Vec<&J> = table.iter().filter(|f| rid_of(f) == rid).collect();
    if feats.is_empty() {
        return Err(cx.fail(Msg::new("В таблице нет точек маршрута «{rid}»").s("rid", rid)));
    }
    let m_of = |f: &J| num(&f["attrs"]["m"]).unwrap_or(f64::NAN);
    feats.sort_by(|a, b| m_of(a).total_cmp(&m_of(b)));
    let pts: Vec<ProfilePoint> = feats
        .iter()
        .map(|f| {
            let a = &f["attrs"];
            let text = |k: &str| {
                if common::is_null(&a[k]) {
                    String::new()
                } else {
                    py_str(&a[k])
                }
            };
            let mut p = ProfilePoint::new(m_of(f), &text("kind"));
            if p.kind.is_empty() {
                p.kind = "step".into();
            }
            p.label = text("label");
            p.turn = num(&a["turn"]).unwrap_or(0.0);
            p.station = num(&a["station"]).unwrap_or(f64::NAN);
            p.station_ahead = num(&a["station_ahead"]).unwrap_or(f64::NAN);
            p.section = num(&a["section"]).map_or(0, |s| s as usize);
            p
        })
        .collect();

    let mut subst: HashMap<&str, Option<String>> = HashMap::new();
    subst.insert("d", Some(pyfmt::repr(cx.number("D"))));
    subst.insert("base", Some(pyfmt::repr(cx.number("BASE"))));
    for (key, par) in [("ground", "GROUND"), ("design", "DESIGN"), ("pipe", "PIPE")] {
        let mut name = cx.text(par);
        if name.is_none() && key == "pipe" && fields.iter().any(|f| f == "z_axis") {
            name = Some("z_axis".into());
        }
        subst.insert(key, name.map(|n| quoted(&n)));
    }
    static SUBST: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\{(\w+)\}").unwrap_or_else(|e| unreachable!("{e}")));
    let evaluate = |text: &str| -> Result<Vec<f64>, Msg> {
        let missing: Vec<String> = SUBST
            .captures_iter(text)
            .map(|c| c[1].to_string())
            .filter(|k| subst.get(k.as_str()).is_none_or(Option::is_none))
            .collect();
        if !missing.is_empty() {
            return Err(Msg::new("не задано: {names}").s("names", missing.join(", ")));
        }
        let expr_text = SUBST.replace_all(text, |c: &regex::Captures<'_>| {
            subst.get(&c[1]).cloned().flatten().unwrap_or_default()
        });
        let e = Expr::parse(&expr_text).map_err(|w| Msg::new("{w}").s("w", w))?;
        if let Some(f) = e
            .fields()
            .into_iter()
            .find(|f| !fields.iter().any(|x| x == f))
        {
            return Err(Msg::new("{w}").s("w", format!("поля «{f}» нет в таблице")));
        }
        Ok(feats.iter().map(|f| e.eval(&f["attrs"])).collect())
    };

    let sections = sections(cx, &rid)?;
    let grade_tol = cx.number("GRADE_TOL");
    let raw = cx.raw("ROWS");
    let flat: Vec<J> = raw
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|v| match v {
            J::Array(a) => a.clone(),
            other => vec![other.clone()],
        })
        .collect();
    let mut rows = Vec::new();
    let mut log = Vec::new();
    let (grid_rows, _) = flat.as_chunks::<6>();
    for c in grid_rows {
        let [title, kind, source, dec, height, draw] = c.each_ref().map(cell);
        let title = cx.tr(&title);
        let kind = kind.to_lowercase();
        if !ROW_TYPES.contains(&kind.as_str()) {
            log.push(
                Msg::new("Строка «{t}»: неизвестный тип «{k}»")
                    .s("t", title)
                    .s("k", kind),
            );
            continue;
        }
        let decimals = if dec.is_empty() {
            Some(if kind == "station" || kind == "text" {
                0.0
            } else {
                2.0
            })
        } else {
            dec.parse::<f64>().ok()
        };
        let h = if height.is_empty() {
            Some(15.0)
        } else {
            height.parse::<f64>().ok()
        };
        let (Some(decimals), Some(h)) = (decimals, h) else {
            log.push(Msg::new("Строка «{t}»: знаков и высота должны быть числами").s("t", title));
            continue;
        };
        let draw = matches!(draw.as_str(), "1" | "да" | "yes" | "true");
        let mut row = Row::new(&title, &kind, h, decimals.trunc().max(0.0) as usize, draw);
        row.tolerance = grade_tol;
        if kind == "value" || kind == "grade" {
            match evaluate(&source) {
                Ok(v) => row.values = Some(v),
                Err(why) => {
                    log.push(
                        Msg::new("Строка «{t}» пропущена: {why}")
                            .s("t", title)
                            .m("why", why),
                    );
                    continue;
                }
            }
        } else if kind == "text" {
            row.segments = sections.get(&source).cloned().unwrap_or_default();
            if row.segments.is_empty() {
                log.push(
                    Msg::new("Строка «{t}» пропущена: нет участков с полем «{f}»")
                        .s("t", title)
                        .s("f", source),
                );
                continue;
            }
        }
        rows.push(row);
    }
    for m in log {
        cx.say(m);
    }

    let plan = plan(cx, &rid)?;
    let horizon = match cx.values.get("HORIZON") {
        Some(v) if !v.is_null() && v.as_str() != Some("") => rustgis_kit::tool::number(v),
        _ => None,
    };
    let labels: Vec<String> = feats
        .iter()
        .map(|f| {
            let pk = &f["attrs"]["pk"];
            compact_station(&if common::is_null(pk) {
                String::new()
            } else {
                py_str(pk)
            })
        })
        .collect();
    let tr = |m: &Msg| cx.text_of(m);
    let sheet = ProfileDrawer::new(
        pts,
        labels,
        cx.number("H_SCALE"),
        cx.number("V_SCALE"),
        horizon,
        cx.number("MIN_GAP"),
        plan,
        &tr,
    )
    .build(&rows);

    // чертёж ставится под таблицей: на 100 мм ниже её нижнего края
    let xy: Vec<(f64, f64)> = table
        .iter()
        .filter_map(|f| Some((f["xyz"][0].as_f64()?, f["xyz"][1].as_f64()?)))
        .collect();
    if xy.is_empty() {
        return Err(cx.fail(
            Msg::new("В таблице нет поля «{f}», нужна таблица инструмента 5.01").s("f", "geometry"),
        ));
    }
    let top = sheet
        .lines
        .iter()
        .flat_map(|l| l.0.iter().map(|p| p.1))
        .fold(0.0, f64::max);
    let ox = xy.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let oy = xy.iter().map(|p| p.1).fold(f64::INFINITY, f64::min) - 100.0 - top;

    let lines: Vec<J> = sheet
        .lines
        .iter()
        .map(|(coords, kind, row, color, width)| {
            let pts: Vec<J> = coords
                .iter()
                .map(|(x, y)| json!([ox + x, oy + y, J::Null]))
                .collect();
            json!({"values": [kind, row, color, width], "geometry": {"points": pts}})
        })
        .collect();
    let mut texts: Vec<J> = sheet
        .texts
        .iter()
        .map(|t| {
            json!({"values": [t.text, t.kind, t.rot, t.size, t.halign, t.valign],
                   "geometry": {"xyz": [ox + t.x, oy + t.y, J::Null]}})
        })
        .collect();
    texts.extend(sheet.marks.iter().map(|(x, y, kind, _)| {
        json!({"values": ["", kind, 0.0, 0.0, "Center", "Half"],
               "geometry": {"xyz": [ox + x, oy + y, J::Null]}})
    }));
    cx.say(
        Msg::new(
            "Профиль маршрута «{rid}»: строк {r}, ширина {w:.0f} мм, условный горизонт {h:.2f}",
        )
        .s("rid", rid)
        .i("r", rows.len() as i64)
        .f("w", sheet.width)
        .f("h", sheet.horizon),
    );
    Ok(json!({
        "LINES": {"geometry": "line",
                  "fields": [["kind", "string"], ["row", "string"], ["color", "string"], ["width", "double"]],
                  "features": lines},
        "TEXTS": {"geometry": "point",
                  "fields": [["text", "string"], ["kind", "string"], ["rot", "double"], ["size", "double"],
                             ["halign", "string"], ["valign", "string"]],
                  "features": texts},
    }))
}

/// Участки текстовых строк по полю слоя: (мера начала, мера конца, текст).
type Sections = IndexMap<String, Vec<(f64, f64, String)>>;

/// Участки для текстовых строк по каждому полю слоя участков.
fn sections(cx: &Cx<'_>, rid: &str) -> Result<Sections, Failure> {
    let mut out = Sections::new();
    let Some(src) = cx.layer("SECTIONS") else {
        return Ok(out);
    };
    let rf = cx.text("SEC_ROUTE");
    let f0 = cx.text("SEC_FROM").unwrap_or_else(|| "rl_m_from".into());
    let f1 = cx.text("SEC_TO").unwrap_or_else(|| "rl_m_to".into());
    let names: Vec<String> = cx.values["@SECTIONS"]["fields"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| f.as_str().map(str::to_string))
        .collect();
    if !names.contains(&f0) || !names.contains(&f1) {
        return Err(cx.fail(
            Msg::new("В слое участков нет полей меры «{a}» и «{b}»")
                .s("a", f0)
                .s("b", f1),
        ));
    }
    for f in src {
        let a = &f["attrs"];
        if let Some(r) = &rf
            && route_key(&a[r]).as_deref() != Some(rid)
            && py_str(&a[r]) != rid
        {
            continue;
        }
        let (Some(m0), Some(m1)) = (num(&a[&f0]), num(&a[&f1])) else {
            continue;
        };
        for n in &names {
            let v = &a[n];
            if !v.is_null() && !py_str(v).is_empty() {
                out.entry(n.clone()).or_default().push((m0, m1, py_str(v)));
            }
        }
    }
    Ok(out)
}

/// Точки развёрнутого плана из итога 4.02: мера, смещение, подпись.
fn plan(cx: &Cx<'_>, rid: &str) -> Result<Vec<(f64, f64, String)>, Failure> {
    let Some(src) = cx.layer("PLAN") else {
        return Ok(Vec::new());
    };
    let names: Vec<&str> = cx.values["@PLAN"]["fields"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(J::as_str)
        .collect();
    if !names.contains(&"rl_m") || !names.contains(&"rl_offset") {
        return Err(cx.fail(Msg::new(
            "В слое плана нужны поля rl_m и rl_offset (результат 4.02)",
        )));
    }
    let lab = cx.text("PLAN_LABEL");
    let mut out = Vec::new();
    for f in src {
        let a = &f["attrs"];
        if names.contains(&"rl_route") && py_str(&a["rl_route"]) != rid {
            continue;
        }
        let (Some(m), Some(off)) = (num(&a["rl_m"]), num(&a["rl_offset"])) else {
            continue;
        };
        let label = lab
            .as_ref()
            .map(|l| &a[l])
            .filter(|v| !v.is_null())
            .map(py_str)
            .unwrap_or_default();
        out.push((m, off, label));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_labels() {
        assert_eq!(compact_station("ПК 6+20.00"), "ПК 6+20");
        assert_eq!(compact_station("ПК 6+79.30"), "ПК 6+79.3");
        assert_eq!(
            compact_station("ПК 6+99.30 = ПК 6+74.30"),
            "ПК 6+99.3 = ПК 6+74.3"
        );
    }
}
