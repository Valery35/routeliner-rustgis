// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 2.0 или новее.
// SPDX-License-Identifier: GPL-2.0-or-later

//! Ядро расчёта, перенос `routeliner/core` модуля QGIS: пикеты, система
//! пикетажа с уравнениями, геометрия маршрута с M, сборка маршрута из
//! частей, постановка событий, коды ошибок. Без RustGIS и без ввода-вывода.

pub mod assembler;
pub mod chainage;
pub mod errors;
pub mod events;
pub mod route;
pub mod stations;
