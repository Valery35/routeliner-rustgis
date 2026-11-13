// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 2.0 или новее.
// SPDX-License-Identifier: GPL-2.0-or-later

//! Инструменты: склейка ядра со значениями прогона модуля
//! (`routeliner/processing` и `routeliner/layers` модуля QGIS).
//!
//! Значения и выходы - в формате прогонов модуля (`fixtures/tool_runs.json`):
//! линии `[{"id", "parts", "attrs"}]`, точки и таблицы
//! `[{"id", "xyz", "attrs"}]`, выход `{"geometry", "fields", "features"}`.

pub mod common;
pub mod prep;

use rustgis_kit::{Failure, Lang, Tool};
use serde_json::Value as J;

/// Инструменты, перенесённые в расширение, по имени Processing.
pub const DONE: &[&str] = &["check_routes", "calibrate_routes"];

/// Расчёт инструмента: выходы по ключам и сообщения на языке показа.
pub fn run(tool: &Tool, values: &J, lang: Lang) -> Result<(J, Vec<String>), Failure> {
    let mut cx = common::Cx::new(tool, values, lang);
    let out = match tool.name.as_str() {
        "check_routes" => prep::check_routes(&mut cx)?,
        "calibrate_routes" => prep::calibrate_routes(&mut cx)?,
        other => return Err(Failure::Run(other.to_string())),
    };
    Ok((out, cx.messages()))
}
