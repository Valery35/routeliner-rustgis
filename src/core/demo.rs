// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 3.0 или новее.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Демонстрационный пример с эталонным ответом (`routeliner/core/demo.py`).
//!
//! Геометрия задана аналитически, прямыми и дугами окружности, поэтому
//! истинные координаты любой меры известны без участия расчёта. Маршруты
//! в метрах EPSG:32640, район Перми.
//!
//! - R1 - прямая 2000 м на восток с отметками Z 150-170.
//! - R2 - прямая 500 м, дуга R=300 м на 90° влево, прямая 500 м. Модуль
//!   хранит её настоящей дугой, здесь она сразу нарезана так, как её
//!   нарезает QGIS при чтении, через 0,02° на хорду.
//! - R3 - ломаная 4 x 400 м из трёх объектов с перепутанными и
//!   перевёрнутой частями, к ней ведомость с обратной вставкой 25 м
//!   и прямой вставкой 40 м.
//! - R4 - две части с разрывом 15 м, он должен попасть в ошибки сборки.
//!
//! Записи строятся тем же генератором случайных чисел, что у Python
//! ([`super::pyrand`]), и при том же зерне совпадают с записями модуля.

use super::pyrand::PyRandom;
use super::route::P3;
use crate::pyfmt;

pub const X0: f64 = 455000.0;
pub const Y0: f64 = 6428000.0;
pub const CRS: &str = "EPSG:32640";
pub const SYSTEM: &str = "исполнительная";
/// Рельеф примера - наклонная плоскость: xmin, ymin, xmax, ymax.
pub const DEM_EXTENT: [f64; 4] = [X0 - 200.0, Y0 - 300.0, X0 + 3000.0, Y0 + 2100.0];
pub const DEM_CELL: f64 = 5.0;
/// Угол хорды при нарезке дуги, градусы (`CURVE_ANGLE_DEG` модуля).
pub const CURVE_ANGLE_DEG: f64 = 0.02;

/// Отметка рельефа: вдоль R1 земля на 1,5 м выше оси, ось R1 - труба.
pub fn dem_z(x: f64, y: f64) -> f64 {
    151.5 + 0.01 * (x - X0) + 0.02 * (y - Y0)
}

/// Отметка оси R1 по Z вершин: 150 в начале, 170 в конце.
pub fn r1_z(m: f64) -> f64 {
    150.0 + m / 100.0
}

#[derive(Debug, Clone, Copy)]
enum Item {
    Line(f64),
    /// Радиус и угол в градусах, плюс - влево.
    Arc(f64, f64),
}

impl Item {
    fn length(self) -> f64 {
        match self {
            Item::Line(l) => l,
            Item::Arc(r, a) => r * a.abs().to_radians(),
        }
    }
}

/// Ось из прямых и дуг от начальной точки с азимутом.
#[derive(Debug, Clone)]
struct Path {
    x: f64,
    y: f64,
    azimuth: f64,
    items: Vec<Item>,
}

/// Python `a % b` для положительного `b`.
fn py_mod(a: f64, b: f64) -> f64 {
    let r = a % b;
    if r != 0.0 && r < 0.0 { r + b } else { r }
}

impl Path {
    fn length(&self) -> f64 {
        self.items.iter().fold(0.0, |s, i| s + i.length())
    }

    /// (x, y, азимут) на мере m.
    fn at(&self, mut m: f64) -> (f64, f64, f64) {
        let (mut x, mut y, mut az) = (self.x, self.y, self.azimuth);
        for it in &self.items {
            match *it {
                Item::Line(len) => {
                    let d = m.min(len);
                    x += d * az.to_radians().sin();
                    y += d * az.to_radians().cos();
                    if m <= len {
                        return (x, y, az);
                    }
                    m -= len;
                }
                Item::Arc(r, angle) => {
                    let l = r * angle.abs().to_radians();
                    let d = m.min(l);
                    let s = if angle > 0.0 { 1.0 } else { -1.0 };
                    // центр слева (s=1) или справа от направления
                    let cx = x + r * (az - 90.0 * s).to_radians().sin();
                    let cy = y + r * (az - 90.0 * s).to_radians().cos();
                    let turn = (d / r).to_degrees() * s;
                    let a0 = (x - cx).atan2(y - cy);
                    let a1 = a0 - turn.to_radians();
                    x = cx + r * a1.sin();
                    y = cy + r * a1.cos();
                    az = py_mod(az - turn, 360.0);
                    if m <= l {
                        return (x, y, az);
                    }
                    m -= l;
                }
            }
        }
        (x, y, az)
    }

    fn point(&self, m: f64, offset: f64) -> (f64, f64) {
        let (x, y, az) = self.at(m);
        (
            x - offset * az.to_radians().cos(),
            y + offset * az.to_radians().sin(),
        )
    }

    fn vertices_m(&self) -> Vec<f64> {
        let mut out = vec![0.0];
        let mut m = 0.0;
        for it in &self.items {
            m += it.length();
            out.push(m);
        }
        out
    }
}

/// Аналитическая ломаная: точные координаты вершин.
#[derive(Debug, Clone)]
struct Polyline {
    v: Vec<[f64; 2]>,
    seg: Vec<f64>,
}

impl Polyline {
    fn new(v: Vec<[f64; 2]>) -> Self {
        let seg = v
            .windows(2)
            .map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]))
            .collect();
        Self { v, seg }
    }

    fn vertices_m(&self) -> Vec<f64> {
        let mut out = vec![0.0];
        for s in &self.seg {
            let last = out.last().copied().unwrap_or(0.0);
            out.push(last + s);
        }
        out
    }

    fn point(&self, mut m: f64, offset: f64) -> (f64, f64) {
        for (w, &s) in self.v.windows(2).zip(&self.seg) {
            let (a, b) = (w[0], w[1]);
            if m <= s + 1e-9 {
                let (dx, dy) = ((b[0] - a[0]) / s, (b[1] - a[1]) / s);
                return (a[0] + dx * m - dy * offset, a[1] + dy * m + dx * offset);
            }
            m -= s;
        }
        let n = self.v.len();
        let (a, b, s) = (self.v[n - 2], self.v[n - 1], self.seg[n - 2]);
        let (dx, dy) = ((b[0] - a[0]) / s, (b[1] - a[1]) / s);
        (b[0] - dy * offset, b[1] + dx * offset)
    }
}

/// Ось маршрута примера.
#[derive(Debug, Clone)]
enum Shape {
    Path(Path),
    Poly(Polyline),
}

impl Shape {
    /// Точка на мере m со смещением, плюс влево.
    fn point(&self, m: f64, offset: f64) -> (f64, f64) {
        match self {
            Shape::Path(p) => p.point(m, offset),
            Shape::Poly(p) => p.point(m, offset),
        }
    }

    fn vertices_m(&self) -> Vec<f64> {
        match self {
            Shape::Path(p) => p.vertices_m(),
            Shape::Poly(p) => p.vertices_m(),
        }
    }
}

// ------------------------------------------------------------ пикетаж R3
/// Участки: (m_from, m_to, st_from, st_to) - то же, что должна построить
/// ведомость.
pub const R3_SECTIONS: [(f64, f64, f64, f64); 4] = [
    (0.0, 400.0, 0.0, 399.6),
    (400.0, 700.0, 399.6, 699.3),
    // обратная вставка 6+99.30 = 6+74.30
    (700.0, 1300.0, 674.3, 1274.3),
    // прямая вставка 12+74.30 = 13+14.30
    (1300.0, 1600.0, 1314.3, 1614.3),
];

pub fn r3_station(m: f64) -> (f64, usize) {
    let last = R3_SECTIONS.len() - 1;
    for (i, &(m0, m1, s0, s1)) in R3_SECTIONS.iter().enumerate() {
        if (m0 <= m && m < m1) || (i == last && m <= m1) {
            return (s0 + (m - m0) * (s1 - s0) / (m1 - m0), i);
        }
    }
    (f64::NAN, last)
}

pub fn r3_measures(st: f64) -> Vec<(f64, usize)> {
    R3_SECTIONS
        .iter()
        .enumerate()
        .filter(|(_, sec)| sec.2 <= st && st <= sec.3)
        .map(|(i, &(m0, m1, s0, s1))| (m0 + (st - s0) * (m1 - m0) / (s1 - s0), i))
        .collect()
}

/// Пикет записью одного из трёх видов: `ПК 7+29.89`, `7+29,89`, `пк7+29.89`.
pub fn pk(st: f64, style: usize) -> String {
    let n = (st / 100.0 + 1e-9).floor();
    let plus = st - n * 100.0;
    let n = n as i64;
    match style {
        1 => format!("{n}+{}", pyfmt::fixed_width(plus, 5, 2)).replace('.', ","),
        2 => format!("пк{n}+{}", pyfmt::fixed(plus, 2)),
        _ => format!("ПК {n}+{}", pyfmt::fixed_width(plus, 5, 2)),
    }
}

// ------------------------------------------------------------ нарезка дуги
/// Центр и радиус окружности по трём точкам (`QgsGeometryUtils::
/// circleCenterRadius`). Точки на одной прямой - радиус -1.
fn circle(p1: [f64; 2], p2: [f64; 2], p3: [f64; 2]) -> (f64, f64, f64) {
    let (dx21, dy21) = (p2[0] - p1[0], p2[1] - p1[1]);
    let (dx31, dy31) = (p3[0] - p1[0], p3[1] - p1[1]);
    let h21 = dx21.powi(2) + dy21.powi(2);
    let h31 = dx31.powi(2) + dy31.powi(2);
    let d = 2.0 * (dx21 * dy31 - dx31 * dy21);
    if d.abs() <= 1e-11 {
        return (0.0, 0.0, -1.0);
    }
    let cx = p1[0] + (h21 * dy31 - h31 * dy21) / d;
    let cy = p1[1] - (h21 * dx31 - h31 * dx21) / d;
    let r = ((cx - p1[0]).powi(2) + (cy - p1[1]).powi(2)).sqrt();
    (cx, cy, r)
}

/// Дуга по трём точкам - вершинами, как `QgsGeometryUtils::segmentizeArc`
/// с наибольшим углом хорды `step` (радианы), нарезка симметричная.
pub fn segmentize_arc(p1: [f64; 2], p2: [f64; 2], p3: [f64; 2], step: f64) -> Vec<[f64; 2]> {
    let side = (p2[0] - p1[0]) * (p3[1] - p1[1]) - (p3[0] - p1[0]) * (p2[1] - p1[1]);
    let reversed = side < 0.0;
    let (c1, c3) = if reversed { (p3, p1) } else { (p1, p3) };
    let (cx, cy, r) = circle(c1, p2, c3);
    if c1 != c3 && (r < 0.0 || side == 0.0) {
        return vec![p1, p2, p3];
    }
    let a1 = (c1[1] - cy).atan2(c1[0] - cx);
    let mut a3 = (c3[1] - cy).atan2(c3[0] - cx);
    let mut angle = a3 - a1;
    if angle <= 0.0 {
        angle += std::f64::consts::PI * 2.0;
    }
    let segs = (angle / step).ceil();
    let inc = angle / segs;
    if a3 <= a1 {
        a3 += 2.0 * std::f64::consts::PI;
    }
    let mut pts = vec![c1];
    if p2 != c3 && c1 != p2 {
        let stop = a3 - inc / 100.0;
        let mut a = a1 + inc;
        while a < stop {
            pts.push([cx + r * a.cos(), cy + r * a.sin()]);
            a += inc;
        }
    }
    pts.push(c3);
    if reversed {
        pts.reverse();
    }
    pts
}

/// Вершины, соединённые части без повтора общей точки.
fn join(points: &mut Vec<[f64; 2]>, more: Vec<[f64; 2]>) {
    let skip = usize::from(points.last().is_some() && points.last() == more.first());
    points.extend(more.into_iter().skip(skip));
}

/// Координата как в WKT примера: `f"{x:.4f}"`.
fn c4(v: f64) -> f64 {
    pyfmt::fixed(v, 4).parse().unwrap_or(v)
}

fn xy4(p: (f64, f64)) -> [f64; 2] {
    [c4(p.0), c4(p.1)]
}

// ------------------------------------------------------------ сборка примера
/// Объект слоя маршрутов: части в метрах с Z (у R1 отметки, у остальных 0).
#[derive(Debug, Clone)]
pub struct DemoRoute {
    pub route_id: &'static str,
    /// Русский текст, перевод - при выдаче.
    pub name: &'static str,
    pub parts: Vec<Vec<P3>>,
}

#[derive(Debug, Clone)]
pub struct LedgerRow {
    pub route_id: &'static str,
    pub system: &'static str,
    pub station: &'static str,
    pub measure: f64,
    pub station_ahead: Option<&'static str>,
    pub note: &'static str,
}

#[derive(Debug, Clone)]
pub struct PointEvent {
    pub eid: i64,
    pub route_id: &'static str,
    pub pk: String,
    pub offset: f64,
    pub section: Option<usize>,
    pub exp_m: Option<f64>,
    pub exp_x: Option<f64>,
    pub exp_y: Option<f64>,
    pub exp_error: &'static str,
    pub note: &'static str,
}

#[derive(Debug, Clone)]
pub struct LineEvent {
    pub eid: i64,
    pub route_id: &'static str,
    pub pk_from: &'static str,
    pub pk_to: &'static str,
    pub offset: f64,
    pub exp_m_from: Option<f64>,
    pub exp_m_to: Option<f64>,
    pub exp_error: &'static str,
    pub note: &'static str,
}

#[derive(Debug, Clone)]
pub struct Defect {
    pub did: i64,
    pub x: f64,
    pub y: f64,
    pub exp_route: &'static str,
    pub exp_m: f64,
    pub exp_station: f64,
    pub exp_offset: f64,
}

#[derive(Debug, Clone)]
pub struct Demo {
    pub routes: Vec<DemoRoute>,
    pub ledger: Vec<LedgerRow>,
    pub points: Vec<PointEvent>,
    pub lines: Vec<LineEvent>,
    pub defects: Vec<Defect>,
}

const IDS: [&str; 3] = ["R1", "R2", "R3"];

/// Пример: `n_points` случайных точечных событий, `n_defects` дефектов.
pub fn build(n_points: usize, n_defects: usize, seed: i64) -> Demo {
    let mut rnd = PyRandom::new(seed);
    let r1 = Path {
        x: X0,
        y: Y0,
        azimuth: 90.0,
        items: vec![Item::Line(2000.0)],
    };
    let r2 = Path {
        x: X0,
        y: Y0 + 500.0,
        azimuth: 90.0,
        items: vec![Item::Line(500.0), Item::Arc(300.0, 90.0), Item::Line(500.0)],
    };
    let mut routes = Vec::new();

    // R1 с Z: отметка растёт от 150 до 170
    let (a, mid, b) = (
        xy4(r1.point(0.0, 0.0)),
        xy4(r1.point(1000.0, 0.0)),
        xy4(r1.point(2000.0, 0.0)),
    );
    routes.push(DemoRoute {
        route_id: "R1",
        name: "Прямая с отметками",
        parts: vec![vec![
            [a[0], a[1], 150.0],
            [mid[0], mid[1], 160.0],
            [b[0], b[1], 170.0],
        ]],
    });

    // R2 - дуга, нарезанная как при чтении в QGIS
    let pi = std::f64::consts::PI;
    let s = xy4(r2.point(0.0, 0.0));
    let e1 = xy4(r2.point(500.0, 0.0));
    let e2 = xy4(r2.point(500.0 + 300.0 * pi / 2.0, 0.0));
    let end = xy4(r2.point(r2.length(), 0.0));
    let amid = xy4(r2.point(500.0 + 300.0 * pi / 4.0, 0.0));
    let mut line = vec![s, e1];
    join(
        &mut line,
        segmentize_arc(e1, amid, e2, CURVE_ANGLE_DEG.to_radians()),
    );
    join(&mut line, vec![e2, end]);
    routes.push(DemoRoute {
        route_id: "R2",
        name: "Кривая R=300",
        parts: vec![line.iter().map(|p| [p[0], p[1], 0.0]).collect()],
    });

    // R3 - ломаная, три объекта с частями в произвольном порядке
    let mut r3v = vec![[X0, Y0 + 1500.0]];
    for az in [60.0f64, 120.0, 60.0, 120.0] {
        let [x, y] = r3v[r3v.len() - 1];
        r3v.push([
            x + 400.0 * az.to_radians().sin(),
            y + 400.0 * az.to_radians().cos(),
        ]);
    }
    let v: Vec<P3> = r3v.iter().map(|p| [c4(p[0]), c4(p[1]), 0.0]).collect();
    let name3 = "Части в произвольном порядке";
    routes.push(DemoRoute {
        route_id: "R3",
        name: name3,
        parts: vec![vec![v[3], v[2]], vec![v[1], v[2]]],
    });
    routes.push(DemoRoute {
        route_id: "R3",
        name: name3,
        parts: vec![vec![v[3], v[4]]],
    });
    routes.push(DemoRoute {
        route_id: "R3",
        name: name3,
        parts: vec![vec![v[0], v[1]]],
    });

    // R4 - разрыв 15 м
    for (y0, y1) in [(Y0, Y0 + 300.0), (Y0 + 315.0, Y0 + 615.0)] {
        routes.push(DemoRoute {
            route_id: "R4",
            name: "С разрывом",
            parts: vec![vec![[X0 + 2500.0, y0, 0.0], [X0 + 2500.0, y1, 0.0]]],
        });
    }

    let lr = |station, measure, ahead, note| LedgerRow {
        route_id: "R3",
        system: SYSTEM,
        station,
        measure,
        station_ahead: ahead,
        note,
    };
    let ledger = vec![
        lr("ПК 0+00", 0.0, None, "начало"),
        lr("ПК 3+99.60", 400.0, None, "репер, невязка -0.40"),
        lr(
            "ПК 6+99.30",
            700.0,
            Some("ПК 6+74.30"),
            "обратная вставка 25 м",
        ),
        lr("ПК 9+74.30", 1000.0, None, "репер"),
        lr(
            "ПК 12+74.30",
            1300.0,
            Some("ПК 13+14.30"),
            "прямая вставка 40 м",
        ),
        lr("ПК 16+14.30", 1600.0, None, "конец"),
    ];

    let shapes = [
        Shape::Path(r1),
        Shape::Path(r2.clone()),
        Shape::Poly(Polyline::new(r3v)),
    ];
    let shape = |rid: &str| match rid {
        "R1" => Some(&shapes[0]),
        "R2" => Some(&shapes[1]),
        "R3" => Some(&shapes[2]),
        _ => None,
    };
    let lengths = [2000.0, r2.length(), 1600.0];

    let mut points: Vec<PointEvent> = Vec::new();
    let mut add_point = |rid: &'static str,
                         text: String,
                         m: Option<f64>,
                         offset: f64,
                         section: Option<usize>,
                         err: &'static str,
                         note: &'static str| {
        let (mut ex, mut ey) = (None, None);
        if let (Some(m), "", Some(sh)) = (m, err, shape(rid)) {
            let (x, y) = sh.point(m, offset);
            ex = Some(x);
            ey = Some(y);
        }
        points.push(PointEvent {
            eid: points.len() as i64 + 1,
            route_id: rid,
            pk: text,
            offset,
            section,
            exp_m: if err.is_empty() { m } else { None },
            exp_x: ex,
            exp_y: ey,
            exp_error: err,
            note,
        });
    };

    for i in 0..n_points {
        let k = i % 3;
        let rid = IDS[k];
        let mut m = rnd.uniform(0.0, lengths[k]);
        let u = rnd.uniform(-15.0, 15.0);
        let off = rnd.choice(&[0.0, 0.0, u]);
        // пикет в таблице записан до 0.01 м - эталонная мера от него
        if rid == "R3" {
            let (st, sec) = r3_station(m);
            let st = pyfmt::round(st, 2);
            let cands = r3_measures(st);
            m = cands
                .iter()
                .find(|(_, si)| *si == sec)
                .or(cands.first())
                .map_or(m, |c| c.0);
            let hint = (cands.len() > 1).then_some(sec);
            let note = if hint.is_some() {
                "в обратной вставке, указан участок"
            } else {
                ""
            };
            add_point(rid, pk(st, k), Some(m), off, hint, "", note);
        } else {
            m = pyfmt::round(m, 2);
            add_point(rid, pk(m, k), Some(m), off, None, "", "");
        }
    }

    add_point(
        "R1",
        "ПК 7+10".into(),
        Some(710.0),
        0.0,
        None,
        "",
        "опорный для относительной записи",
    );
    add_point(
        "R1",
        "+55".into(),
        Some(755.0),
        0.0,
        None,
        "",
        "относительная запись: тот же пикет",
    );
    add_point(
        "R3",
        "ПК 6+80".into(),
        None,
        0.0,
        None,
        "station_ambiguous",
        "обратная вставка без участка",
    );
    add_point(
        "R3",
        "ПК 12+90".into(),
        None,
        0.0,
        None,
        "station_in_gap",
        "попал в прямую вставку",
    );
    add_point(
        "R1",
        "ПК 25+00".into(),
        None,
        0.0,
        None,
        "station_out_of_range",
        "за концом маршрута",
    );
    add_point(
        "R1",
        "ПК 5+".into(),
        None,
        0.0,
        None,
        "station_parse_failed",
        "испорченная запись",
    );
    add_point(
        "R9",
        "ПК 1+00".into(),
        None,
        0.0,
        None,
        "route_not_found",
        "нет такого маршрута",
    );
    add_point(
        "R4",
        "ПК 1+00".into(),
        None,
        0.0,
        None,
        "route_not_found",
        "маршрут не собран: разрыв",
    );

    let mut lines: Vec<LineEvent> = Vec::new();
    let mut add_line = |rid: &'static str,
                        a: &'static str,
                        b: &'static str,
                        m: Option<(f64, f64)>,
                        offset: f64,
                        err: &'static str,
                        note: &'static str| {
        lines.push(LineEvent {
            eid: lines.len() as i64 + 1,
            route_id: rid,
            pk_from: a,
            pk_to: b,
            offset,
            exp_m_from: m.map(|v| v.0),
            exp_m_to: m.map(|v| v.1),
            exp_error: err,
            note,
        });
    };
    let r3m = |st: f64| r3_measures(st).first().map_or(f64::NAN, |c| c.0);
    add_line(
        "R1",
        "ПК 2+00",
        "ПК 7+50",
        Some((200.0, 750.0)),
        0.0,
        "",
        "простой участок",
    );
    add_line(
        "R1",
        "ПК 9+00",
        "ПК 3+00",
        Some((300.0, 900.0)),
        0.0,
        "",
        "записан против хода, меняется местами",
    );
    add_line(
        "R2",
        "ПК 4+00",
        "ПК 11+00",
        Some((400.0, 1100.0)),
        10.0,
        "",
        "через дугу, смещение 10 м влево",
    );
    add_line(
        "R3",
        "ПК 6+00",
        "ПК 8+00",
        Some((r3m(600.0), r3m(800.0))),
        0.0,
        "",
        "через обратную вставку",
    );
    add_line(
        "R3",
        "ПК 2+00",
        "ПК 14+00",
        Some((r3m(200.0), r3m(1400.0))),
        0.0,
        "",
        "через обе вставки и изломы",
    );
    add_line(
        "R1",
        "ПК 3+00",
        "ПК 3+00",
        None,
        0.0,
        "from_ge_to",
        "нулевая длина",
    );

    let mut defects = Vec::new();
    for i in 0..n_defects {
        let k = i % 3;
        let sh = &shapes[k];
        let verts = sh.vertices_m();
        let inner = &verts[1..verts.len() - 1];
        let m = loop {
            let m = rnd.uniform(20.0, lengths[k] - 20.0);
            if inner.iter().all(|v| (m - v).abs() > 40.0) {
                break m;
            }
        };
        let sign = rnd.choice(&[-1.0, 1.0]);
        let off = sign * rnd.uniform(1.0, 15.0);
        let (x, y) = sh.point(m, off);
        let st = if k == 2 { r3_station(m).0 } else { m };
        defects.push(Defect {
            did: i as i64 + 1,
            x,
            y,
            exp_route: IDS[k],
            exp_m: m,
            exp_station: st,
            exp_offset: off,
        });
    }

    Demo {
        routes,
        ledger,
        points,
        lines,
        defects,
    }
}

// ------------------------------------------------------------ рельеф
/// Растр рельефа примера: плоскость [`dem_z`] по центрам ячеек, строки
/// сверху вниз, как GeoTIFF модуля.
pub struct Dem {
    pub x0: f64,
    pub y1: f64,
    pub cell: f64,
    pub cols: usize,
    pub rows: usize,
    pub grid: Vec<f64>,
}

impl Dem {
    pub fn new() -> Self {
        let [x0, y0, x1, y1] = DEM_EXTENT;
        let c = DEM_CELL;
        let cols = ((x1 - x0) / c).round() as usize;
        let rows = ((y1 - y0) / c).round() as usize;
        let mut grid = Vec::with_capacity(cols * rows);
        for r in 0..rows {
            let yc = y1 - (r as f64 + 0.5) * c;
            for col in 0..cols {
                let xc = x0 + (col as f64 + 0.5) * c;
                grid.push(dem_z(xc, yc));
            }
        }
        Self {
            x0,
            y1,
            cell: c,
            cols,
            rows,
            grid,
        }
    }

    /// Значение в точке: билинейно по центрам ячеек
    /// (`core/profile.bilinear` модуля), снаружи растра - NaN.
    pub fn sample(&self, x: f64, y: f64) -> f64 {
        let (cols, rows) = (self.cols as f64, self.rows as f64);
        let fc = (x - self.x0) / self.cell - 0.5;
        let fr = (y - self.y1) / -self.cell - 0.5;
        if !(fc >= -0.5 && fc <= cols - 0.5 && fr >= -0.5 && fr <= rows - 0.5) {
            return f64::NAN;
        }
        let c0 = (fc.floor() as i64).clamp(0, (self.cols as i64 - 2).max(0)) as usize;
        let r0 = (fr.floor() as i64).clamp(0, (self.rows as i64 - 2).max(0)) as usize;
        let (c1, r1) = ((c0 + 1).min(self.cols - 1), (r0 + 1).min(self.rows - 1));
        let (tc, tr) = (fc - c0 as f64, fr - r0 as f64);
        let g = |r: usize, c: usize| self.grid[r * self.cols + c];
        let vals = [g(r0, c0), g(r0, c1), g(r1, c0), g(r1, c1)];
        let w = [
            (1.0 - tc) * (1.0 - tr),
            tc * (1.0 - tr),
            (1.0 - tc) * tr,
            tc * tr,
        ];
        let ws: f64 = w.iter().sum();
        if ws > 1e-12 {
            vals.iter().zip(w).map(|(v, w)| v * w).sum::<f64>() / ws
        } else {
            f64::NAN
        }
    }
}

impl Default for Dem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stations_and_pk() {
        assert_eq!(pk(729.89, 0), "ПК 7+29.89");
        assert_eq!(pk(375.27, 1), "3+75,27");
        assert_eq!(pk(729.89, 2), "пк7+29.89");
        assert_eq!(pk(5.0, 0), "ПК 0+05.00");
        assert_eq!(r3_measures(680.0).len(), 2);
        assert!(
            (Dem::new().sample(X0 + 1000.0, Y0 + 3.0) - dem_z(X0 + 1000.0, Y0 + 3.0)).abs() < 1e-9
        );
    }
}
