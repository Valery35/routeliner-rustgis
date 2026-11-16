// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 3.0 или новее.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Пример 1.01 против записи модуля: `tests/data/demo_seed1.json` -
//! копия `tests/fixtures/demo_seed1.json` модуля без правки (зерно 1, 60
//! точечных событий, 30 дефектов). Маршруты - части с нарезкой дуги R2
//! как в QGIS, координаты в записи до 6 знаков.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use routeliner_rustgis::core::demo::build;
use routeliner_rustgis::pyfmt::round;
use serde_json::Value as J;

fn fixture() -> J {
    serde_json::from_str(include_str!("data/demo_seed1.json")).unwrap()
}

fn same(a: f64, b: &J, tol: f64, what: &str) {
    let b = b
        .as_f64()
        .unwrap_or_else(|| panic!("{what}: в записи не число {b}"));
    assert!((a - b).abs() <= tol, "{what}: {a} против {b}");
}

fn opt(a: Option<f64>, b: &J, what: &str) {
    match a {
        None => assert!(b.is_null(), "{what}: пусто против {b}"),
        Some(a) => same(round(a, 3), b, 1e-9, what),
    }
}

/// Самопроверка 1.01 против итога модуля (`tests/data/tool_1_01.json`,
/// копия `tests/fixtures/tool_1_01.json` модуля).
#[test]
fn demo_check_matches_module() {
    use routeliner_rustgis::product::TOOLS_JSON;
    use rustgis_kit::{Catalog, Lang};
    let cat = Catalog::parse(TOOLS_JSON).unwrap();
    let tool = cat.tool("demo").unwrap();
    let values = serde_json::json!({"N_POINTS": 60, "SEED": 1, "RUN": true});
    let (out, msgs) = routeliner_rustgis::tools::run(tool, &values, Lang::Ru).unwrap();
    let text = msgs.join("\n");
    println!("{text}");
    let want: J = serde_json::from_str(include_str!("data/tool_1_01.json")).unwrap();
    let c = &want["cases"][0]["expected"]["check"];
    let line = |s: String| assert!(text.contains(&s), "нет строки «{s}» в\n{text}");
    line("Маршруты: не собран только R4 - да".into());
    line(format!(
        "Точечные: совпало {} из {} (наибольшее расхождение {:.1} мм), ожидаемых ошибок распознано {} из {}",
        c["points_matched"],
        c["points_total"],
        c["points_worst_mm"].as_f64().unwrap(),
        c["point_errors_recognized"],
        c["point_errors_expected"]
    ));
    line(format!(
        "Калибровка: источники и уравнения маршрутов верны - да, точечные события по M совпали {} из {}",
        c["points_by_m_matched"], c["points_by_m_total"]
    ));
    line(format!(
        "Участки: совпало {} из {}, ошибка нулевой длины распознана: да",
        c["lines_matched"], c["lines_total"]
    ));
    line(format!(
        "Дефекты: привязано верно {} из {}",
        c["defects_matched"], c["defects_total"]
    ));
    line(format!(
        "Профиль: отметки рельефа совпали {} из {} (наибольшее расхождение {:.1} мм), отметки оси R1 {} из {}, уравнений R3 {} из 2, чертёж построен: да",
        c["profile_ground_matched"],
        c["profile_points"],
        c["profile_ground_worst_mm"].as_f64().unwrap(),
        c["profile_axis_r1_matched"],
        c["profile_axis_r1_total"],
        c["profile_equations_r3"]
    ));
    line("ПРОВЕРКА ПРОЙДЕНА".into());
    // пример и итоги проверки - слоями и таблицами
    assert_eq!(out.as_object().unwrap().len(), 15);
}

/// 1.01 через RustGIS: пример встаёт на место. Слои уходят в метрах
/// EPSG:32640 с членом `crs`, начало R1 - точка 455000, 6428000, как
/// в примере модуля, а сообщения кончаются «ПРОВЕРКА ПРОЙДЕНА».
#[test]
fn demo_through_rustgis_lands_in_place() {
    use routeliner_rustgis::product::Routeliner;
    use rustgis_kit::Lang;
    let p = Routeliner::new().unwrap();
    let req = serde_json::json!({
        "kind": "tool", "id": "demo", "lang": "ru",
        "params": {"n_points": 60, "seed": 1, "run": true}, "layers": {}
    });
    let reply = rustgis_kit::runner::run_tool(&p, &req, Lang::Ru);
    let ok = &reply["ok"];
    assert!(ok.is_object(), "{reply}");
    let layers = ok["layers"].as_array().unwrap();
    let routes = &layers.iter().find(|l| l["name"] == "Маршруты").unwrap()["geojson"];
    assert_eq!(routes["crs"]["properties"]["name"], "EPSG:32640");
    let first = &routes["features"][0]["geometry"]["coordinates"][0][0];
    assert_eq!(first[0].as_f64().unwrap(), 455000.0);
    assert_eq!(first[1].as_f64().unwrap(), 6428000.0);
    for l in layers.iter().chain(ok["tables"].as_array().unwrap()) {
        assert_eq!(
            l["geojson"]["crs"]["properties"]["name"], "EPSG:32640",
            "{}",
            l["name"]
        );
    }
    assert_eq!(layers.len() + ok["tables"].as_array().unwrap().len(), 15);
    let msgs = ok["messages"].as_array().unwrap();
    assert_eq!(msgs.last().unwrap(), "ПРОВЕРКА ПРОЙДЕНА");
}

#[test]
fn demo_matches_module() {
    let fx = fixture();
    let d = build(60, 30, 1);

    let routes = fx["routes"].as_array().unwrap();
    assert_eq!(d.routes.len(), routes.len());
    for (r, f) in d.routes.iter().zip(routes) {
        assert_eq!(r.route_id, f["route_id"]);
        let parts = f["parts"].as_array().unwrap();
        assert_eq!(r.parts.len(), parts.len(), "{} частей", r.route_id);
        // запись модуля - до 6 знаков, дуга R2 нарезана как в QGIS 4.2.3
        let tol = 1e-6;
        for (p, q) in r.parts.iter().zip(parts) {
            let q = q.as_array().unwrap();
            assert_eq!(p.len(), q.len(), "{} вершин", r.route_id);
            for (v, w) in p.iter().zip(q) {
                for k in 0..3 {
                    same(v[k], &w[k], tol, r.route_id);
                }
            }
        }
    }

    let ledger = fx["ledger"].as_array().unwrap();
    assert_eq!(d.ledger.len(), ledger.len());
    for (l, f) in d.ledger.iter().zip(ledger) {
        assert_eq!(l.station, f["station"]);
        assert_eq!(
            l.station_ahead.map(J::from).unwrap_or(J::Null),
            f["station_ahead"]
        );
        same(l.measure, &f["measure"], 0.0, "measure");
    }

    let pts = fx["events_points"].as_array().unwrap();
    assert_eq!(d.points.len(), pts.len());
    for (p, f) in d.points.iter().zip(pts) {
        let w = format!("точка {}", p.eid);
        assert_eq!(p.eid, f["eid"].as_i64().unwrap(), "{w}");
        assert_eq!(p.route_id, f["route_id"], "{w}");
        assert_eq!(p.pk, f["pk"].as_str().unwrap(), "{w}");
        same(round(p.offset, 3), &f["offset"], 1e-9, &w);
        assert_eq!(
            p.section.map(|s| s as i64),
            f["section"].as_i64(),
            "{w} участок"
        );
        opt(p.exp_m, &f["exp_m"], &w);
        opt(p.exp_x, &f["exp_x"], &w);
        opt(p.exp_y, &f["exp_y"], &w);
        assert_eq!(p.exp_error, f["exp_error"], "{w}");
    }

    let lines = fx["events_lines"].as_array().unwrap();
    assert_eq!(d.lines.len(), lines.len());
    for (l, f) in d.lines.iter().zip(lines) {
        assert_eq!(l.pk_from, f["pk_from"]);
        opt(l.exp_m_from, &f["exp_m_from"], "m_from");
        opt(l.exp_m_to, &f["exp_m_to"], "m_to");
        assert_eq!(l.exp_error, f["exp_error"]);
    }

    let defects = fx["defects"].as_array().unwrap();
    assert_eq!(d.defects.len(), defects.len());
    for (p, f) in d.defects.iter().zip(defects) {
        let w = format!("дефект {}", p.did);
        same(p.x, &f["x"], 1e-6, &w);
        same(p.y, &f["y"], 1e-6, &w);
        assert_eq!(p.exp_route, f["exp_route"], "{w}");
        same(round(p.exp_m, 3), &f["exp_m"], 1e-9, &w);
        same(round(p.exp_station, 3), &f["exp_station"], 1e-9, &w);
        same(round(p.exp_offset, 3), &f["exp_offset"], 1e-9, &w);
    }
}
