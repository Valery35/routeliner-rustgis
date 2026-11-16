// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 3.0 или новее.
// SPDX-License-Identifier: GPL-3.0-or-later

//! 3.01 и 3.02 на поддельном RustGIS: слои проекта, `host.layers`,
//! `host.features`, `host.addLayer`, `layer.remove`, `layer.group`,
//! `group.add` - как в RustGIS 0.7.0. Слой считается тем же расчётом, что
//! 2.01 и 2.02, пересчитывается после правки таблицы и маршрута, без
//! правок не трогается, удалённый слой событий снимает привязку.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use routeliner_rustgis::product::Routeliner;
use rustgis_kit::geo::Utm;
use rustgis_kit::{Call, Lang, Product};
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

fn lines_layer(items: &J) -> J {
    let u = Utm::at(57.0, 58.0);
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
            json!({"type": "Feature", "row": f["id"], "properties": f["attrs"],
                   "geometry": {"type": "MultiLineString", "coordinates": parts}})
        })
        .collect();
    json!({"type": "FeatureCollection", "features": feats})
}

fn table_layer(items: &J) -> J {
    let feats: Vec<J> = items
        .as_array()
        .unwrap()
        .iter()
        .map(|f| json!({"type": "Feature", "row": f["id"], "properties": f["attrs"], "geometry": null}))
        .collect();
    json!({"type": "FeatureCollection", "features": feats})
}

/// Проект RustGIS в памяти.
#[derive(Default)]
struct Fake {
    /// id, имя, объекты, группа.
    layers: Vec<(u64, String, J, Option<String>)>,
    groups: Vec<String>,
    next: u64,
    added: usize,
}

impl Fake {
    fn add(&mut self, name: &str, fc: J) -> u64 {
        self.next += 1;
        self.layers.push((self.next, name.to_string(), fc, None));
        self.next
    }

    fn by_name(&self, name: &str) -> &(u64, String, J, Option<String>) {
        self.layers.iter().find(|l| l.1 == name).unwrap()
    }

    fn fc_mut(&mut self, name: &str) -> &mut J {
        &mut self.layers.iter_mut().find(|l| l.1 == name).unwrap().2
    }
}

fn ids(v: &J) -> Vec<u64> {
    match v {
        J::Array(a) => a.iter().filter_map(J::as_u64).collect(),
        other => other.as_u64().into_iter().collect(),
    }
}

impl Call for Fake {
    fn call(&mut self, op: &str, p: J) -> Result<J, String> {
        match op {
            "host.layers" => Ok(json!(
                self.layers
                    .iter()
                    .map(|l| json!({"id": l.0, "name": l.1, "group": l.3}))
                    .collect::<Vec<_>>()
            )),
            "host.features" => {
                let l = p["layer"].as_u64().unwrap();
                self.layers
                    .iter()
                    .find(|x| x.0 == l)
                    .map(|x| x.2.clone())
                    .ok_or_else(|| "no such layer".to_string())
            }
            "host.addLayer" => {
                self.added += 1;
                let id = self.add(p["name"].as_str().unwrap(), p["geojson"].clone());
                Ok(json!({"layer": id, "skipped": 0}))
            }
            "layer.remove" => {
                let gone = ids(&p["layer"]);
                self.layers.retain(|l| !gone.contains(&l.0));
                Ok(J::Null)
            }
            "layer.group" => {
                let g = p["group"].as_str().unwrap().to_string();
                if !self.groups.contains(&g) {
                    return Err("no such group".into());
                }
                for id in ids(&p["layer"]) {
                    if let Some(l) = self.layers.iter_mut().find(|l| l.0 == id) {
                        l.3 = Some(g.clone());
                    }
                }
                Ok(J::Null)
            }
            "group.add" => {
                let g = p["name"].as_str().unwrap().to_string();
                self.groups.push(g.clone());
                for id in ids(&p["layers"]) {
                    if let Some(l) = self.layers.iter_mut().find(|l| l.0 == id) {
                        l.3 = Some(g.clone());
                    }
                }
                Ok(J::Null)
            }
            other => Err(format!("unknown {other}")),
        }
    }
}

/// Значения 2.01 прогона `ledger` - поля окна RustGIS.
fn params(r: &J) -> J {
    let v = &r["values"];
    let mut params = json!({"routes": "Routes", "events": "Events", "ledger": "Ledger"});
    for k in [
        "ROUTE_ID",
        "EV_ROUTE",
        "EV_FROM",
        "EV_OFFSET",
        "EV_SECTION",
        "LG_ROUTE",
        "LG_STATION",
        "LG_MEASURE",
        "LG_AHEAD",
        "LG_SYSTEM",
        "SYSTEM",
    ] {
        params[k.to_lowercase()] = v[k].clone();
    }
    params
}

fn project(r: &J) -> Fake {
    let mut f = Fake::default();
    f.add("Routes", lines_layer(&r["input"]["ROUTES"]));
    f.add("Events", table_layer(&r["input"]["EVENTS"]));
    f.add("Ledger", table_layer(&r["input"]["LEDGER"]));
    f
}

/// Тот же расчёт инструментом 2.01 по текущему проекту.
fn tool_reply(p: &Routeliner, f: &Fake, params: &J) -> J {
    let mut prm = params.clone();
    prm["errors"] = json!(true);
    let req = json!({"kind": "tool", "id": "point-events", "lang": "ru", "params": prm,
        "layers": {"Routes": f.by_name("Routes").2, "Events": f.by_name("Events").2,
                   "Ledger": f.by_name("Ledger").2}});
    rustgis_kit::runner::run_tool(p, &req, Lang::Ru)
}

fn props(fc: &J) -> Vec<J> {
    fc["features"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["properties"].clone())
        .collect()
}

#[test]
fn live_point_events_follow_edits() {
    let r = run("point_events", "ledger");
    let p = Routeliner::new().unwrap();
    let mut f = project(&r);
    let prm = params(&r);
    let mut cmd = prm.clone();
    cmd["name"] = json!("Живые события");
    let reply = p.command(
        &mut f,
        &json!({"kind": "command", "id": "live-point-events", "params": cmd}),
        Lang::Ru,
    );
    let report = reply["ok"]["report"]
        .as_str()
        .unwrap_or_else(|| panic!("{reply}"));
    assert!(
        report.starts_with("Динамический слой «Живые события»: Итого"),
        "{report}"
    );

    // первый расчёт - тот же, что 2.01, слои в группе Routeliner
    let want = tool_reply(&p, &f, &prm);
    let live = f.by_name("Живые события").clone();
    let errs = f.by_name("Живые события, ошибки").clone();
    assert_eq!(props(&live.2), props(&want["ok"]["layers"][0]["geojson"]));
    assert_eq!(
        live.2["features"],
        want["ok"]["layers"][0]["geojson"]["features"]
    );
    assert_eq!(props(&errs.2), props(&want["ok"]["tables"][0]["geojson"]));
    assert_eq!(live.3.as_deref(), Some("Routeliner"));
    assert_eq!(errs.3.as_deref(), Some("Routeliner"));
    assert_eq!(f.added, 2);

    // событие без правок исходных слоёв - слои не трогаются
    p.event(&mut f, "layers");
    assert_eq!(f.added, 2);

    // правка таблицы: первая запись переписана на ПК 3+00
    let fc = f.fc_mut("Events");
    let first = &mut fc["features"][0]["properties"];
    let eid = first["eid"].clone();
    first["pk"] = json!("ПК 3+00");
    p.event(&mut f, "layers");
    assert_eq!(f.added, 4);
    assert!(
        f.layers.iter().all(|l| l.0 != live.0 && l.0 != errs.0),
        "прежние слои удалены"
    );
    let now = f.by_name("Живые события").clone();
    let rec = now.2["features"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["properties"]["eid"] == eid)
        .unwrap();
    assert_eq!(rec["properties"]["rl_pk"], "ПК 3+00.00");
    assert_eq!(
        props(&now.2),
        props(&tool_reply(&p, &f, &prm)["ok"]["layers"][0]["geojson"])
    );
    assert_eq!(now.3.as_deref(), Some("Routeliner"));

    // правка маршрута: объекты R2 удалены, его события уходят в ошибки
    let before = props(&f.by_name("Живые события, ошибки").2).len();
    let routes = f.fc_mut("Routes");
    routes["features"]
        .as_array_mut()
        .unwrap()
        .retain(|x| x["properties"]["route_id"] != "R2");
    p.event(&mut f, "layers");
    let errs = props(&f.by_name("Живые события, ошибки").2);
    assert!(errs.len() > before);
    assert!(
        errs.iter()
            .any(|e| e["rl_route"] == "R2" && e["rl_error"] == "route_not_found")
    );
    assert_eq!(
        errs,
        props(&tool_reply(&p, &f, &prm)["ok"]["tables"][0]["geojson"])
    );

    // слой событий удалён - привязка снята, новых слоёв нет
    let id = f.by_name("Живые события").0;
    f.layers.retain(|l| l.0 != id);
    let added = f.added;
    f.fc_mut("Events")["features"][0]["properties"]["pk"] = json!("ПК 4+00");
    p.event(&mut f, "layers");
    assert_eq!(f.added, added);
    assert!(p.live.bindings.borrow().is_empty());
}

#[test]
fn live_sections_match_line_events() {
    let r = run("line_events", "ledger");
    let p = Routeliner::new().unwrap();
    let mut f = project(&r);
    let v = &r["values"];
    let mut prm = json!({"routes": "Routes", "events": "Events", "ledger": "Ledger"});
    for k in [
        "ROUTE_ID",
        "EV_ROUTE",
        "EV_FROM",
        "EV_TO",
        "EV_OFFSET",
        "LG_ROUTE",
        "LG_STATION",
        "LG_MEASURE",
        "LG_AHEAD",
        "LG_SYSTEM",
        "SYSTEM",
    ] {
        prm[k.to_lowercase()] = v[k].clone();
    }
    let reply = p.command(
        &mut f,
        &json!({"kind": "command", "id": "live-line-events", "params": prm}),
        Lang::En,
    );
    assert!(reply["ok"]["report"].is_string(), "{reply}");
    // имя по умолчанию - «Sections», расчёт - как у 2.02
    let live = f.by_name("Sections").clone();
    let mut tp = prm.clone();
    tp["errors"] = json!(true);
    let req = json!({"kind": "tool", "id": "line-events", "lang": "en", "params": tp,
        "layers": {"Routes": f.by_name("Routes").2, "Events": f.by_name("Events").2,
                   "Ledger": f.by_name("Ledger").2}});
    let want = rustgis_kit::runner::run_tool(&p, &req, Lang::En);
    assert_eq!(
        live.2["features"],
        want["ok"]["layers"][0]["geojson"]["features"]
    );
    assert!(f.layers.iter().any(|l| l.1 == "Sections, errors"));
}
