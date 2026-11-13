// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 3.0 или новее.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Геометрия маршрута (`routeliner/core/route.py`): упорядоченные части,
//! накопленные длины.
//!
//! Маршрут - последовательность частей. Мера m идёт непрерывно по частям,
//! разрывы между частями в длину не входят.
//!
//! M-значения вершин хранятся отдельно от координат, в
//! [`RouteGeometry::measures`], чтобы M не путался с Z. Смещение
//! положительное влево по ходу маршрута, азимут - в градусах от севера
//! по часовой стрелке.

use super::errors::{CoreError, ErrorCode};
use crate::pyfmt::{self, Msg};

const EPS: f64 = 1e-9;

/// Вершина: x, y, z. Высоты нет - NaN.
pub type P3 = [f64; 3];
/// Вершина с M: x, y, z, m. Нет - NaN.
pub type P4 = [f64; 4];
/// Кусок участка: вершины и мера по оси каждой вершины.
pub type Piece = (Vec<P3>, Vec<f64>);

/// Разрыв между частями: мера, где он стоит, и его длина по прямой.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RouteGap {
    pub m: f64,
    pub distance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocatedPoint {
    pub x: f64,
    pub y: f64,
    pub z: Option<f64>,
    pub azimuth: f64,
}

/// Привязка точки к маршруту.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projection {
    pub m: f64,
    /// Со знаком, плюс влево.
    pub offset: f64,
    pub x: f64,
    pub y: f64,
    pub part: usize,
}

/// Точка на маршруте пакетного расчёта.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct At {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub azimuth: f64,
    pub ok: bool,
}

/// Вершина маршрута с углом поворота в плане (градусы, плюс влево)
/// и изменением уклона по Z.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub m: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub turn: f64,
    pub dgrade: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouteGeometry {
    pub route_id: String,
    pub parts: Vec<Vec<P3>>,
    pub use_z: bool,
    pub gaps: Vec<RouteGap>,
    /// M вершин по частям или None.
    pub measures: Option<Vec<Vec<f64>>>,
    pub length: f64,
    part_m: Vec<(f64, f64)>,
    a: Vec<P3>,
    b: Vec<P3>,
    len: Vec<f64>,
    m0: Vec<f64>,
    part: Vec<usize>,
    has_z: bool,
}

fn seg_len(a: &P3, b: &P3, use_z: bool) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    if use_z {
        let dz = b[2] - a[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    } else {
        (dx * dx + dy * dy).sqrt()
    }
}

fn lerp(a: &P3, b: &P3, t: f64) -> P3 {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// numpy `searchsorted(..., side="right")`.
fn search_right(a: &[f64], x: f64) -> usize {
    a.partition_point(|v| *v <= x)
}

impl RouteGeometry {
    /// Части с M (`P4`): столбец M отделяется от координат.
    pub fn new(route_id: &str, parts: Vec<Vec<P4>>, use_z: bool, gaps: Vec<RouteGap>) -> Self {
        let measures: Vec<Vec<f64>> = parts
            .iter()
            .map(|c| c.iter().map(|p| p[3]).collect())
            .collect();
        let has_m = measures.iter().flatten().any(|v| v.is_finite());
        let parts: Vec<Vec<P3>> = parts
            .into_iter()
            .map(|c| c.into_iter().map(|p| [p[0], p[1], p[2]]).collect())
            .collect();
        let has_z = parts.iter().flatten().any(|p| p[2].is_finite());
        let use_3d = use_z && has_z;
        let mut g = Self {
            route_id: route_id.to_string(),
            parts,
            use_z,
            gaps,
            measures: has_m.then_some(measures),
            length: 0.0,
            part_m: Vec::new(),
            a: Vec::new(),
            b: Vec::new(),
            len: Vec::new(),
            m0: Vec::new(),
            part: Vec::new(),
            has_z,
        };
        let mut m = 0.0;
        for (pi, c) in g.parts.iter().enumerate() {
            let mut lens = Vec::new();
            let mut acc = 0.0;
            for w in c.windows(2) {
                let ln = seg_len(&w[0], &w[1], use_3d);
                if ln > EPS {
                    g.a.push(w[0]);
                    g.b.push(w[1]);
                    g.len.push(ln);
                    g.m0.push(m + acc);
                    g.part.push(pi);
                    acc += ln;
                    lens.push(ln);
                }
            }
            let total = pyfmt::np_sum(&lens);
            g.part_m.push((m, m + total));
            m += total;
        }
        g.length = m;
        g
    }

    pub fn has_m(&self) -> bool {
        self.measures.is_some()
    }

    pub fn has_z(&self) -> bool {
        self.has_z
    }

    /// (мера по оси, M) для всех вершин по ходу маршрута, включая вершины
    /// с нулевым отрезком между ними: две такие вершины с разным M - это
    /// пикетажное уравнение. Пусто, если у маршрута нет M.
    pub fn vertex_measures(&self) -> Vec<(f64, f64)> {
        let Some(ms) = &self.measures else {
            return Vec::new();
        };
        let use_3d = self.use_z && self.has_z;
        let mut out = Vec::new();
        for (pi, c) in self.parts.iter().enumerate() {
            let mut m = self.part_m[pi].0;
            for (k, p) in c.iter().enumerate() {
                if k > 0 {
                    m += seg_len(&c[k - 1], p, use_3d);
                }
                out.push((m, ms[pi][k]));
            }
        }
        out
    }

    fn check(&self, m: f64) -> Option<CoreError> {
        if self.length <= 0.0 {
            return Some(
                CoreError::new(ErrorCode::RouteNotFound, Msg::new("маршрут нулевой длины"))
                    .route(Some(&self.route_id)),
            );
        }
        if m < -1e-6 || m > self.length + 1e-6 {
            return Some(
                CoreError::new(
                    ErrorCode::MeasureOutOfRange,
                    Msg::new("мера {m:.3f} вне 0..{L:.3f}")
                        .f("m", m)
                        .f("L", self.length),
                )
                .route(Some(&self.route_id)),
            );
        }
        None
    }

    /// Пакетный расчёт точек: координаты, высота, азимут и признак, что
    /// мера внутри маршрута.
    pub fn points_at(&self, ms: &[f64], offsets: &[f64]) -> Vec<At> {
        let n = self.m0.len();
        ms.iter()
            .enumerate()
            .map(|(k, &m)| {
                let off = offsets.get(k).or(offsets.first()).copied().unwrap_or(0.0);
                let ok = m >= -1e-6 && m <= self.length + 1e-6 && self.length > 0.0;
                if n == 0 {
                    return At {
                        x: f64::NAN,
                        y: f64::NAN,
                        z: f64::NAN,
                        azimuth: f64::NAN,
                        ok: false,
                    };
                }
                let i = search_right(&self.m0, m.clamp(0.0, self.length))
                    .saturating_sub(1)
                    .min(n - 1);
                let t = ((m - self.m0[i]) / self.len[i]).clamp(0.0, 1.0);
                let (a, b) = (self.a[i], self.b[i]);
                let p = lerp(&a, &b, t);
                let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                let h = dx.hypot(dy);
                let h = if h > EPS { h } else { 1.0 };
                let (nx, ny) = (-dy / h, dx / h);
                At {
                    x: p[0] + nx * off,
                    y: p[1] + ny * off,
                    z: if self.has_z { p[2] } else { f64::NAN },
                    azimuth: dx.atan2(dy).to_degrees().rem_euclid(360.0),
                    ok,
                }
            })
            .collect()
    }

    pub fn point_at(&self, m: f64, offset: f64) -> Result<LocatedPoint, CoreError> {
        if let Some(e) = self.check(m) {
            return Err(e);
        }
        let r = self.points_at(&[m], &[offset])[0];
        Ok(LocatedPoint {
            x: r.x,
            y: r.y,
            z: (!r.z.is_nan()).then_some(r.z),
            azimuth: r.azimuth,
        })
    }

    /// Вершины маршрута по частям: мера, координаты, угол поворота в плане
    /// и изменение уклона.
    pub fn vertices(&self) -> Vec<Vertex> {
        let mut out = Vec::new();
        for pi in 0..self.part_m.len() {
            let idx: Vec<usize> = (0..self.part.len())
                .filter(|&i| self.part[i] == pi)
                .collect();
            let Some(&last) = idx.last() else {
                continue;
            };
            let mut pts: Vec<P3> = idx.iter().map(|&i| self.a[i]).collect();
            pts.push(self.b[last]);
            let mut ms: Vec<f64> = idx.iter().map(|&i| self.m0[i]).collect();
            ms.push(self.m0[last] + self.len[last]);
            let d: Vec<(f64, f64)> = idx
                .iter()
                .map(|&i| (self.b[i][0] - self.a[i][0], self.b[i][1] - self.a[i][1]))
                .collect();
            let az: Vec<f64> = d.iter().map(|(dx, dy)| dy.atan2(*dx)).collect();
            let np = pts.len();
            let mut turn = vec![0.0; np];
            if az.len() > 1 {
                for k in 1..np - 1 {
                    let v = (az[k] - az[k - 1] + std::f64::consts::PI)
                        .rem_euclid(2.0 * std::f64::consts::PI)
                        - std::f64::consts::PI;
                    turn[k] = v.to_degrees();
                }
            }
            let mut g = vec![0.0; np];
            if self.has_z && idx.len() > 1 {
                let gr: Vec<f64> = idx
                    .iter()
                    .zip(&d)
                    .map(|(&i, (dx, dy))| (self.b[i][2] - self.a[i][2]) / dx.hypot(*dy).max(EPS))
                    .collect();
                for k in 1..np - 1 {
                    let v = gr[k] - gr[k - 1];
                    g[k] = if v.is_nan() { 0.0 } else { v };
                }
            }
            for k in 0..np {
                out.push(Vertex {
                    m: ms[k],
                    x: pts[k][0],
                    y: pts[k][1],
                    z: if self.has_z { pts[k][2] } else { f64::NAN },
                    turn: turn[k],
                    dgrade: g[k],
                });
            }
        }
        out
    }

    /// Участок маршрута кусками (несколько, если он проходит через разрыв)
    /// и мера по оси каждой вершины куска.
    pub fn substring_m(
        &self,
        m_from: f64,
        m_to: f64,
        offset: f64,
    ) -> Result<Vec<Piece>, CoreError> {
        if m_from >= m_to {
            return Err(CoreError::new(
                ErrorCode::FromGeTo,
                Msg::new("начало {a:.3f} не меньше конца {b:.3f}")
                    .f("a", m_from)
                    .f("b", m_to),
            )
            .route(Some(&self.route_id)));
        }
        for m in [m_from, m_to] {
            if let Some(e) = self.check(m) {
                return Err(e);
            }
        }
        let mut pieces = Vec::new();
        for (pi, &(p0, p1)) in self.part_m.iter().enumerate() {
            let (lo, hi) = (m_from.max(p0), m_to.min(p1));
            if hi - lo <= 1e-9 {
                continue;
            }
            let sel: Vec<usize> = (0..self.m0.len())
                .filter(|&i| self.part[i] == pi && self.m0[i] > lo + 1e-9 && self.m0[i] < hi - 1e-9)
                .collect();
            let mut coords = vec![self.in_part(lo, pi)];
            coords.extend(sel.iter().map(|&i| self.a[i]));
            coords.push(self.in_part(hi, pi));
            let mut ms = vec![lo];
            ms.extend(sel.iter().map(|&i| self.m0[i]));
            ms.push(hi);
            if offset != 0.0 {
                coords = offset_polyline(&coords, offset, 4.0);
            }
            pieces.push((coords, ms));
        }
        Ok(pieces)
    }

    /// Участок кусками без мер.
    pub fn substring(
        &self,
        m_from: f64,
        m_to: f64,
        offset: f64,
    ) -> Result<Vec<Vec<P3>>, CoreError> {
        Ok(self
            .substring_m(m_from, m_to, offset)?
            .into_iter()
            .map(|(c, _)| c)
            .collect())
    }

    /// Точка с мерой m строго внутри части `pi`: на стыке частей важно,
    /// с какой стороны разрыва её брать.
    fn in_part(&self, m: f64, pi: usize) -> P3 {
        let idx: Vec<usize> = (0..self.part.len())
            .filter(|&i| self.part[i] == pi)
            .collect();
        let m0: Vec<f64> = idx.iter().map(|&i| self.m0[i]).collect();
        let k = search_right(&m0, m)
            .saturating_sub(1)
            .min(idx.len().saturating_sub(1));
        let j = idx[k];
        let t = ((m - self.m0[j]) / self.len[j]).clamp(0.0, 1.0);
        lerp(&self.a[j], &self.b[j], t)
    }

    /// Ближайшая точка маршрута: мера и смещение со знаком (плюс влево).
    pub fn locate(&self, x: f64, y: f64) -> Result<Projection, CoreError> {
        if self.m0.is_empty() {
            return Err(CoreError::new(
                ErrorCode::RouteNotFound,
                Msg::new("маршрут нулевой длины"),
            )
            .route(Some(&self.route_id)));
        }
        let mut best: Option<(usize, f64, f64, f64, f64)> = None;
        for i in 0..self.m0.len() {
            let (a, b) = (self.a[i], self.b[i]);
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let dd = dx * dx + dy * dy;
            let t = (((x - a[0]) * dx + (y - a[1]) * dy) / dd).clamp(0.0, 1.0);
            let (px, py) = (a[0] + dx * t, a[1] + dy * t);
            let dist = (x - px).hypot(y - py);
            if best.is_none_or(|bst| dist < bst.1) {
                best = Some((i, dist, t, px, py));
            }
        }
        let Some((i, dist, t, px, py)) = best else {
            return Err(CoreError::new(
                ErrorCode::RouteNotFound,
                Msg::new("маршрут нулевой длины"),
            ));
        };
        let (a, b) = (self.a[i], self.b[i]);
        let cross = (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0]);
        let sign = if cross >= 0.0 { 1.0 } else { -1.0 };
        Ok(Projection {
            m: self.m0[i] + t * self.len[i],
            offset: sign * dist,
            x: px,
            y: py,
            part: self.part[i],
        })
    }
}

/// Параллельная полилиния (плюс влево) со срезанием острых углов, без
/// устранения самопересечений.
pub fn offset_polyline(coords: &[P3], offset: f64, miter_limit: f64) -> Vec<P3> {
    let n = coords.len();
    if n < 2 {
        return coords.to_vec();
    }
    // нормаль каждого отрезка
    let seg: Vec<(f64, f64)> = coords
        .windows(2)
        .map(|w| {
            let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
            let h = dx.hypot(dy);
            let h = if h < EPS { 1.0 } else { h };
            (-dy / h, dx / h)
        })
        .collect();
    let mut out = coords.to_vec();
    for i in 0..n {
        // биссектриса в вершине
        let v = if i == 0 {
            seg[0]
        } else if i == n - 1 {
            seg[n - 2]
        } else {
            (seg[i - 1].0 + seg[i].0, seg[i - 1].1 + seg[i].1)
        };
        let ln = v.0.hypot(v.1);
        let ln = if ln < EPS { 1.0 } else { ln };
        let vn = (v.0 / ln, v.1 / ln);
        let sn = seg[i.saturating_sub(1)];
        let cos_half = vn.0 * sn.0 + vn.1 * sn.1;
        let k = (1.0 / cos_half.max(1.0 / miter_limit)).min(miter_limit);
        out[i][0] = coords[i][0] + vn.0 * (offset * k);
        out[i][1] = coords[i][1] + vn.1 * (offset * k);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p4(x: f64, y: f64) -> P4 {
        [x, y, f64::NAN, f64::NAN]
    }

    #[test]
    fn points_substring_and_locate() {
        // Г: 100 м на восток, затем 100 м на север
        let r = RouteGeometry::new(
            "R",
            vec![vec![p4(0.0, 0.0), p4(100.0, 0.0), p4(100.0, 100.0)]],
            false,
            vec![],
        );
        assert_eq!(r.length, 200.0);
        let p = r.point_at(50.0, 10.0).unwrap();
        assert!((p.x - 50.0).abs() < 1e-12 && (p.y - 10.0).abs() < 1e-12);
        assert!((p.azimuth - 90.0).abs() < 1e-12);
        assert!(r.point_at(250.0, 0.0).is_err());
        let s = r.substring_m(50.0, 150.0, 0.0).unwrap();
        assert_eq!(s[0].0.len(), 3);
        assert_eq!(s[0].1, vec![50.0, 100.0, 150.0]);
        let l = r.locate(50.0, -5.0).unwrap();
        assert!((l.m - 50.0).abs() < 1e-12 && (l.offset + 5.0).abs() < 1e-12);
        let v = r.vertices();
        assert_eq!(v.len(), 3);
        assert!((v[1].turn - 90.0).abs() < 1e-9);
        // смещение влево на изломе - по биссектрисе со срезом угла
        let o = offset_polyline(&s[0].0, 10.0, 4.0);
        assert!((o[1][0] - 90.0).abs() < 1e-9 && (o[1][1] - 10.0).abs() < 1e-9);
    }

    #[test]
    fn measures_split_from_coordinates() {
        let r = RouteGeometry::new(
            "R",
            vec![vec![
                [0.0, 0.0, f64::NAN, 1000.0],
                [100.0, 0.0, f64::NAN, 1100.0],
                [100.0, 0.0, f64::NAN, 1090.0],
                [200.0, 0.0, f64::NAN, 1190.0],
            ]],
            false,
            vec![],
        );
        assert!(r.has_m() && !r.has_z());
        assert_eq!(r.length, 200.0);
        let vm = r.vertex_measures();
        assert_eq!(vm[2], (100.0, 1090.0));
    }
}
