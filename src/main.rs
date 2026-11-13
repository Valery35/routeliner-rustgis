// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 2.0 или новее.
// SPDX-License-Identifier: GPL-2.0-or-later

//! Запуск: `routeliner-rustgis rustgis-plugin` делает RustGIS по
//! `plugin.json`, `routeliner-rustgis manifest` печатает `plugin.json`
//! с разделом `tools`.

use std::process::ExitCode;

use routeliner_rustgis::product::{Routeliner, manifest};

fn main() -> ExitCode {
    let p = match Routeliner::new() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("tools.json: {e}");
            return ExitCode::FAILURE;
        }
    };
    match std::env::args().nth(1).as_deref() {
        Some("rustgis-plugin") => rustgis_kit::serve_stdio(&p),
        Some("manifest") => match manifest(&p.catalog) {
            Ok(text) => {
                print!("{text}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!(
                "Routeliner для RustGIS. Запуск: routeliner-rustgis rustgis-plugin | manifest"
            );
            ExitCode::from(2)
        }
    }
}
