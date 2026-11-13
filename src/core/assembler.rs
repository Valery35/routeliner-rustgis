// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 2.0 или новее.
// SPDX-License-Identifier: GPL-2.0-or-later

//! Сборка маршрута из частей (`routeliner/core/assembler.py`).
//!
//! На вход - полилинии всех объектов с одним ID маршрута в порядке
//! хранения, мультилинии уже разложены на части. Порядок хранения ничего
//! не значит: части упорядочиваются по совпадению концов в пределах
//! допуска. Направление маршрута - то, при котором большая по длине доля
//! частей сохраняет направление оцифровки, `reverse` переворачивает итог.

use super::errors::{CoreError, ErrorCode};
use super::route::{P4, RouteGap, RouteGeometry};
use crate::pyfmt::{self, Msg};

struct Part {
    coords: Vec<P4>,
    length: f64,
}

/// Цепочка частей: (номер части, перевёрнута ли) по порядку.
type Chain = Vec<(usize, bool)>;

/// В общем узле двух частей M разный: узел остаётся дважды, и повтор
/// вершины с другим M читается как пикетажное уравнение.
fn m_jump(a: &P4, b: &P4) -> bool {
    !a[3].is_nan() && !b[3].is_nan() && (a[3] - b[3]).abs() > 1e-6
}

fn plan_length(c: &[P4]) -> f64 {
    let h: Vec<f64> = c
        .windows(2)
        .map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]))
        .collect();
    pyfmt::np_sum(&h)
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RouteAssembler {
    pub tol: f64,
    pub allow_gaps: bool,
    pub use_z: bool,
    pub reverse: bool,
}

impl Default for RouteAssembler {
    fn default() -> Self {
        Self {
            tol: 0.01,
            allow_gaps: false,
            use_z: false,
            reverse: false,
        }
    }
}

impl RouteAssembler {
    fn err(code: ErrorCode, route_id: &str, msg: Msg) -> CoreError {
        CoreError::new(code, msg).route(Some(route_id))
    }

    pub fn build(
        &self,
        route_id: &str,
        parts: &[Vec<P4>],
        geographic_crs: bool,
    ) -> Result<RouteGeometry, CoreError> {
        if geographic_crs {
            return Err(Self::err(
                ErrorCode::GeographicCrs,
                route_id,
                Msg::new(
                    "длины в географической системе координат считаются в градусах. Перепроецируйте маршруты в метрическую СК",
                ),
            ));
        }
        let ps: Vec<Part> = parts
            .iter()
            .filter(|c| c.len() >= 2)
            .map(|c| Part {
                coords: c.clone(),
                length: plan_length(c),
            })
            .filter(|p| p.length > 0.0)
            .collect();
        if ps.is_empty() {
            return Err(Self::err(
                ErrorCode::RouteNotFound,
                route_id,
                Msg::new("нет геометрии"),
            ));
        }
        let chains = self.chains(&ps, route_id)?;
        if chains.len() > 1 && !self.allow_gaps {
            let d = nearest_gap(&ps, &chains);
            return Err(Self::err(
                ErrorCode::RouteGap,
                route_id,
                Msg::new("маршрут распадается на {n} куска, ближайший разрыв {d:.3f} больше допуска {t:g}")
                    .i("n", chains.len() as i64)
                    .f("d", d)
                    .f("t", self.tol),
            ));
        }
        let chains: Vec<Chain> = chains.into_iter().map(|ch| orient(&ps, ch)).collect();
        let chains = order_chains(&ps, chains);
        let mut coords: Vec<Vec<P4>> = chains.iter().map(|ch| merge(&ps, ch)).collect();
        if self.reverse {
            coords.reverse();
            for c in &mut coords {
                c.reverse();
            }
        }
        let mut gaps = Vec::new();
        let mut m = 0.0;
        for w in coords.windows(2) {
            m += plan_length(&w[0]);
            let (a, b) = (&w[0][w[0].len() - 1], &w[1][0]);
            gaps.push(RouteGap {
                m,
                distance: (b[0] - a[0]).hypot(b[1] - a[1]),
            });
        }
        Ok(RouteGeometry::new(route_id, coords, self.use_z, gaps))
    }

    /// Цепочки частей по узлам: концы кластеризуются по допуску.
    fn chains(&self, ps: &[Part], route_id: &str) -> Result<Vec<Chain>, CoreError> {
        let ends: Vec<[f64; 2]> = ps
            .iter()
            .flat_map(|p| {
                let (a, b) = (p.coords[0], p.coords[p.coords.len() - 1]);
                [[a[0], a[1]], [b[0], b[1]]]
            })
            .collect();
        let mut node = vec![0usize; ends.len()];
        let mut centers: Vec<[f64; 2]> = Vec::new();
        for (i, e) in ends.iter().enumerate() {
            match centers.iter().position(|c| dist(*e, *c) <= self.tol) {
                Some(k) => node[i] = k,
                None => {
                    node[i] = centers.len();
                    centers.push(*e);
                }
            }
        }
        let mut deg = vec![0usize; centers.len()];
        for &n in &node {
            deg[n] += 1;
        }
        if let Some(&max) = deg.iter().max()
            && max > 2
        {
            let k = deg.iter().position(|&d| d == max).unwrap_or(0);
            let [x, y] = centers[k];
            return Err(Self::err(
                ErrorCode::RouteBranching,
                route_id,
                Msg::new("в точке ({x:.2f}, {y:.2f}) сходятся {n} конца частей")
                    .f("x", x)
                    .f("y", y)
                    .i("n", max as i64),
            ));
        }
        let mut adj: Vec<Vec<(usize, usize)>> = vec![Vec::new(); centers.len()];
        for pi in 0..ps.len() {
            let (a, b) = (node[2 * pi], node[2 * pi + 1]);
            adj[a].push((pi, b));
            adj[b].push((pi, a));
        }
        let mut used = vec![false; ps.len()];
        let mut chains = Vec::new();
        // сначала цепочки от висячих концов, затем замкнутые кольца
        let starts: Vec<usize> = (0..centers.len())
            .filter(|&n| deg[n] == 1)
            .chain(0..centers.len())
            .collect();
        for s in starts {
            for k in 0..adj[s].len() {
                let (pi, _) = adj[s][k];
                if used[pi] {
                    continue;
                }
                let mut chain = Vec::new();
                let mut cur = s;
                while let Some(&(q, other)) = adj[cur].iter().find(|(q, _)| !used[*q]) {
                    used[q] = true;
                    // перевёрнута, если входим с конца
                    chain.push((q, node[2 * q] != cur));
                    cur = other;
                }
                chains.push(chain);
            }
        }
        Ok(chains)
    }
}

fn orient(ps: &[Part], chain: Chain) -> Chain {
    let keep: f64 = chain
        .iter()
        .filter(|(_, r)| !r)
        .map(|(q, _)| ps[*q].length)
        .sum();
    let flip: f64 = chain
        .iter()
        .filter(|(_, r)| *r)
        .map(|(q, _)| ps[*q].length)
        .sum();
    if flip > keep {
        chain.iter().rev().map(|&(q, r)| (q, !r)).collect()
    } else {
        chain
    }
}

fn endpoints(ps: &[Part], chain: &Chain) -> ([f64; 2], [f64; 2]) {
    let (q0, r0) = chain[0];
    let (q1, r1) = chain[chain.len() - 1];
    let (c0, c1) = (&ps[q0].coords, &ps[q1].coords);
    let s = if r0 { c0[c0.len() - 1] } else { c0[0] };
    let e = if r1 { c1[0] } else { c1[c1.len() - 1] };
    ([s[0], s[1]], [e[0], e[1]])
}

/// Куски при разрывах: сначала кусок с первой частью в порядке хранения,
/// дальше жадно к ближайшему началу, куски «позади» - в начало.
fn order_chains(ps: &[Part], chains: Vec<Chain>) -> Vec<Chain> {
    if chains.len() == 1 {
        return chains;
    }
    let first = chains
        .iter()
        .position(|ch| ch.iter().any(|(q, _)| *q == 0))
        .unwrap_or(0);
    let mut order = vec![first];
    let mut rest: Vec<usize> = (0..chains.len()).filter(|&i| i != first).collect();
    let argmin = |rest: &[usize], f: &dyn Fn(usize) -> f64| -> usize {
        let mut best = rest[0];
        for &i in rest {
            if f(i) < f(best) {
                best = i;
            }
        }
        best
    };
    while !rest.is_empty() {
        let head = endpoints(ps, &chains[order[0]]).0;
        let tail = endpoints(ps, &chains[order[order.len() - 1]]).1;
        let to_fwd = |i: usize| dist(endpoints(ps, &chains[i]).0, tail);
        let to_back = |i: usize| dist(endpoints(ps, &chains[i]).1, head);
        let fwd = argmin(&rest, &to_fwd);
        let back = argmin(&rest, &to_back);
        if to_fwd(fwd) <= to_back(back) {
            order.push(fwd);
            rest.retain(|&i| i != fwd);
        } else {
            order.insert(0, back);
            rest.retain(|&i| i != back);
        }
    }
    order.into_iter().map(|i| chains[i].clone()).collect()
}

fn merge(ps: &[Part], chain: &Chain) -> Vec<P4> {
    let mut out: Vec<P4> = Vec::new();
    for &(q, rev) in chain {
        let mut c = ps[q].coords.clone();
        if rev {
            c.reverse();
        }
        let skip = usize::from(out.last().is_some_and(|last| !m_jump(last, &c[0])));
        out.extend_from_slice(&c[skip..]); // общий узел не дублируется
    }
    out
}

fn nearest_gap(ps: &[Part], chains: &[Chain]) -> f64 {
    let pts: Vec<[f64; 2]> = chains
        .iter()
        .flat_map(|ch| {
            let (a, b) = endpoints(ps, ch);
            [a, b]
        })
        .collect();
    let mut best = f64::INFINITY;
    for i in (0..pts.len()).step_by(2) {
        for j in (0..pts.len()).step_by(2) {
            if i == j {
                continue;
            }
            for a in &pts[i..i + 2] {
                for b in &pts[j..j + 2] {
                    best = best.min(dist(*a, *b));
                }
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(pts: &[(f64, f64)]) -> Vec<P4> {
        pts.iter()
            .map(|&(x, y)| [x, y, f64::NAN, f64::NAN])
            .collect()
    }

    #[test]
    fn shuffled_parts_gap_and_branch() {
        let asm = RouteAssembler::default();
        // три части в произвольном порядке и направлении
        let parts = vec![
            line(&[(200.0, 0.0), (100.0, 0.0)]),
            line(&[(200.0, 0.0), (300.0, 0.0)]),
            line(&[(0.0, 0.0), (100.0, 0.0)]),
        ];
        let r = asm.build("R3", &parts, false).unwrap();
        assert_eq!(r.length, 300.0);
        assert_eq!(r.parts.len(), 1);
        assert_eq!(r.parts[0].len(), 4);
        assert_eq!(r.parts[0][0][0], 0.0);
        let rev = RouteAssembler {
            reverse: true,
            ..asm
        }
        .build("R3", &parts, false)
        .unwrap();
        assert_eq!(rev.parts[0][0][0], 300.0);
        // разрыв 15 м
        let gap = vec![
            line(&[(0.0, 0.0), (0.0, 300.0)]),
            line(&[(0.0, 315.0), (0.0, 615.0)]),
        ];
        let e = asm.build("R4", &gap, false).unwrap_err();
        assert_eq!(e.code, ErrorCode::RouteGap);
        assert_eq!(
            e.msg.render(&|t| t.to_string()),
            "маршрут распадается на 2 куска, ближайший разрыв 15.000 больше допуска 0.01"
        );
        let ok = RouteAssembler {
            allow_gaps: true,
            ..asm
        }
        .build("R4", &gap, false)
        .unwrap();
        assert_eq!(ok.gaps.len(), 1);
        assert_eq!(ok.gaps[0].distance, 15.0);
        assert_eq!(ok.length, 600.0);
        // развилка
        let fork = vec![
            line(&[(0.0, 0.0), (10.0, 0.0)]),
            line(&[(10.0, 0.0), (20.0, 0.0)]),
            line(&[(10.0, 0.0), (10.0, 10.0)]),
        ];
        assert_eq!(
            asm.build("F", &fork, false).unwrap_err().code,
            ErrorCode::RouteBranching
        );
    }
}
