// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 3.0 или новее.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Продольный профиль (`routeliner/core/profile.py`): точки профиля, уклоны
//! и раскладка чертежа. Снятия значений с растра здесь нет - у расширений
//! RustGIS растров нет.
//!
//! Чертёж строится в миллиметрах бумаги. По горизонтали
//! X = (m - m0) * 1000 / H, где H - знаменатель горизонтального масштаба,
//! по вертикали над сеткой Y = (z - горизонт) * 1000 / V. Строки сетки идут
//! вниз от Y = 0.

use super::chainage::ChainageSystem;
use super::route::RouteGeometry;
use crate::pyfmt::{self, Msg};

/// Точки ближе этого по мере сливаются, м.
pub const MERGE_TOL: f64 = 0.01;

/// Приоритет вида точки: выше - важнее при слиянии и прореживании подписей.
pub fn priority(kind: &str) -> i32 {
    match kind {
        "start" | "end" => 6,
        "equation" => 5,
        "event" => 4,
        "picket" => 3,
        "vertex" => 2,
        "step" => 1,
        _ => 0,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProfilePoint {
    pub m: f64,
    pub kind: String,
    pub label: String,
    /// Угол поворота трассы в вершине, градусы, плюс влево.
    pub turn: f64,
    pub station: f64,
    /// Только у уравнения.
    pub station_ahead: f64,
    pub section: usize,
}

impl ProfilePoint {
    pub fn new(m: f64, kind: &str) -> Self {
        Self {
            m,
            kind: kind.to_string(),
            label: String::new(),
            turn: 0.0,
            station: f64::NAN,
            station_ahead: f64::NAN,
            section: 0,
        }
    }

    pub fn priority(&self) -> i32 {
        priority(&self.kind)
    }
}

/// Какие вершины оси идут в профиль.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vertices {
    /// Только с поворотом в плане или изломом уклона.
    Significant,
    All,
    None,
}

/// Точки профиля одного маршрута: начало и конец, пикетажные уравнения,
/// события, целые пикеты, вершины оси и точки с постоянным шагом.
pub fn collect_points(
    route: &RouteGeometry,
    system: &ChainageSystem,
    picket_step: f64,
    step: f64,
    vertices: Vertices,
    events: &[(f64, String)],
) -> Vec<ProfilePoint> {
    let (turn_tol, grade_tol) = (1.0, 0.001);
    let l = route.length;
    let mut pts = vec![ProfilePoint::new(0.0, "start"), ProfilePoint::new(l, "end")];
    for (m, back, ahead) in system.equations() {
        let mut p = ProfilePoint::new(m, "equation");
        p.station = back;
        p.station_ahead = ahead;
        pts.push(p);
    }
    for (m, label) in events {
        if -MERGE_TOL <= *m && *m <= l + MERGE_TOL {
            let mut p = ProfilePoint::new(m.clamp(0.0, l), "event");
            p.label = label.clone();
            pts.push(p);
        }
    }
    if picket_step > 0.0 {
        for (m, _, _) in system.whole_stations(picket_step) {
            if 0.0 <= m && m <= l {
                pts.push(ProfilePoint::new(m, "picket"));
            }
        }
    }
    if vertices != Vertices::None {
        for v in route.vertices() {
            if vertices == Vertices::All || v.turn.abs() > turn_tol || v.dgrade.abs() > grade_tol {
                let mut p = ProfilePoint::new(v.m, "vertex");
                p.turn = v.turn;
                pts.push(p);
            }
        }
    }
    if step > 0.0 {
        let n = pyfmt::floor_div(l, step) as i64;
        for k in 1..=n {
            pts.push(ProfilePoint::new(k as f64 * step, "step"));
        }
    }
    pts.sort_by(|a, b| {
        a.m.total_cmp(&b.m)
            .then_with(|| b.priority().cmp(&a.priority()))
    });
    let mut merged: Vec<ProfilePoint> = Vec::new();
    for mut p in pts {
        if let Some(q) = merged.last_mut()
            && p.m - q.m <= MERGE_TOL
        {
            if p.priority() > q.priority() {
                if p.label.is_empty() {
                    p.label = q.label.clone();
                }
                if p.turn == 0.0 {
                    p.turn = q.turn;
                }
                *q = p;
            } else {
                if q.label.is_empty() {
                    q.label = p.label;
                }
                if q.turn == 0.0 {
                    q.turn = p.turn;
                }
            }
            continue;
        }
        merged.push(p);
    }
    let starts: Vec<f64> = system.sections.iter().map(|s| s.m_from).collect();
    for p in &mut merged {
        let i = starts
            .partition_point(|v| *v <= p.m + 1e-9)
            .saturating_sub(1);
        p.section = i;
        if p.kind != "equation" {
            p.station = system.sections[i].station_at(p.m);
        }
    }
    merged
}

/// Участок постоянного уклона.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradeSegment {
    pub m0: f64,
    pub m1: f64,
    pub z0: f64,
    pub z1: f64,
}

impl GradeSegment {
    pub fn length(&self) -> f64 {
        self.m1 - self.m0
    }

    /// Уклон в промилле, плюс - подъём по ходу маршрута.
    pub fn grade(&self) -> f64 {
        if self.length() > 0.0 {
            (self.z1 - self.z0) / self.length() * 1000.0
        } else {
            0.0
        }
    }
}

/// Участки постоянного уклона упрощением Дугласа - Пекера по вертикали
/// с допуском `tolerance`, м. Точки без значения разрывают линию.
pub fn grade_segments(ms: &[f64], zs: &[f64], tolerance: f64) -> Vec<GradeSegment> {
    let mut out = Vec::new();
    let n = ms.len();
    let mut i = 0;
    while i < n {
        if zs[i].is_nan() {
            i += 1;
            continue;
        }
        let mut j = i;
        while j + 1 < n && !zs[j + 1].is_nan() {
            j += 1;
        }
        if j > i {
            let keep = douglas_peucker(&ms[i..=j], &zs[i..=j], tolerance);
            let idx: Vec<usize> = keep.iter().map(|k| i + k).collect();
            for w in idx.windows(2) {
                let (a, b) = (w[0], w[1]);
                if ms[b] - ms[a] > 1e-9 {
                    out.push(GradeSegment {
                        m0: ms[a],
                        m1: ms[b],
                        z0: zs[a],
                        z1: zs[b],
                    });
                }
            }
        }
        i = j + 1;
    }
    out
}

fn douglas_peucker(x: &[f64], y: &[f64], tol: f64) -> Vec<usize> {
    let mut keep = vec![0, x.len() - 1];
    let mut stack = vec![(0, x.len() - 1)];
    while let Some((a, b)) = stack.pop() {
        if b - a < 2 {
            continue;
        }
        let (xa, ya, xb, yb) = (x[a], y[a], x[b], y[b]);
        let mut best = (0, f64::NEG_INFINITY);
        for (k, i) in (a + 1..b).enumerate() {
            let yl = if xb != xa {
                ya + (yb - ya) * (x[i] - xa) / (xb - xa)
            } else {
                ya
            };
            let d = (y[i] - yl).abs();
            if d > best.1 {
                best = (k, d);
            }
        }
        if best.1 > tol {
            let c = a + 1 + best.0;
            keep.push(c);
            stack.push((a, c));
            stack.push((c, b));
        }
    }
    keep.sort_unstable();
    keep.dedup();
    keep
}

/// Точки, подписи которых не налезают друг на друга: сначала с большим
/// приоритетом, затем остальные, если до выбранных не меньше `min_gap` мм.
pub fn thin(xs: &[f64], priorities: &[i32], min_gap: f64) -> Vec<bool> {
    let mut order: Vec<usize> = (0..xs.len()).collect();
    order.sort_by(|&a, &b| {
        priorities[b]
            .cmp(&priorities[a])
            .then_with(|| xs[a].total_cmp(&xs[b]))
    });
    let mut taken: Vec<f64> = Vec::new();
    let mut mask = vec![false; xs.len()];
    for i in order {
        let k = taken.partition_point(|v| *v < xs[i]);
        if (k > 0 && xs[i] - taken[k - 1] < min_gap - 1e-9)
            || (k < taken.len() && taken[k] - xs[i] < min_gap - 1e-9)
        {
            continue;
        }
        taken.insert(k, xs[i]);
        mask[i] = true;
    }
    mask
}

/// Оценка ширины подписи в мм.
pub fn text_width(text: &str, size: f64) -> f64 {
    let n = text
        .split('\n')
        .map(|t| t.chars().count())
        .max()
        .unwrap_or(0);
    0.62 * size * n as f64 + 0.5
}

pub const ROW_TYPES: [&str; 6] = ["value", "text", "grade", "distance", "station", "plan"];
pub const PALETTE: [&str; 7] = [
    "#8c6d1f", "#2e7d32", "#000000", "#1f4fbf", "#b23b3b", "#6a3d9a", "#e08000",
];

/// Строка сетки профиля.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub title: String,
    pub kind: String,
    /// мм, 0 - строка не выводится в сетку.
    pub height: f64,
    pub decimals: usize,
    /// Рисовать линию над сеткой (для value).
    pub draw: bool,
    /// value, grade - по всем точкам.
    pub values: Option<Vec<f64>>,
    /// text - (m0, m1, текст).
    pub segments: Vec<(f64, f64, String)>,
    /// grade - допуск упрощения, м.
    pub tolerance: f64,
    pub color: String,
}

impl Row {
    pub fn new(title: &str, kind: &str, height: f64, decimals: usize, draw: bool) -> Self {
        Self {
            title: title.to_string(),
            kind: kind.to_string(),
            height,
            decimals,
            draw,
            values: None,
            segments: Vec::new(),
            tolerance: 0.02,
            color: String::new(),
        }
    }
}

/// Линия чертежа: вершины, вид, строка, цвет, толщина.
pub type Line = (Vec<(f64, f64)>, String, String, String, f64);

/// Подпись чертежа.
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    pub x: f64,
    pub y: f64,
    pub text: String,
    pub rot: f64,
    pub size: f64,
    pub halign: &'static str,
    pub valign: &'static str,
    pub kind: &'static str,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sheet {
    pub lines: Vec<Line>,
    pub texts: Vec<Text>,
    /// x, y, вид, подпись.
    pub marks: Vec<(f64, f64, &'static str, String)>,
    pub horizon: f64,
    pub width: f64,
}

const HEADER: f64 = 60.0;
const GAP: f64 = 5.0;
const TEXT: f64 = 2.5;
const TITLE: f64 = 3.0;
const GRADE_FULL: f64 = 4.0;

/// Раскладка чертежа профиля. `tr` переводит слова чертежа на язык показа.
pub struct ProfileDrawer<'a> {
    pub p: Vec<ProfilePoint>,
    ms: Vec<f64>,
    m0: f64,
    labels: Vec<String>,
    h: f64,
    v: f64,
    horizon: Option<f64>,
    plan_points: Vec<(f64, f64, String)>,
    xs: Vec<f64>,
    take: Vec<bool>,
    tr: &'a dyn Fn(&Msg) -> String,
    sheet: Sheet,
}

impl<'a> ProfileDrawer<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        points: Vec<ProfilePoint>,
        station_labels: Vec<String>,
        h_scale: f64,
        v_scale: f64,
        horizon: Option<f64>,
        min_gap: f64,
        plan_points: Vec<(f64, f64, String)>,
        tr: &'a dyn Fn(&Msg) -> String,
    ) -> Self {
        let ms: Vec<f64> = points.iter().map(|p| p.m).collect();
        let m0 = ms.first().copied().unwrap_or(0.0);
        let xs: Vec<f64> = ms.iter().map(|m| (m - m0) * 1000.0 / h_scale).collect();
        let prio: Vec<i32> = points.iter().map(ProfilePoint::priority).collect();
        let take = thin(&xs, &prio, min_gap);
        Self {
            p: points,
            ms,
            m0,
            labels: station_labels,
            h: h_scale,
            v: v_scale,
            horizon,
            plan_points,
            xs,
            take,
            tr,
            sheet: Sheet::default(),
        }
    }

    fn x(&self, m: f64) -> f64 {
        (m - self.m0) * 1000.0 / self.h
    }

    fn y(&self, z: f64) -> f64 {
        (z - self.sheet.horizon) * 1000.0 / self.v
    }

    fn taken(&self) -> Vec<usize> {
        (0..self.take.len()).filter(|&i| self.take[i]).collect()
    }

    pub fn build(mut self, rows: &[Row]) -> Sheet {
        self.sheet.width = self.xs.last().copied().unwrap_or(0.0);
        let drawn: Vec<&Row> = rows
            .iter()
            .filter(|r| {
                r.kind == "value"
                    && r.draw
                    && r.values
                        .as_ref()
                        .is_some_and(|v| v.iter().any(|z| z.is_finite()))
            })
            .collect();
        let nanmin = |v: &[f64]| {
            v.iter()
                .copied()
                .filter(|z| !z.is_nan())
                .fold(f64::INFINITY, f64::min)
        };
        let nanmax = |v: &[f64]| {
            v.iter()
                .copied()
                .filter(|z| !z.is_nan())
                .fold(f64::NEG_INFINITY, f64::max)
        };
        let vals = |r: &Row| r.values.clone().unwrap_or_default();
        let zmin = if drawn.is_empty() {
            0.0
        } else {
            drawn
                .iter()
                .map(|r| nanmin(&vals(r)))
                .fold(f64::INFINITY, f64::min)
        };
        let zmax = if drawn.is_empty() {
            zmin + 1.0
        } else {
            drawn
                .iter()
                .map(|r| nanmax(&vals(r)))
                .fold(f64::NEG_INFINITY, f64::max)
        };
        self.sheet.horizon = self.horizon.unwrap_or(zmin.floor() - 1.0);

        let w = self.sheet.width;
        let xl = -HEADER - GAP;
        self.line(vec![(xl, 0.0), (-GAP, 0.0)], "frame");
        self.line(vec![(0.0, 0.0), (w, 0.0)], "frame");
        let mut y = 0.0;
        for r in rows.iter().filter(|r| r.height > 0.0 && has_content(r)) {
            let (top, bottom) = (y, y - r.height);
            self.text(
                xl + 1.5,
                (top + bottom) / 2.0,
                &r.title,
                0.0,
                TITLE,
                "Left",
                "Half",
                "title",
            );
            match r.kind.as_str() {
                "value" => self.row_value(r, top, bottom),
                "station" => self.row_station(top, bottom),
                "distance" => self.row_distance(r, top, bottom),
                "text" => self.row_text(r, top, bottom),
                "grade" => self.row_grade(r, top, bottom),
                "plan" => self.row_plan(r, top, bottom),
                _ => {}
            }
            self.line(vec![(xl, bottom), (-GAP, bottom)], "frame");
            self.line(vec![(0.0, bottom), (w, bottom)], "frame");
            y = bottom;
        }
        for xx in [xl, -GAP, 0.0, w] {
            self.line(vec![(xx, 0.0), (xx, y)], "frame");
        }

        let mut top_y = vec![f64::NEG_INFINITY; self.ms.len()];
        for (i, r) in drawn.iter().enumerate() {
            let color = if r.color.is_empty() {
                PALETTE[i % PALETTE.len()].to_string()
            } else {
                r.color.clone()
            };
            let yy: Vec<f64> = vals(r).iter().map(|z| self.y(*z)).collect();
            for chunk in chunks(&self.xs, &yy) {
                self.sheet.lines.push((
                    chunk,
                    "surface".into(),
                    r.title.clone(),
                    color.clone(),
                    0.35,
                ));
            }
            for (t, v) in top_y.iter_mut().zip(&yy) {
                if v.is_finite() {
                    *t = t.max(*v);
                }
            }
        }
        for i in self.taken() {
            if top_y[i].is_finite() {
                self.line(vec![(self.xs[i], 0.0), (self.xs[i], top_y[i])], "ordinate");
            }
        }
        self.events(&top_y);
        self.scale_bar(zmax);
        self.sheet
    }

    fn line(&mut self, coords: Vec<(f64, f64)>, kind: &str) {
        self.sheet
            .lines
            .push((coords, kind.into(), String::new(), "#000000".into(), 0.1));
    }

    #[allow(clippy::too_many_arguments)]
    fn text(
        &mut self,
        x: f64,
        y: f64,
        text: &str,
        rot: f64,
        size: f64,
        halign: &'static str,
        valign: &'static str,
        kind: &'static str,
    ) {
        self.sheet.texts.push(Text {
            x,
            y,
            text: text.to_string(),
            rot,
            size,
            halign,
            valign,
            kind,
        });
    }

    /// Вертикальная подпись по центру строки. Длинная подпись уменьшается,
    /// чтобы уместиться в высоту строки.
    fn vtext(&mut self, x: f64, top: f64, bottom: f64, text: &str, kind: &'static str) {
        let mut size = TEXT;
        let room = top - bottom - 1.0;
        let need = text_width(text, size);
        if need > room && room > 0.0 {
            size = (size * room / need).max(1.2);
        }
        self.text(
            x,
            (top + bottom) / 2.0,
            text,
            270.0,
            size,
            "Center",
            "Half",
            kind,
        );
    }

    fn row_value(&mut self, r: &Row, top: f64, bottom: f64) {
        let vals = r.values.clone().unwrap_or_default();
        for i in self.taken() {
            if vals[i].is_finite() {
                let s = pyfmt::fixed(vals[i], r.decimals);
                self.vtext(self.xs[i], top, bottom, &s, "value");
            }
        }
    }

    fn row_station(&mut self, top: f64, bottom: f64) {
        for i in self.taken() {
            self.line(vec![(self.xs[i], top), (self.xs[i], top - 1.5)], "tick");
            // пикетажное уравнение в две строки: ПК назад = / ПК вперёд
            let label = self.labels[i].replace(" = ", " =\n");
            self.vtext(self.xs[i], top - 1.5, bottom, &label, "station");
        }
    }

    fn row_distance(&mut self, r: &Row, top: f64, bottom: f64) {
        let idx = self.taken();
        for &i in &idx {
            self.line(vec![(self.xs[i], top), (self.xs[i], bottom)], "tick");
        }
        for w in idx.windows(2) {
            let (a, b) = (w[0], w[1]);
            let d = self.ms[b] - self.ms[a];
            let text = if r.decimals > 0 {
                pyfmt::fixed(d, r.decimals)
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_string()
            } else {
                pyfmt::fixed(d, 0)
            };
            if self.xs[b] - self.xs[a] >= text_width(&text, TEXT) {
                let xm = (self.xs[a] + self.xs[b]) / 2.0;
                self.text(
                    xm,
                    (top + bottom) / 2.0,
                    &text,
                    0.0,
                    TEXT,
                    "Center",
                    "Half",
                    "distance",
                );
            }
        }
    }

    fn row_text(&mut self, r: &Row, top: f64, bottom: f64) {
        let (lo, hi) = (self.ms[0], self.ms[self.ms.len() - 1]);
        for (m0, m1, text) in &r.segments {
            let a = m0.min(*m1).max(lo);
            let b = m0.max(*m1).min(hi);
            if b <= a {
                continue;
            }
            let (xa, xb) = (self.x(a), self.x(b));
            for xx in [xa, xb] {
                self.line(vec![(xx, top), (xx, bottom)], "tick");
            }
            if !text.is_empty() && xb - xa >= text_width(text, TEXT) {
                self.text(
                    (xa + xb) / 2.0,
                    (top + bottom) / 2.0,
                    text,
                    0.0,
                    TEXT,
                    "Center",
                    "Half",
                    "section",
                );
            }
        }
    }

    fn row_grade(&mut self, r: &Row, top: f64, bottom: f64) {
        let vals = r.values.clone().unwrap_or_default();
        for g in grade_segments(&self.ms, &vals, r.tolerance) {
            let (xa, xb) = (self.x(g.m0), self.x(g.m1));
            for xx in [xa, xb] {
                self.line(vec![(xx, top), (xx, bottom)], "tick");
            }
            let gr = g.grade();
            let gt = pyfmt::fixed(gr.abs(), r.decimals);
            let lt = if g.length() >= 10.0 {
                pyfmt::fixed(g.length(), 0)
            } else {
                pyfmt::fixed(g.length(), 1)
            };
            let h = top - bottom;
            // диагональ на весь участок, пока она не круче 1:4, на длинном
            // участке - знак уклона шириной 4 высоты строки по центру
            let (a, b) = if xb - xa <= GRADE_FULL * h {
                (xa, xb)
            } else {
                let c = (xa + xb) / 2.0;
                (c - GRADE_FULL * h / 2.0, c + GRADE_FULL * h / 2.0)
            };
            let mid = (top + bottom) / 2.0;
            if gr.abs() < 0.05 {
                self.line(vec![(a, mid), (b, mid)], "grade");
            } else if gr > 0.0 {
                self.line(vec![(a, bottom), (b, top)], "grade");
            } else {
                self.line(vec![(a, top), (b, bottom)], "grade");
            }
            if b - a >= text_width(&gt, TEXT).max(text_width(&lt, TEXT)) + 2.0 {
                if gr >= 0.0 {
                    self.text(
                        a + 1.0,
                        top - h * 0.25,
                        &gt,
                        0.0,
                        TEXT,
                        "Left",
                        "Half",
                        "grade",
                    );
                    self.text(
                        b - 1.0,
                        bottom + h * 0.25,
                        &lt,
                        0.0,
                        TEXT,
                        "Right",
                        "Half",
                        "grade",
                    );
                } else {
                    self.text(
                        b - 1.0,
                        top - h * 0.25,
                        &gt,
                        0.0,
                        TEXT,
                        "Right",
                        "Half",
                        "grade",
                    );
                    self.text(
                        a + 1.0,
                        bottom + h * 0.25,
                        &lt,
                        0.0,
                        TEXT,
                        "Left",
                        "Half",
                        "grade",
                    );
                }
            }
        }
    }

    fn row_plan(&mut self, r: &Row, top: f64, bottom: f64) {
        let mid = (top + bottom) / 2.0;
        let half = (top - bottom) / 2.0 - 1.0;
        self.sheet.lines.push((
            vec![(0.0, mid), (self.sheet.width, mid)],
            "plan_axis".into(),
            r.title.clone(),
            "#000000".into(),
            0.35,
        ));
        // план схематичный: смещения сжимаются, чтобы дальняя точка
        // поместилась в строку, но не растягиваются сверх масштаба чертежа
        let (lo, hi) = (self.ms[0], self.ms[self.ms.len() - 1]);
        let inside: Vec<f64> = self
            .plan_points
            .iter()
            .filter(|(m, _, _)| lo <= *m && *m <= hi)
            .map(|(_, off, _)| off.abs())
            .collect();
        let mut k_off = 1000.0 / self.h;
        if let Some(mx) = inside.iter().copied().reduce(f64::max)
            && mx * k_off > half
        {
            k_off = half / mx;
        }
        let mut k = 0;
        for i in 0..self.p.len() {
            let p = &self.p[i];
            if p.kind == "vertex" && p.turn.abs() >= 1.0 {
                k += 1;
                let side = Msg::new(if p.turn > 0.0 {
                    "влево"
                } else {
                    "вправо"
                });
                let label = (self.tr)(
                    &Msg::new("ВУ{n} {a:.1f}° {side}")
                        .i("n", k)
                        .f("a", p.turn.abs())
                        .m("side", side),
                );
                let x = self.xs[i];
                self.line(vec![(x, mid - 1.5), (x, mid + 1.5)], "plan_turn");
                self.text(
                    x + 0.8,
                    mid - 1.0,
                    &label,
                    0.0,
                    TEXT,
                    "Left",
                    "Top",
                    "plan_turn",
                );
            }
        }
        for (m, off, label) in self.plan_points.clone() {
            if !(lo - 1e-6 <= m && m <= hi + 1e-6) {
                continue;
            }
            let xx = self.x(m);
            let yy = mid + off * k_off;
            self.sheet.marks.push((xx, yy, "plan_point", label.clone()));
            if !label.is_empty() {
                self.text(
                    xx + 1.0,
                    yy,
                    &label,
                    0.0,
                    TEXT,
                    "Left",
                    "Half",
                    "plan_label",
                );
            }
        }
    }

    fn events(&mut self, top_y: &[f64]) {
        for (i, &t) in top_y.iter().enumerate().take(self.p.len()) {
            if self.p[i].kind != "event" || self.p[i].label.is_empty() {
                continue;
            }
            let base = if t.is_finite() { t } else { 0.0 };
            let x = self.xs[i];
            let label = self.p[i].label.clone();
            self.line(vec![(x, base), (x, base + 4.0)], "leader");
            self.text(x, base + 5.0, &label, 270.0, TEXT, "Left", "Half", "event");
        }
    }

    fn scale_bar(&mut self, zmax: f64) {
        let xb = -GAP - 2.0;
        let z0 = self.sheet.horizon;
        let z1 = zmax.max(z0 + 1.0).ceil();
        let steps = [
            0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0,
        ];
        // мелкие деления не чаще 5 мм, подписи не чаще 10 мм на бумаге
        let v = self.v;
        let minor = steps
            .iter()
            .copied()
            .find(|s| s * 1000.0 / v >= 5.0)
            .unwrap_or(1000.0);
        let major = steps
            .iter()
            .copied()
            .find(|s| s * 1000.0 / v >= 10.0 && (s / minor - (s / minor).round()).abs() < 1e-9)
            .unwrap_or(minor);
        let ytop = self.y(z1);
        self.line(vec![(xb, 0.0), (xb, ytop)], "scale");
        let mut k = 0.0;
        while z0 + k * minor <= z1 + 1e-9 {
            let z = z0 + k * minor;
            let yy = self.y(z);
            let labelled = (z / major - (z / major).round()).abs() < 1e-6;
            let tick = if labelled { 2.0 } else { 1.0 };
            self.line(vec![(xb - tick, yy), (xb, yy)], "scale");
            if labelled {
                self.text(
                    xb - 2.5,
                    yy,
                    &pyfmt::fixed(z, 2),
                    0.0,
                    TEXT,
                    "Right",
                    "Half",
                    "scale",
                );
            }
            k += 1.0;
        }
        let xl = -HEADER - GAP + 1.5;
        let captions = [
            Msg::new("Условный горизонт {z:.2f}").f("z", z0),
            Msg::new("М 1:{v:g} по вертикали").f("v", self.v),
            Msg::new("М 1:{h:g} по горизонтали").f("h", self.h),
        ];
        for (k, c) in captions.iter().enumerate() {
            let t = (self.tr)(c);
            self.text(
                xl,
                3.0 + k as f64 * 6.0,
                &t,
                0.0,
                TEXT,
                "Left",
                "Half",
                "caption",
            );
        }
    }
}

fn has_content(r: &Row) -> bool {
    match r.kind.as_str() {
        "value" | "grade" => r
            .values
            .as_ref()
            .is_some_and(|v| v.iter().any(|z| z.is_finite())),
        "text" => !r.segments.is_empty(),
        _ => true,
    }
}

/// Непрерывные куски линии без NaN.
fn chunks(xs: &[f64], ys: &[f64]) -> Vec<Vec<(f64, f64)>> {
    let mut out = Vec::new();
    let mut cur = Vec::new();
    for (x, y) in xs.iter().zip(ys) {
        if y.is_finite() {
            cur.push((*x, *y));
        } else {
            if cur.len() > 1 {
                out.push(std::mem::take(&mut cur));
            }
            cur.clear();
        }
    }
    if cur.len() > 1 {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grades_and_thinning() {
        let ms = [0.0, 50.0, 100.0, 150.0, 200.0];
        let zs = [10.0, 10.5, 11.0, 10.0, 9.0];
        let g = grade_segments(&ms, &zs, 0.02);
        assert_eq!(g.len(), 2);
        assert!((g[0].grade() - 10.0).abs() < 1e-9 && (g[1].grade() + 20.0).abs() < 1e-9);
        let mask = thin(&[0.0, 1.0, 2.0, 5.0], &[1, 6, 1, 1], 3.0);
        assert_eq!(mask, vec![false, true, false, true]);
        assert!((text_width("ПК 1+00", 2.5) - (0.62 * 2.5 * 7.0 + 0.5)).abs() < 1e-12);
    }
}
