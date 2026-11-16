// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 3.0 или новее.
// SPDX-License-Identifier: GPL-3.0-or-later

//! 3.01 и 3.02 - динамические слои (`layers/binder.py` модуля) на сессиях
//! расширений RustGIS 0.7.0 (#1553).
//!
//! Инструмент RustGIS запускает программу заново на каждый прогон, а
//! сессию получают только команды. Поэтому динамический слой - команда
//! с окном полей (`form`, #1723): поля те же, что у 2.01 и 2.02, и имя
//! слоя. Команда считает события тем же расчётом, что инструмент, кладёт
//! в проект слой событий и слой ошибок в группу Routeliner и запоминает
//! привязку. Событие сессии `layers` RustGIS шлёт и на правку объектов,
//! и на отмену. По нему привязка заново читает исходные слои и, если их
//! отпечаток изменился, пересчитывает и заменяет оба слоя. Свои слои
//! отпечаток не меняют, поэтому замена не зацикливается.
//!
//! Привязка живёт в программе сессии, до закрытия RustGIS или
//! выключения расширения. Хранить её в проекте RustGIS расширению негде.
//! Удалённый слой событий снимает привязку, удалённый исходный слой
//! останавливает пересчёт, как у модуля.

use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use rustgis_kit::{Call, Lang, Product};
use serde_json::{Value as J, json};

use crate::pyfmt::Msg;

/// Динамический инструмент модуля и инструмент его расчёта.
pub const LIVE: &[(&str, &str)] = &[
    ("live_point_events", "point_events"),
    ("live_line_events", "line_events"),
];

/// Группа слоёв проекта, как у модуля.
pub const GROUP: &str = "Routeliner";

/// Поля-слои инструментов событий: их объекты читаются из проекта.
const SOURCES: &[&str] = &["ROUTES", "EVENTS", "LEDGER"];

/// Привязка: параметры расчёта, исходные слои и слои итога по id RustGIS.
#[derive(Debug, Clone)]
pub struct Binding {
    pub live: &'static str,
    pub base: &'static str,
    pub name: String,
    /// Значения окна по именам полей RustGIS, без имени слоя.
    pub params: J,
    /// Ключ поля-слоя и id слоя.
    pub sources: Vec<(String, u64)>,
    pub out: Vec<u64>,
    pub stamp: u64,
    pub lang: Lang,
}

/// Привязки сессии.
#[derive(Debug, Default)]
pub struct Live {
    pub bindings: RefCell<Vec<Binding>>,
}

/// Имя слоя ошибок.
fn errors_name(name: &str, lang: Lang) -> String {
    match lang {
        Lang::Ru => format!("{name}, ошибки"),
        Lang::En => format!("{name}, errors"),
    }
}

/// Слои проекта: id, имя.
fn layers(host: &mut dyn Call) -> Result<Vec<(u64, String)>, String> {
    let v = host.call("host.layers", json!({}))?;
    Ok(v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|l| Some((l["id"].as_u64()?, l["name"].as_str()?.to_string())))
        .collect())
}

/// Объекты исходных слоёв по их текущим именам и отпечаток.
fn read_sources(
    host: &mut dyn Call,
    b: &Binding,
    all: &[(u64, String)],
) -> Result<(J, J, u64), String> {
    let mut params = b.params.clone();
    let mut fcs = serde_json::Map::new();
    let mut h = DefaultHasher::new();
    b.params.to_string().hash(&mut h);
    for (key, id) in &b.sources {
        let Some((_, name)) = all.iter().find(|(i, _)| i == id) else {
            return Err(key.clone());
        };
        let fc = host.call("host.features", json!({"layer": id}))?;
        fc.to_string().hash(&mut h);
        params[key.to_lowercase()] = json!(name);
        fcs.insert(name.clone(), fc);
    }
    Ok((params, J::Object(fcs), h.finish()))
}

/// Расчёт привязки: ответ инструмента 2.01 или 2.02.
fn compute(product: &dyn Product, b: &Binding, params: J, fcs: J) -> J {
    let id = product
        .catalog()
        .tool(b.base)
        .map(|t| t.rg_id())
        .unwrap_or_default();
    let lang = match b.lang {
        Lang::Ru => "ru",
        Lang::En => "en",
    };
    let req = json!({"kind": "tool", "id": id, "lang": lang, "params": params, "layers": fcs});
    rustgis_kit::runner::run_tool(product, &req, b.lang)
}

/// Слои итога - в проект, в группу Routeliner. Ответ - их id.
fn put(host: &mut dyn Call, b: &Binding, reply: &J) -> Result<Vec<u64>, String> {
    let ok = &reply["ok"];
    let mut ids = Vec::new();
    let sets = [
        (ok["layers"][0]["geojson"].clone(), b.name.clone()),
        (
            ok["tables"][0]["geojson"].clone(),
            errors_name(&b.name, b.lang),
        ),
    ];
    for (geojson, name) in sets {
        if geojson.is_null() {
            continue;
        }
        let r = host.call("host.addLayer", json!({"name": name, "geojson": geojson}))?;
        if let Some(id) = r["layer"].as_u64() {
            ids.push(id);
        }
    }
    if !ids.is_empty()
        && host
            .call("layer.group", json!({"layer": ids, "group": GROUP}))
            .is_err()
    {
        host.call("group.add", json!({"name": GROUP, "layers": ids}))?;
    }
    Ok(ids)
}

/// Строка итога расчёта: последняя строка сообщений инструмента.
fn status(reply: &J) -> String {
    reply["ok"]["messages"]
        .as_array()
        .and_then(|m| m.last())
        .and_then(J::as_str)
        .unwrap_or("")
        .to_string()
}

impl Live {
    /// Команда 3.01 или 3.02: привязка, первый расчёт, отчёт для окна.
    pub fn start(
        &self,
        product: &dyn Product,
        host: &mut dyn Call,
        live: &str,
        params: &J,
        lang: Lang,
    ) -> J {
        let Some(&(live, base)) = LIVE.iter().find(|(l, _)| *l == live) else {
            return json!({"error": live});
        };
        let tool = product.catalog().tool(live);
        let tr = |s: &str| tool.map_or(s.to_string(), |t| t.translate(s, lang).to_string());
        let all = match layers(host) {
            Ok(a) => a,
            Err(e) => return json!({"error": e}),
        };
        let mut sources = Vec::new();
        for key in SOURCES {
            // поле-слой окна - имя слоя, а может прийти и id
            let v = &params[key.to_lowercase()];
            if v.is_null() || v.as_str() == Some("") {
                continue;
            }
            let id = v.as_u64().or_else(|| {
                let name = v.as_str()?;
                all.iter().find(|(_, n)| n == name).map(|(i, _)| *i)
            });
            match id {
                Some(id) => sources.push(((*key).to_string(), id)),
                None => {
                    let m = Msg::new("Не задан слой для параметра «{p}»").s("p", *key);
                    return json!({"error": m.render(&tr)});
                }
            }
        }
        let default = if base == "line_events" {
            "Участки"
        } else {
            "События"
        };
        let name = params["name"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .map_or_else(|| tr(default), |s| s.trim().to_string());
        let mut p = params.clone();
        if let Some(o) = p.as_object_mut() {
            o.remove("name");
        }
        let mut b = Binding {
            live,
            base,
            name,
            params: p,
            sources,
            out: Vec::new(),
            stamp: 0,
            lang,
        };
        let (params, fcs, stamp) = match read_sources(host, &b, &all) {
            Ok(r) => r,
            Err(e) => return json!({"error": e}),
        };
        let reply = compute(product, &b, params, fcs);
        if reply.get("error").is_some() {
            return reply;
        }
        match put(host, &b, &reply) {
            Ok(ids) => b.out = ids,
            Err(e) => return json!({"error": e}),
        }
        b.stamp = stamp;
        let report = Msg::new("Динамический слой «{name}»: {status}")
            .s("name", b.name.clone())
            .s("status", status(&reply))
            .render(&tr);
        self.bindings.borrow_mut().push(b);
        json!({"ok": {"report": report}})
    }

    /// Событие `layers`: пересчёт привязок с изменёнными исходными слоями.
    pub fn changed(&self, product: &dyn Product, host: &mut dyn Call) {
        if self.bindings.borrow().is_empty() {
            return;
        }
        let Ok(all) = layers(host) else {
            return;
        };
        let ids: Vec<u64> = all.iter().map(|(i, _)| *i).collect();
        let mut keep = Vec::new();
        let list = self.bindings.borrow().clone();
        for mut b in list {
            // слой событий удалён - привязка снимается
            if b.out.first().is_none_or(|o| !ids.contains(o)) {
                continue;
            }
            let (params, fcs, stamp) = match read_sources(host, &b, &all) {
                Ok(r) => r,
                // исходного слоя нет - пересчёт остановлен
                Err(_) => continue,
            };
            if stamp != b.stamp {
                let reply = compute(product, &b, params, fcs);
                if reply.get("ok").is_some() {
                    let old: Vec<u64> = b.out.iter().copied().filter(|o| ids.contains(o)).collect();
                    if !old.is_empty() {
                        let _ = host.call("layer.remove", json!({"layer": old}));
                    }
                    if let Ok(new) = put(host, &b, &reply) {
                        b.out = new;
                    }
                }
                b.stamp = stamp;
            }
            keep.push(b);
        }
        *self.bindings.borrow_mut() = keep;
    }
}
