// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 3.0 или новее.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Постановка событий на маршруты (`routeliner/core/events.py`).
//!
//! Запись события несёт сырые значения из таблицы, текст пикета или
//! число. Цепочка: разбор записи, пикет, мера через систему пикетажа
//! маршрута, координаты через геометрию маршрута. Ошибка любого шага
//! уходит в список ошибок с ключом записи.

use std::collections::HashMap;

use indexmap::IndexMap;
use serde_json::Value as J;

use super::chainage::{ChainageSystem, SystemName};
use super::errors::{CoreError, ErrorCode, rid_text};
use super::route::{P3, RouteGeometry};
use super::stations::{Station, StationParser};
use crate::pyfmt::Msg;

/// Маршруты по ID в порядке первого появления.
pub type Routes = IndexMap<String, RouteGeometry>;

#[derive(Debug, Clone, PartialEq)]
pub struct EventRecord {
    pub key: Option<i64>,
    pub route_id: Option<String>,
    pub raw_from: J,
    pub raw_to: J,
    pub offset: f64,
    pub section: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PointResult {
    pub key: Option<i64>,
    pub route_id: String,
    pub m: f64,
    pub station: f64,
    pub x: f64,
    pub y: f64,
    pub z: Option<f64>,
    pub azimuth: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LineResult {
    pub key: Option<i64>,
    pub route_id: String,
    pub m_from: f64,
    pub m_to: f64,
    pub st_from: f64,
    pub st_to: f64,
    pub pieces: Vec<Vec<P3>>,
    /// «От» и «до» шли против направления маршрута.
    pub swapped: bool,
    /// Мера по оси для вершин каждого куска.
    pub piece_m: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocateResult {
    pub key: Option<i64>,
    pub route_id: String,
    pub m: f64,
    pub station: f64,
    /// Со знаком, плюс влево.
    pub offset: f64,
    pub distance: f64,
}

pub struct EventLocator<'a> {
    pub routes: &'a Routes,
    pub parser: StationParser,
    /// Системы пикетажа по маршрутам.
    pub systems: HashMap<String, ChainageSystem>,
    /// Пикет начала маршрутов без ведомости.
    pub start_station: f64,
}

impl<'a> EventLocator<'a> {
    pub fn new(routes: &'a Routes, parser: StationParser) -> Self {
        Self {
            routes,
            parser,
            systems: HashMap::new(),
            start_station: 0.0,
        }
    }

    /// Система маршрута. Без ведомости - по длине от пикета начала.
    pub fn system_for(&mut self, rid: &str) -> Option<&ChainageSystem> {
        if !self.systems.contains_key(rid) {
            let r = self.routes.get(rid)?;
            self.systems.insert(
                rid.to_string(),
                ChainageSystem::from_length(
                    SystemName::Text("по длине".into()),
                    r.length,
                    self.start_station,
                ),
            );
        }
        self.systems.get(rid)
    }

    fn measure(
        &mut self,
        raw: &J,
        rid: Option<&str>,
        section: Option<i64>,
        prev: &mut HashMap<Option<String>, Station>,
    ) -> Result<(f64, f64), CoreError> {
        let route = rid.and_then(|r| self.routes.get(r));
        let (Some(rid), Some(route)) = (rid, route) else {
            return Err(CoreError::new(
                ErrorCode::RouteNotFound,
                Msg::new("маршрут «{rid}» не найден").s("rid", rid_text(rid)),
            ));
        };
        let key = Some(rid.to_string());
        let st = self.parser.parse(raw, prev.get(&key))?;
        prev.insert(key, st);
        let length = route.length;
        let parser = self.parser;
        let Some(sys) = self.system_for(rid) else {
            return Err(CoreError::new(
                ErrorCode::RouteNotFound,
                Msg::new("маршрут «{rid}» не найден").s("rid", rid),
            ));
        };
        let m = sys.to_measure(st.value, section)?;
        if m < -1e-6 || m > length + 1e-6 {
            return Err(CoreError::new(
                ErrorCode::MeasureOutOfRange,
                Msg::new("пикет {pk} даёт меру {m:.2f} при длине маршрута {L:.2f}")
                    .s("pk", parser.format(st.value))
                    .f("m", m)
                    .f("L", length),
            ));
        }
        Ok((m.clamp(0.0, length), st.value))
    }

    /// Точечные события. Итог - по маршрутам в порядке первого появления.
    pub fn locate_points(&mut self, records: &[EventRecord]) -> (Vec<PointResult>, Vec<CoreError>) {
        let mut errors = Vec::new();
        let mut prev = HashMap::new();
        let mut by_route: IndexMap<String, Vec<(&EventRecord, f64, f64)>> = IndexMap::new();
        for r in records {
            match self.measure(&r.raw_from, r.route_id.as_deref(), r.section, &mut prev) {
                Ok((m, st)) => {
                    let rid = r.route_id.clone().unwrap_or_default();
                    by_route.entry(rid).or_default().push((r, m, st));
                }
                Err(e) => errors.push(e.with_key(r.key, r.route_id.as_deref())),
            }
        }
        let mut out = Vec::new();
        for (rid, items) in by_route {
            let route = &self.routes[&rid];
            let ms: Vec<f64> = items.iter().map(|(_, m, _)| *m).collect();
            let offs: Vec<f64> = items.iter().map(|(r, _, _)| r.offset).collect();
            let p = route.points_at(&ms, &offs);
            for ((r, m, st), at) in items.into_iter().zip(p) {
                out.push(PointResult {
                    key: r.key,
                    route_id: rid.clone(),
                    m,
                    station: st,
                    x: at.x,
                    y: at.y,
                    z: (!at.z.is_nan()).then_some(at.z),
                    azimuth: at.azimuth,
                });
            }
        }
        (out, errors)
    }

    /// Линейные события по порядку записей.
    pub fn locate_lines(&mut self, records: &[EventRecord]) -> (Vec<LineResult>, Vec<CoreError>) {
        let mut out = Vec::new();
        let mut errors = Vec::new();
        let mut prev = HashMap::new();
        for r in records {
            let rid = r.route_id.as_deref();
            let ab = self
                .measure(&r.raw_from, rid, r.section, &mut prev)
                .and_then(|a| Ok((a, self.measure(&r.raw_to, rid, r.section, &mut prev)?)));
            let ((mut m0, mut s0), (mut m1, mut s1)) = match ab {
                Ok(v) => v,
                Err(e) => {
                    errors.push(e.with_key(r.key, rid));
                    continue;
                }
            };
            let swapped = m0 > m1;
            if swapped {
                std::mem::swap(&mut m0, &mut m1);
                std::mem::swap(&mut s0, &mut s1);
            }
            if m1 - m0 <= 1e-6 {
                errors.push(
                    CoreError::new(ErrorCode::FromGeTo, Msg::new("участок нулевой длины"))
                        .with_key(r.key, rid),
                );
                continue;
            }
            let rid = rid.unwrap_or_default();
            match self.routes[rid].substring_m(m0, m1, r.offset) {
                Ok(pieces) => {
                    let (pieces, piece_m) = pieces.into_iter().unzip();
                    out.push(LineResult {
                        key: r.key,
                        route_id: rid.to_string(),
                        m_from: m0,
                        m_to: m1,
                        st_from: s0,
                        st_to: s1,
                        pieces,
                        swapped,
                        piece_m,
                    });
                }
                Err(e) => errors.push(e.with_key(r.key, Some(rid))),
            }
        }
        (out, errors)
    }

    /// Обратная задача: ближайший маршрут, мера, пикет и смещение.
    pub fn locate_xy(
        &mut self,
        key: Option<i64>,
        x: f64,
        y: f64,
        route_ids: Option<&[String]>,
        max_distance: Option<f64>,
    ) -> Result<LocateResult, CoreError> {
        let ids: Vec<String> = match route_ids {
            Some(ids) => ids.to_vec(),
            None => self.routes.keys().cloned().collect(),
        };
        let mut best: Option<(String, super::route::Projection)> = None;
        for rid in ids {
            let Some(route) = self.routes.get(&rid) else {
                continue;
            };
            let Ok(pr) = route.locate(x, y) else {
                continue;
            };
            if best
                .as_ref()
                .is_none_or(|(_, b)| pr.offset.abs() < b.offset.abs())
            {
                best = Some((rid, pr));
            }
        }
        let Some((rid, pr)) = best else {
            return Err(CoreError::new(
                ErrorCode::RouteNotFound,
                Msg::new("нет маршрута для привязки"),
            )
            .with_key(key, None));
        };
        if let Some(r) = max_distance
            && pr.offset.abs() > r
        {
            return Err(CoreError::new(
                ErrorCode::MeasureOutOfRange,
                Msg::new("ближайший маршрут «{rid}» в {d:.2f} м, дальше радиуса {r:g}")
                    .s("rid", rid.clone())
                    .f("d", pr.offset.abs())
                    .f("r", r),
            )
            .with_key(key, Some(&rid)));
        }
        let st = match self.system_for(&rid).map(|s| s.to_station(pr.m)) {
            Some(Ok(st)) => st,
            Some(Err(e)) => return Err(e.with_key(key, Some(&rid))),
            None => f64::NAN,
        };
        Ok(LocateResult {
            key,
            route_id: rid,
            m: pr.m,
            station: st,
            offset: pr.offset,
            distance: pr.offset.abs(),
        })
    }
}
