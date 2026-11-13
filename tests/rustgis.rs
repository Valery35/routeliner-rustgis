// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 2.0 или новее.
// SPDX-License-Identifier: GPL-2.0-or-later

//! Сквозная проверка через протокол RustGIS: слои примера 1.01 в градусах
//! WGS 84 без Z, как их отдаёт RustGIS, запрос инструмента одним запуском,
//! ответ - слои и таблицы. Крейт переводит градусы в метры зоны UTM 40N и
//! обратно, итог совпадает с прогоном модуля до 0,001 м.
//!
//! Тест `plugin_json_is_current` сверяет раздел `tools` манифеста с
//! описаниями. Переписать файл - `ROUTELINER_WRITE_MANIFEST=1 cargo test`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use routeliner_rustgis::product::{Routeliner, manifest};
use rustgis_kit::geo::Utm;
use serde_json::{Value as J, json};

const RUNS: &str = include_str!("../fixtures/tool_runs.json");

fn run(tool: &str, case: &str) -> J {
    let runs: J = serde_json::from_str(RUNS).unwrap();
    runs["runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["tool"] == tool && r["case"] == case)
        .cloned()
        .unwrap()
}

/// Зона примера: EPSG:32640.
fn zone() -> Utm {
    Utm::at(57.0, 58.0)
}

/// Линии прогона - слой RustGIS в градусах без Z.
fn lines_layer(items: &J) -> J {
    let u = zone();
    let feats: Vec<J> = items
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            let parts: Vec<Vec<J>> = f["parts"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| {
                    p.as_array()
                        .unwrap()
                        .iter()
                        .map(|v| json!(u.inverse(v[0].as_f64().unwrap(), v[1].as_f64().unwrap())))
                        .collect()
                })
                .collect();
            let geometry = if parts.len() == 1 {
                json!({"type": "LineString", "coordinates": parts[0]})
            } else {
                json!({"type": "MultiLineString", "coordinates": parts})
            };
            json!({"type": "Feature", "row": f["id"], "properties": f["attrs"], "geometry": geometry})
        })
        .collect();
    json!({"type": "FeatureCollection", "features": feats})
}

/// Таблица прогона - слой RustGIS без геометрии.
fn table_layer(items: &J) -> J {
    let feats: Vec<J> = items
        .as_array()
        .unwrap()
        .iter()
        .map(|f| json!({"type": "Feature", "row": f["id"], "properties": f["attrs"], "geometry": null}))
        .collect();
    json!({"type": "FeatureCollection", "features": feats})
}

fn ask(req: &J) -> J {
    let p = Routeliner::new().unwrap();
    let mut out = Vec::new();
    rustgis_kit::serve(&p, format!("{req}\n").as_bytes(), &mut out).unwrap();
    serde_json::from_slice(&out).unwrap()
}

/// Поле итога по маршруту.
fn by_route<'a>(layer: &'a J, rid: &str) -> &'a J {
    layer["geojson"]["features"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["properties"]["route_id"] == rid)
        .map(|f| &f["properties"])
        .unwrap()
}

#[test]
fn check_routes_through_rustgis() {
    let r = run("check_routes", "allow_gaps");
    let req = json!({"abi": 1, "kind": "tool", "id": "check-routes", "lang": "ru",
        "params": {"routes": "Маршруты", "route_id": "route_id", "allow_gaps": true, "errors": true},
        "layers": {"Маршруты": lines_layer(&r["input"]["ROUTES"])}});
    let reply = ask(&req);
    let ok = &reply["ok"];
    assert!(ok.is_object(), "{reply}");
    let layer = &ok["layers"][0];
    assert_eq!(layer["name"], "Собранные маршруты");
    for f in r["output"]["OUTPUT"]["features"].as_array().unwrap() {
        let v = &f["values"];
        let got = by_route(layer, v[0].as_str().unwrap());
        let d = (got["length"].as_f64().unwrap() - v[1].as_f64().unwrap()).abs();
        assert!(d <= 1e-3, "{}: {d}", v[0]);
        assert_eq!(got["parts"], v[2]);
        assert_eq!(got["gaps"], v[3]);
    }
    // R4 с разрывом - мультилинией
    let r4 = layer["geojson"]["features"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["properties"]["route_id"] == "R4")
        .unwrap();
    assert_eq!(r4["geometry"]["type"], "MultiLineString");
    assert_eq!(ok["messages"][0], "Маршрутов собрано 4, не собрано 0");
}

#[test]
fn calibrate_with_ledger_through_rustgis() {
    let r = run("calibrate_routes", "ledger_station");
    let req = json!({"abi": 1, "kind": "tool", "id": "calibrate-routes", "lang": "en",
        "params": {"routes": "Routes", "route_id": "route_id", "ledger": "Ledger",
                   "lg_route": "route_id", "lg_station": "station", "lg_measure": "measure",
                   "lg_ahead": "station_ahead", "lg_system": "system", "system": "исполнительная",
                   "out_m": 2, "errors": true},
        "layers": {"Routes": lines_layer(&r["input"]["ROUTES"]),
                   "Ledger": table_layer(&r["input"]["LEDGER"])}});
    let reply = ask(&req);
    let ok = &reply["ok"];
    assert!(ok.is_object(), "{reply}");
    assert_eq!(ok["layers"][0]["name"], "Calibrated routes");
    let r3 = by_route(&ok["layers"][0], "R3");
    assert_eq!(r3["source"], "ledger");
    assert_eq!(r3["pk_to"], "ПК 16+14.30");
    assert!((r3["st_to"].as_f64().unwrap() - 1614.3).abs() <= 1e-3);
    assert_eq!(r3["equations"], 2);
    // ошибка сборки R4 - в таблице, пояснение по-английски, код как есть
    let e = &ok["tables"][0]["geojson"]["features"][0]["properties"];
    assert_eq!(e["rl_error"], "route_gap");
    assert_eq!(
        e["rl_message"],
        "the route falls apart into 2 pieces, the nearest gap 15.000 exceeds the tolerance 0.01"
    );
    assert!(
        ok["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m == "Ledger: chainage systems 1, errors 0"),
        "{}",
        ok["messages"]
    );
}

#[test]
fn about_and_unknown_command() {
    let reply = ask(&json!({"abi": 1, "kind": "command", "id": "about", "lang": "ru"}));
    assert_eq!(reply["ok"]["title"], "Routeliner");
    let reply = ask(&json!({"abi": 1, "kind": "command", "id": "nope", "lang": "en"}));
    assert!(reply["error"].is_string());
}

#[test]
fn plugin_json_is_current() {
    let p = Routeliner::new().unwrap();
    let text = manifest(&p.catalog).unwrap();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/plugin.json");
    if std::env::var_os("ROUTELINER_WRITE_MANIFEST").is_some() {
        std::fs::write(path, &text).unwrap();
    }
    let on_disk = std::fs::read_to_string(path).unwrap_or_default();
    assert!(
        on_disk == text,
        "plugin.json устарел: ROUTELINER_WRITE_MANIFEST=1 cargo test"
    );
    let m: J = serde_json::from_str(&text).unwrap();
    let ids: Vec<&str> = m["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["check-routes", "calibrate-routes"]);
}
