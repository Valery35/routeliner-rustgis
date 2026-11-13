// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 3.0 или новее.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Сверка с прогонами модуля QGIS: `fixtures/tool_runs.json` (собирает
//! `tools/make_tool_runs.py` модуля). Каждый прогон перенесённого
//! инструмента считается расширением на тех же значениях, выходы
//! сравниваются поле в поле и вершина в вершину с допусками прогона.
//! Пояснения `rl_message` сравниваются по-русски, как их пишет модуль.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use routeliner_rustgis::product::Routeliner;
use routeliner_rustgis::tools::DONE;
use rustgis_kit::{Lang, Product};
use serde_json::Value as J;

const RUNS: &str = include_str!("../fixtures/tool_runs.json");

struct Tol {
    xy: f64,
    m: f64,
    fields: f64,
}

/// Наибольшее отклонение чисел по видам: поля, координаты, M.
#[derive(Default)]
struct Worst {
    fields: f64,
    xy: f64,
    m: f64,
}

fn compare(exp: &J, got: &J, path: &str, tol: &Tol, worst: &mut Worst, bad: &mut Vec<String>) {
    match (exp, got) {
        (J::Number(a), J::Number(b)) => {
            let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            let d = (a - b).abs();
            let (t, w) = if path.contains("/measure") {
                (tol.m, &mut worst.m)
            } else if path.contains("geometry") {
                (tol.xy, &mut worst.xy)
            } else {
                (tol.fields, &mut worst.fields)
            };
            *w = w.max(d);
            if d > t {
                bad.push(format!("{path}: {a} != {b}"));
            }
        }
        (J::Array(a), J::Array(b)) => {
            if a.len() != b.len() {
                bad.push(format!("{path}: длина {} != {}", a.len(), b.len()));
                return;
            }
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                compare(x, y, &format!("{path}/{i}"), tol, worst, bad);
            }
        }
        (J::Object(a), J::Object(b)) => {
            for (k, x) in a {
                match b.get(k) {
                    Some(y) => compare(x, y, &format!("{path}/{k}"), tol, worst, bad),
                    None => bad.push(format!("{path}/{k}: нет")),
                }
            }
            for k in b.keys() {
                if !a.contains_key(k) {
                    bad.push(format!("{path}/{k}: лишний"));
                }
            }
        }
        (a, b) if a == b => {}
        (a, b) => bad.push(format!("{path}: {a} != {b}")),
    }
}

#[test]
fn tool_runs_match_the_module() {
    let p = Routeliner::new().unwrap();
    let runs: J = serde_json::from_str(RUNS).unwrap();
    let mut bad = Vec::new();
    let mut checked = Vec::new();
    let mut worst = Worst::default();
    for run in runs["runs"].as_array().unwrap() {
        let name = run["tool"].as_str().unwrap();
        if !DONE.contains(&name) {
            continue;
        }
        let case = run["case"].as_str().unwrap();
        let tool = p.catalog.tool(name).unwrap();
        let mut values = run["values"].clone();
        for (k, v) in run["input"].as_object().unwrap() {
            values[k] = v.clone();
        }
        let t = &run["tolerance"];
        let tol = Tol {
            xy: t["xy"].as_f64().unwrap(),
            m: t["m"].as_f64().unwrap(),
            fields: t["fields"].as_f64().unwrap(),
        };
        match p.compute(tool, &values, Lang::Ru) {
            Ok((out, _)) => compare(
                &run["output"],
                &out,
                &format!("{name}.{case}"),
                &tol,
                &mut worst,
                &mut bad,
            ),
            Err(e) => bad.push(format!("{name}.{case}: {e:?}")),
        }
        checked.push(format!("{name}.{case}"));
    }
    eprintln!(
        "прогонов {}, наибольшее отклонение: поля {:e}, координаты {:e} м, M {:e} м",
        checked.len(),
        worst.fields,
        worst.xy,
        worst.m
    );
    let shown: Vec<&String> = bad.iter().take(40).collect();
    assert!(bad.is_empty(), "расхождений {}:\n{shown:#?}", bad.len());
    // все прогоны перенесённых инструментов сверены
    let expected = runs["runs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| DONE.contains(&r["tool"].as_str().unwrap()))
        .count();
    assert_eq!(checked.len(), expected);
    assert!(worst.fields <= 1e-3 && worst.xy <= 1e-3 && worst.m <= 1e-3);
}
