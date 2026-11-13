// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 2.0 или новее.
// SPDX-License-Identifier: GPL-2.0-or-later

//! Система пикетажа маршрута (`routeliner/core/chainage.py`).
//!
//! Опорная величина - геометрическая мера m, длина по оси от начала
//! маршрута. Пикетаж задан кусочно-линейно: участки между реперами
//! ведомости и точками пикетажных уравнений. Внутри участка пикет растёт
//! линейно по m, между участками возможен скачок.
//!
//! - Прямая вставка (ПК назад < ПК вперёд) - часть значений пропущена.
//! - Обратная вставка (ПК назад > ПК вперёд) - часть значений встречается
//!   дважды.

use super::errors::{CoreError, ErrorCode};
use super::route::P3;
use crate::pyfmt::{self, Arg, Msg};

pub const EPS: f64 = 1e-6;
/// Допустимое отклонение масштаба участка от 1 при выборе стороны вставки.
pub const SCALE_TOL: f64 = 0.02;

/// Строка ведомости: пикет, мера или None, пикет вперёд или None.
pub type TableRow = (f64, Option<f64>, Option<f64>);

/// Репер ведомости. У точки уравнения два пикета, назад и вперёд.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LedgerPoint {
    pub m: f64,
    pub station: f64,
    pub station_ahead: Option<f64>,
}

impl LedgerPoint {
    pub fn new(m: f64, station: f64, station_ahead: Option<f64>) -> Self {
        Self {
            m,
            station,
            station_ahead,
        }
    }

    pub fn ahead(&self) -> f64 {
        self.station_ahead.unwrap_or(self.station)
    }

    pub fn is_equation(&self) -> bool {
        self.station_ahead
            .is_some_and(|a| (a - self.station).abs() > EPS)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChainageSection {
    pub index: usize,
    pub m_from: f64,
    pub m_to: f64,
    pub st_from: f64,
    pub st_to: f64,
}

impl ChainageSection {
    /// Метры пикетажа на метр геометрии. Отклонение от 1 - невязка.
    pub fn scale(&self) -> f64 {
        let dm = self.m_to - self.m_from;
        if dm > 0.0 {
            (self.st_to - self.st_from) / dm
        } else {
            1.0
        }
    }

    pub fn station_at(&self, m: f64) -> f64 {
        self.st_from + (m - self.m_from) * self.scale()
    }

    pub fn measure_at(&self, st: f64) -> f64 {
        let k = self.scale();
        if k != 0.0 {
            self.m_from + (st - self.st_from) / k
        } else {
            self.m_from
        }
    }

    pub fn contains_station(&self, st: f64) -> bool {
        self.st_from - EPS <= st && st <= self.st_to + EPS
    }
}

/// Имя системы: текст пользователя или слово модуля, которое переводится.
#[derive(Debug, Clone, PartialEq)]
pub enum SystemName {
    Text(String),
    Word(&'static str),
}

impl SystemName {
    fn arg(&self) -> Arg {
        match self {
            SystemName::Text(s) => Arg::S(s.clone()),
            SystemName::Word(w) => Arg::M(Box::new(Msg::new(w))),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChainageSystem {
    pub name: SystemName,
    pub sections: Vec<ChainageSection>,
}

fn bad(name: &SystemName, why: Msg) -> CoreError {
    let mut msg = Msg::new("система «{name}»: {why}");
    msg.args.push(("name", name.arg()));
    CoreError::new(ErrorCode::LedgerInvalid, msg.m("why", why))
}

/// Python `bisect_right` по возрастающему списку.
fn bisect_right(a: &[f64], x: f64) -> usize {
    a.partition_point(|v| *v <= x)
}

/// Python `bisect_left`.
fn bisect_left(a: &[f64], x: f64) -> usize {
    a.partition_point(|v| *v < x)
}

impl ChainageSystem {
    pub fn from_length(name: SystemName, length: f64, start_station: f64) -> Self {
        Self {
            name,
            sections: vec![ChainageSection {
                index: 0,
                m_from: 0.0,
                m_to: length,
                st_from: start_station,
                st_to: start_station + length,
            }],
        }
    }

    /// Ведомость как список реперов с известной мерой m. С длиной маршрута
    /// участки до первого и после последнего репера достраиваются
    /// с масштабом 1, иначе мера вне реперов - ошибка.
    pub fn from_points(
        name: SystemName,
        points: &[LedgerPoint],
        route_length: Option<f64>,
    ) -> Result<Self, CoreError> {
        let mut pts = points.to_vec();
        pts.sort_by(|a, b| a.m.total_cmp(&b.m));
        if pts.len() < 2 && route_length.is_none() {
            return Err(bad(&name, Msg::new("нужно не меньше двух реперов")));
        }
        if pts.is_empty() {
            return Err(bad(&name, Msg::new("ведомость пуста")));
        }
        for w in pts.windows(2) {
            if w[1].m - w[0].m <= EPS {
                return Err(bad(
                    &name,
                    Msg::new("два репера с одной мерой m={m:.3f}").f("m", w[0].m),
                ));
            }
        }
        if let Some(len) = route_length {
            if pts[0].m > EPS {
                let p = pts[0];
                pts.insert(0, LedgerPoint::new(0.0, p.station - p.m, None));
            }
            let p = pts[pts.len() - 1];
            if p.m < len - EPS {
                pts.push(LedgerPoint::new(len, p.ahead() + (len - p.m), None));
            }
        }
        let mut sections = Vec::new();
        for (i, w) in pts.windows(2).enumerate() {
            let (a, b) = (w[0], w[1]);
            let s = ChainageSection {
                index: i,
                m_from: a.m,
                m_to: b.m,
                st_from: a.ahead(),
                st_to: b.station,
            };
            if s.st_to - s.st_from <= EPS {
                return Err(bad(
                    &name,
                    Msg::new(
                        "пикетаж не растёт на участке m {a:.2f}..{b:.2f}, от {s0:.2f} до {s1:.2f}",
                    )
                    .f("a", a.m)
                    .f("b", b.m)
                    .f("s0", s.st_from)
                    .f("s1", s.st_to),
                ));
            }
            sections.push(s);
        }
        Ok(Self { name, sections })
    }

    /// Пикетаж из M-значений вершин: пары (мера по оси, M) по ходу
    /// маршрута, M нет - NaN. `factor` переводит единицы M в метры.
    ///
    /// Две вершины с одной мерой и разным M дают уравнение. Вершины без M
    /// пропускаются. Вершины на одной прямой пикетажа склеиваются, чтобы
    /// участки начинались только там, где меняется масштаб или стоит
    /// уравнение.
    pub fn from_measures(
        name: SystemName,
        pairs: &[(f64, f64)],
        route_length: Option<f64>,
        factor: f64,
    ) -> Result<Self, CoreError> {
        let mut pts: Vec<LedgerPoint> = Vec::new();
        for &(mg, mv) in pairs {
            if mv.is_nan() {
                continue;
            }
            let mv = mv * factor;
            if let Some(last) = pts.last_mut()
                && mg - last.m <= EPS
            {
                if (mv - last.ahead()).abs() > EPS {
                    *last = LedgerPoint::new(last.m, last.station, Some(mv));
                }
                continue;
            }
            pts.push(LedgerPoint::new(mg, mv, None));
        }
        if pts.len() < 2 {
            return Err(bad(&name, Msg::new("у маршрута меньше двух вершин с M")));
        }
        let mut out = vec![pts[0]];
        for i in 1..pts.len() - 1 {
            let (p, q) = (pts[i], pts[i + 1]);
            let a = out[out.len() - 1];
            if !p.is_equation() {
                let k1 = (p.station - a.ahead()) / (p.m - a.m);
                let k2 = (q.station - p.station) / (q.m - p.m);
                if (k1 - k2).abs() <= 1e-9 * 1f64.max(k1.abs()) {
                    continue;
                }
            }
            out.push(p);
        }
        out.push(pts[pts.len() - 1]);
        Self::from_points(name, &out, route_length)
    }

    /// `rows` - (пикет, m), реперы с измеренной мерой. `equations` -
    /// (ПК назад, ПК вперёд), место уравнения находится интерполяцией
    /// ПК назад по реперам перед ним. Пикет репера отсчитан в системе
    /// участка, где он стоит. Уравнения идут по порядку вдоль трассы.
    pub fn from_ledger(
        name: SystemName,
        rows: &[(f64, f64)],
        equations: &[(f64, f64)],
        route_length: Option<f64>,
    ) -> Result<Self, CoreError> {
        let mut rows = rows.to_vec();
        rows.sort_by(|a, b| a.1.total_cmp(&b.1));
        let mut eqs: std::collections::VecDeque<(f64, f64)> = equations.iter().copied().collect();
        let mut points: Vec<LedgerPoint> = Vec::new();
        for &(st, m) in &rows {
            let p = LedgerPoint::new(m, st, None);
            let mut force_after = false;
            if let (Some(&(back, ahead)), Some(a)) = (eqs.front(), points.last())
                && a.ahead() <= back + EPS
                && ahead < back
                && ahead - EPS <= p.station
                && p.station <= back + EPS
            {
                // пикет репера есть по обе стороны обратной вставки:
                // сторона та, при которой масштаб участка ближе к 1
                let dm = p.m - a.m;
                let k_before = (p.station - a.ahead()) / dm;
                let k_after = (p.station + (back - ahead) - a.ahead()) / dm;
                let d_before = if k_before > 0.0 {
                    (k_before - 1.0).abs()
                } else {
                    f64::INFINITY
                };
                let d_after = (k_after - 1.0).abs();
                if d_before.min(d_after) > SCALE_TOL || (d_before - d_after).abs() < SCALE_TOL {
                    return Err(bad(
                        &name,
                        Msg::new("репер ПК {st:.2f} (m={m:.2f}) лежит в обратной вставке {b}={a}, и его сторона неизвестна. Задайте мерой положение уравнения")
                            .f("st", p.station)
                            .f("m", p.m)
                            .f("b", back)
                            .f("a", ahead),
                    ));
                }
                force_after = d_after < d_before;
            }
            while let (Some(&(back, ahead)), Some(&a)) = (eqs.front(), points.last()) {
                if !((p.station > back + EPS || force_after) && a.ahead() <= back + EPS) {
                    break;
                }
                force_after = false;
                eqs.pop_front();
                // следующий репер отсчитан от ПК вперёд: его значение
                // в системе «назад» даёт m уравнения
                let st_b_equiv = p.station - (ahead - back);
                if st_b_equiv - a.ahead() <= EPS {
                    return Err(bad(
                        &name,
                        Msg::new(
                            "уравнение {b}={a} не помещается между реперами m {m0:.2f} и {m1:.2f}",
                        )
                        .f("b", back)
                        .f("a", ahead)
                        .f("m0", a.m)
                        .f("m1", p.m),
                    ));
                }
                let k = (p.m - a.m) / (st_b_equiv - a.ahead());
                let m_eq = a.m + (back - a.ahead()) * k;
                points.push(LedgerPoint::new(m_eq, back, Some(ahead)));
            }
            points.push(p);
        }
        if !eqs.is_empty() {
            let list: Vec<String> = eqs
                .iter()
                .map(|(b, a)| format!("({}, {})", pyfmt::repr(*b), pyfmt::repr(*a)))
                .collect();
            return Err(bad(
                &name,
                Msg::new("не удалось разместить уравнения {eqs}")
                    .s("eqs", format!("[{}]", list.join(", "))),
            ));
        }
        Self::from_points(name, &points, route_length)
    }

    /// Строки ведомости: (пикет, мера или None, пикет вперёд или None).
    /// Репер - пикет с мерой, уравнение - строка с пикетом вперёд, мера
    /// уравнения берётся, если известна, иначе вычисляется.
    pub fn from_table(
        name: SystemName,
        rows: &[(f64, Option<f64>, Option<f64>)],
        route_length: Option<f64>,
    ) -> Result<Self, CoreError> {
        let mut reps = Vec::new();
        let mut eqs = Vec::new();
        for &(st, m, ahead) in rows {
            match (ahead, m) {
                (Some(a), _) if (a - st).abs() > EPS => eqs.push((st, m, a)),
                (_, Some(m)) => reps.push((st, m)),
                _ => {
                    return Err(bad(
                        &name,
                        Msg::new("у репера ПК {st:.2f} не задана мера").f("st", st),
                    ));
                }
            }
        }
        if eqs.iter().all(|(_, m, _)| m.is_some()) {
            let pts: Vec<LedgerPoint> = reps
                .iter()
                .map(|&(st, m)| LedgerPoint::new(m, st, None))
                .chain(
                    eqs.iter()
                        .map(|&(st, m, a)| LedgerPoint::new(m.unwrap_or(f64::NAN), st, Some(a))),
                )
                .collect();
            return Self::from_points(name, &pts, route_length);
        }
        let e: Vec<(f64, f64)> = eqs.iter().map(|&(st, _, a)| (st, a)).collect();
        Self::from_ledger(name, &reps, &e, route_length)
    }

    pub fn m_start(&self) -> f64 {
        self.sections[0].m_from
    }

    pub fn m_end(&self) -> f64 {
        self.sections[self.sections.len() - 1].m_to
    }

    pub fn is_monotonic(&self) -> bool {
        self.sections
            .windows(2)
            .all(|w| w[1].st_from >= w[0].st_to - EPS)
    }

    /// (m, ПК назад, ПК вперёд) для всех скачков пикетажа.
    pub fn equations(&self) -> Vec<(f64, f64, f64)> {
        self.sections
            .windows(2)
            .filter(|w| (w[1].st_from - w[0].st_to).abs() > EPS)
            .map(|w| (w[1].m_from, w[0].st_to, w[1].st_from))
            .collect()
    }

    /// Целые пикеты, кратные `step`, по всем участкам: (m, пикет, участок).
    /// В обратной вставке одно значение даёт две точки, в прямой - ни одной.
    pub fn whole_stations(&self, step: f64) -> Vec<(f64, f64, usize)> {
        let mut out = Vec::new();
        let last = self.sections.len() - 1;
        for s in &self.sections {
            let k0 = -pyfmt::floor_div(-(s.st_from - EPS), step);
            let k1 = pyfmt::floor_div(s.st_to + EPS, step);
            let (k0, k1) = (k0 as i64, k1 as i64);
            for k in k0..=k1 {
                let st = k as f64 * step;
                if s.index != last && (st - s.st_to).abs() <= EPS {
                    let nxt = &self.sections[s.index + 1];
                    if (nxt.st_from - s.st_to).abs() <= EPS {
                        continue; // стык без уравнения: точку даст следующий участок
                    }
                }
                out.push((s.measure_at(st), st, s.index));
            }
        }
        out
    }

    fn out_of_range(&self, m: f64) -> CoreError {
        CoreError::new(
            ErrorCode::MeasureOutOfRange,
            Msg::new("мера {m:.3f} вне {a:.3f}..{b:.3f}")
                .f("m", m)
                .f("a", self.m_start())
                .f("b", self.m_end()),
        )
    }

    fn starts(&self) -> Vec<f64> {
        self.sections.iter().map(|s| s.m_from).collect()
    }

    pub fn to_station(&self, m: f64) -> Result<f64, CoreError> {
        if m < self.m_start() - EPS || m > self.m_end() + EPS {
            return Err(self.out_of_range(m));
        }
        let i = bisect_right(&self.starts(), m + EPS).saturating_sub(1);
        Ok(self.sections[i].station_at(m))
    }

    /// Пикет в точке m со стороны «назад»: на уравнении это ПК назад,
    /// в остальных точках то же, что [`Self::to_station`].
    pub fn to_station_back(&self, m: f64) -> Result<f64, CoreError> {
        if m < self.m_start() - EPS || m > self.m_end() + EPS {
            return self.to_station(m);
        }
        let i = bisect_left(&self.starts(), m - EPS).saturating_sub(1);
        Ok(self.sections[i].station_at(m))
    }

    /// M для вершин линии с мерами `ms` по возрастанию. Внутрь линии
    /// вставляются вершины уравнений, по две на уравнение в одной точке
    /// (ПК назад и ПК вперёд). Ответ - новые вершины, меры и пикеты.
    pub fn stations_along(&self, coords: &[P3], ms: &[f64]) -> (Vec<P3>, Vec<f64>, Vec<f64>) {
        let mut c = coords.to_vec();
        let mut m = ms.to_vec();
        if m.len() < 2 {
            let st = m
                .iter()
                .map(|v| self.to_station(*v).unwrap_or(f64::NAN))
                .collect();
            return (c, m, st);
        }
        for (me, _, _) in self.equations() {
            if !(m[0] + EPS < me && me < m[m.len() - 1] - EPS) {
                continue;
            }
            let k = bisect_left(&m, me);
            let p = if (m[k] - me).abs() <= EPS {
                c[k]
            } else {
                let t = (me - m[k - 1]) / (m[k] - m[k - 1]);
                let (a, b) = (c[k - 1], c[k]);
                let p = [
                    a[0] + (b[0] - a[0]) * t,
                    a[1] + (b[1] - a[1]) * t,
                    a[2] + (b[2] - a[2]) * t,
                ];
                c.insert(k, p);
                m.insert(k, me);
                p
            };
            c.insert(k, p);
            m.insert(k, me);
        }
        let n = m.len();
        let st = (0..n)
            .map(|i| {
                let v = m[i];
                let back_side = i == n - 1 || (i + 1 < n && (m[i + 1] - v).abs() <= EPS);
                let r = if back_side {
                    self.to_station_back(v)
                } else {
                    self.to_station(v)
                };
                r.unwrap_or(f64::NAN)
            })
            .collect();
        (c, m, st)
    }

    /// Пикет - в m. `section` - номер участка с 0, нужен только когда пикет
    /// попадает в обратную вставку.
    pub fn to_measure(&self, station: f64, section: Option<i64>) -> Result<f64, CoreError> {
        let out = |msg: Msg| CoreError::new(ErrorCode::StationOutOfRange, msg);
        if let Some(n) = section {
            let Some(s) = usize::try_from(n).ok().and_then(|i| self.sections.get(i)) else {
                return Err(out(Msg::new("нет участка {n}").i("n", n)));
            };
            if !s.contains_station(station) {
                return Err(out(Msg::new(
                    "ПК {st:.2f} вне участка {n} ({a:.2f}..{b:.2f})",
                )
                .f("st", station)
                .i("n", n)
                .f("a", s.st_from)
                .f("b", s.st_to)));
            }
            return Ok(s.measure_at(station));
        }
        let mut hits: Vec<f64> = Vec::new();
        for s in &self.sections {
            if s.contains_station(station) {
                let m = s.measure_at(station);
                if hits.last().is_none_or(|h| (h - m).abs() > 1e-3) {
                    hits.push(m);
                }
            }
        }
        if hits.len() == 1 {
            return Ok(hits[0]);
        }
        if hits.len() > 1 {
            return Err(CoreError::new(
                ErrorCode::StationAmbiguous,
                Msg::new("ПК {st:.2f} встречается {n} раза (обратная вставка), укажите участок")
                    .f("st", station)
                    .i("n", hits.len() as i64),
            ));
        }
        for w in self.sections.windows(2) {
            if w[0].st_to < station && station < w[1].st_from {
                return Err(CoreError::new(
                    ErrorCode::StationInGap,
                    Msg::new("ПК {st:.2f} попал в прямую вставку {a:.2f}={b:.2f}")
                        .f("st", station)
                        .f("a", w[0].st_to)
                        .f("b", w[1].st_from),
                ));
            }
        }
        Err(out(
            Msg::new("ПК {st:.2f} вне пикетажа маршрута").f("st", station)
        ))
    }
}

/// Пересчёт пикета из одной системы в другую через меру m.
pub fn convert(
    station: f64,
    source: &ChainageSystem,
    target: &ChainageSystem,
    section: Option<i64>,
) -> Result<f64, CoreError> {
    target.to_station(source.to_measure(station, section)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name() -> SystemName {
        SystemName::Text("исполнительная".into())
    }

    /// Ведомость примера 1.01 у R3: невязка, обратная вставка 25 м
    /// и прямая вставка 40 м, меры известны.
    fn demo_r3() -> ChainageSystem {
        ChainageSystem::from_table(
            name(),
            &[
                (0.0, Some(0.0), None),
                (399.6, Some(400.0), None),
                (699.3, Some(700.0), Some(674.3)),
                (974.3, Some(1000.0), None),
                (1274.3, Some(1300.0), Some(1314.3)),
                (1614.3, Some(1600.0), None),
            ],
            Some(1600.0),
        )
        .unwrap()
    }

    #[test]
    fn ledger_with_equations() {
        let s = demo_r3();
        assert_eq!(s.sections.len(), 5);
        assert_eq!(s.equations().len(), 2);
        assert!(!s.is_monotonic());
        assert!((s.to_station(1600.0).unwrap() - 1614.3).abs() < 1e-9);
        // пикет в обратной вставке - два места, с участком - одно
        let e = s.to_measure(690.0, None).unwrap_err();
        assert_eq!(e.code, ErrorCode::StationAmbiguous);
        assert!((s.to_measure(690.0, Some(2)).unwrap() - 715.7).abs() < 1e-9);
        // пикет в прямой вставке
        assert_eq!(
            s.to_measure(1290.0, None).unwrap_err().code,
            ErrorCode::StationInGap
        );
        // на уравнении назад - ПК назад, вперёд - ПК вперёд
        assert!((s.to_station_back(700.0).unwrap() - 699.3).abs() < 1e-9);
        assert!((s.to_station(700.0).unwrap() - 674.3).abs() < 1e-9);
        // целые пикеты: ПК 13 в прямой вставке пропущен, по 3-4 на участок
        let ws = s.whole_stations(100.0);
        assert_eq!(ws.len(), 16);
        assert!(!ws.iter().any(|w| (w.1 - 1300.0).abs() < 1e-9));
    }

    #[test]
    fn ledger_without_equation_measures() {
        // те же реперы, меры уравнений не заданы: место ищется по реперам
        let s = ChainageSystem::from_table(
            name(),
            &[
                (0.0, Some(0.0), None),
                (399.6, Some(400.0), None),
                (699.3, None, Some(674.3)),
                (974.3, Some(1000.0), None),
                (1274.3, None, Some(1314.3)),
                (1614.3, Some(1600.0), None),
            ],
            Some(1600.0),
        )
        .unwrap();
        // числа - прогон того же ядра модуля на Python
        let eq = s.equations();
        assert_eq!(
            eq,
            vec![(699.8499249624812, 699.3, 674.3), (1300.0, 1274.3, 1314.3)]
        );
    }

    #[test]
    fn from_measures_merges_straight_vertices() {
        let pairs = [
            (0.0, 0.0),
            (50.0, 50.0),
            (100.0, 100.0),
            (100.0, 90.0),
            (200.0, 190.0),
        ];
        let s = ChainageSystem::from_measures(
            SystemName::Word("M геометрии"),
            &pairs,
            Some(200.0),
            1.0,
        )
        .unwrap();
        assert_eq!(s.sections.len(), 2);
        assert_eq!(s.equations(), vec![(100.0, 100.0, 90.0)]);
        let e = ChainageSystem::from_measures(
            SystemName::Word("M геометрии"),
            &[(0.0, 1.0)],
            None,
            1.0,
        )
        .unwrap_err();
        assert_eq!(
            e.msg.render(&|t| t.to_string()),
            "система «M геометрии»: у маршрута меньше двух вершин с M"
        );
    }

    #[test]
    fn stations_along_doubles_equation_vertices() {
        let s = demo_r3();
        let c = vec![[0.0, 0.0, f64::NAN], [1600.0, 0.0, f64::NAN]];
        let (c2, m2, st) = s.stations_along(&c, &[0.0, 1600.0]);
        assert_eq!(c2.len(), 6);
        assert_eq!(m2.len(), 6);
        assert!((st[1] - 699.3).abs() < 1e-9 && (st[2] - 674.3).abs() < 1e-9);
        assert!((st[5] - 1614.3).abs() < 1e-9);
    }
}
