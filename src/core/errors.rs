// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 3.0 или новее.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Коды и записи ошибок ядра (`routeliner/core/errors.py`). Ошибки -
//! значения, а не исключения: каждая запись события либо ставится на
//! карту, либо попадает в таблицу ошибок.

use crate::pyfmt::Msg;

/// Код причины. Коды не переводятся, их значения - поле `rl_error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    RouteNotFound,
    RouteGap,
    RouteBranching,
    GeographicCrs,
    MeasureOutOfRange,
    StationParseFailed,
    StationOutOfRange,
    /// Пикет попал в прямую вставку.
    StationInGap,
    /// Пикет попал в обратную вставку.
    StationAmbiguous,
    LedgerInvalid,
    FromGeTo,
}

impl ErrorCode {
    /// Значение поля `rl_error`.
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::RouteNotFound => "route_not_found",
            ErrorCode::RouteGap => "route_gap",
            ErrorCode::RouteBranching => "route_branching",
            ErrorCode::GeographicCrs => "geographic_crs",
            ErrorCode::MeasureOutOfRange => "measure_out_of_range",
            ErrorCode::StationParseFailed => "station_parse_failed",
            ErrorCode::StationOutOfRange => "station_out_of_range",
            ErrorCode::StationInGap => "station_in_gap",
            ErrorCode::StationAmbiguous => "station_ambiguous",
            ErrorCode::LedgerInvalid => "ledger_invalid",
            ErrorCode::FromGeTo => "from_ge_to",
        }
    }

    /// Все коды по порядку модуля (`error_codes` в `tools.json`).
    pub const ALL: [ErrorCode; 11] = [
        ErrorCode::RouteNotFound,
        ErrorCode::RouteGap,
        ErrorCode::RouteBranching,
        ErrorCode::GeographicCrs,
        ErrorCode::MeasureOutOfRange,
        ErrorCode::StationParseFailed,
        ErrorCode::StationOutOfRange,
        ErrorCode::StationInGap,
        ErrorCode::StationAmbiguous,
        ErrorCode::LedgerInvalid,
        ErrorCode::FromGeTo,
    ];
}

/// Ошибка с ключом записи (номер объекта источника) и маршрутом.
#[derive(Debug, Clone, PartialEq)]
pub struct CoreError {
    pub code: ErrorCode,
    pub msg: Msg,
    pub key: Option<i64>,
    pub route_id: Option<String>,
}

impl CoreError {
    pub fn new(code: ErrorCode, msg: Msg) -> Self {
        Self {
            code,
            msg,
            key: None,
            route_id: None,
        }
    }

    pub fn route(mut self, route_id: Option<&str>) -> Self {
        self.route_id = route_id.map(str::to_string);
        self
    }

    /// Python `with_key`: ключ записи, маршрут - новый или прежний.
    pub fn with_key(mut self, key: Option<i64>, route_id: Option<&str>) -> Self {
        self.key = key;
        if let Some(r) = route_id {
            self.route_id = Some(r.to_string());
        }
        self
    }
}

/// Маршрут по-питоновски для текста: нет маршрута - `None`.
pub fn rid_text(rid: Option<&str>) -> String {
    rid.unwrap_or("None").to_string()
}
