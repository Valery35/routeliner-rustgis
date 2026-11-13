// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 3.0 или новее.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Routeliner для RustGIS - расширение-процесс RustGIS, второе приложение
//! продукта Routeliner рядом с модулем QGIS (`C:\Dev\routeliner`).
//!
//! - [`core`] - ядро расчёта, перенос `routeliner/core` модуля;
//! - [`tools`] - инструменты по значениям прогона модуля;
//! - [`product`] - продукт крейта `rustgis-kit`, связь с RustGIS;
//! - [`pyfmt`] - числа и шаблоны так, как их пишет Python модуля.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod core;
pub mod product;
pub mod pyfmt;
pub mod tools;
