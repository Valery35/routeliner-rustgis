// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 3.0 или новее.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Инструменты: склейка ядра со значениями прогона модуля
//! (`routeliner/processing` и `routeliner/layers` модуля QGIS).
//!
//! Значения и выходы - в формате прогонов модуля (`fixtures/tool_runs.json`):
//! линии `[{"id", "parts", "attrs"}]`, точки и таблицы
//! `[{"id", "xyz", "attrs"}]`, выход `{"geometry", "fields", "features"}`.

pub mod common;
pub mod demo;
pub mod events;
pub mod expr;
pub mod prep;
pub mod profile;

use rustgis_kit::{Failure, Lang, Tool};
use serde_json::Value as J;

/// Инструменты, перенесённые в расширение, по имени Processing, в порядке
/// панели.
pub const DONE: &[&str] = &[
    "demo",
    "check_routes",
    "calibrate_routes",
    "point_events",
    "line_events",
    "pickets",
    "locate_points",
    "profile_table",
    "profile_drawing",
];

/// Расчёт инструмента: выходы по ключам и сообщения на языке показа.
pub fn run(tool: &Tool, values: &J, lang: Lang) -> Result<(J, Vec<String>), Failure> {
    let mut cx = common::Cx::new(tool, values, lang);
    let out = match tool.name.as_str() {
        "demo" => demo::demo(&mut cx)?,
        "check_routes" => prep::check_routes(&mut cx)?,
        "calibrate_routes" => prep::calibrate_routes(&mut cx)?,
        "point_events" => events::events(&mut cx, false)?,
        "line_events" => events::events(&mut cx, true)?,
        "pickets" => events::pickets(&mut cx)?,
        "locate_points" => events::locate_points(&mut cx)?,
        "profile_table" => profile::profile_table(&mut cx)?,
        "profile_drawing" => profile::profile_drawing(&mut cx)?,
        other => return Err(Failure::Run(other.to_string())),
    };
    Ok((out, cx.messages()))
}
