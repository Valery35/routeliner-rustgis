// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 3.0 или новее.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Случайные числа как у `random.Random` Python: вихрь Мерсенна MT19937,
//! зерно-целое через `init_by_array`, `random`, `uniform`, `choice`.
//! Пример 1.01 строится по зерну, и тот же номер зерна даёт те же записи,
//! что у модуля.

const N: usize = 624;
const M: usize = 397;

pub struct PyRandom {
    mt: [u32; N],
    idx: usize,
}

impl PyRandom {
    /// `random.Random(seed)` для целого зерна: слова по 32 бита модуля
    /// числа, младшее первым, нуль - одно слово 0.
    pub fn new(seed: i64) -> Self {
        let mut n = seed.unsigned_abs();
        let mut key = Vec::new();
        while n > 0 {
            key.push((n & 0xffff_ffff) as u32);
            n >>= 32;
        }
        if key.is_empty() {
            key.push(0);
        }
        let mut r = Self {
            mt: [0; N],
            idx: N + 1,
        };
        r.init_by_array(&key);
        r
    }

    fn init_genrand(&mut self, s: u32) {
        self.mt[0] = s;
        for i in 1..N {
            let prev = self.mt[i - 1];
            self.mt[i] = 1_812_433_253u32
                .wrapping_mul(prev ^ (prev >> 30))
                .wrapping_add(i as u32);
        }
        self.idx = N;
    }

    fn init_by_array(&mut self, key: &[u32]) {
        self.init_genrand(19_650_218);
        let (mut i, mut j) = (1usize, 0usize);
        for _ in 0..N.max(key.len()) {
            let prev = self.mt[i - 1];
            self.mt[i] = (self.mt[i] ^ (prev ^ (prev >> 30)).wrapping_mul(1_664_525))
                .wrapping_add(key[j])
                .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= N {
                self.mt[0] = self.mt[N - 1];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
        }
        for _ in 0..N - 1 {
            let prev = self.mt[i - 1];
            self.mt[i] = (self.mt[i] ^ (prev ^ (prev >> 30)).wrapping_mul(1_566_083_941))
                .wrapping_sub(i as u32);
            i += 1;
            if i >= N {
                self.mt[0] = self.mt[N - 1];
                i = 1;
            }
        }
        self.mt[0] = 0x8000_0000;
    }

    fn next_u32(&mut self) -> u32 {
        if self.idx >= N {
            for k in 0..N {
                let y = (self.mt[k] & 0x8000_0000) | (self.mt[(k + 1) % N] & 0x7fff_ffff);
                let mag = if y & 1 == 1 { 0x9908_b0df } else { 0 };
                self.mt[k] = self.mt[(k + M) % N] ^ (y >> 1) ^ mag;
            }
            self.idx = 0;
        }
        let mut y = self.mt[self.idx];
        self.idx += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    /// `random()`: 53 бита из двух слов.
    pub fn random(&mut self) -> f64 {
        let a = f64::from(self.next_u32() >> 5);
        let b = f64::from(self.next_u32() >> 6);
        (a * 67_108_864.0 + b) * (1.0 / 9_007_199_254_740_992.0)
    }

    pub fn uniform(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.random()
    }

    /// `getrandbits(k)` для k до 32.
    fn bits(&mut self, k: u32) -> u32 {
        if k == 0 {
            0
        } else {
            self.next_u32() >> (32 - k)
        }
    }

    /// `_randbelow_with_getrandbits(n)`.
    pub fn below(&mut self, n: usize) -> usize {
        let k = usize::BITS - n.leading_zeros();
        loop {
            let r = self.bits(k) as usize;
            if r < n {
                return r;
            }
        }
    }

    pub fn choice<T: Copy>(&mut self, seq: &[T]) -> T {
        seq[self.below(seq.len())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Значения Python 3.12: `r = random.Random(1)`, затем `r.random()`
    /// дважды, `r.uniform(-15, 15)`, `r.choice` из трёх и из пяти.
    /// Отрицательное зерно берётся по модулю, длинное - словами по 32 бита.
    #[test]
    fn matches_python() {
        let mut r = PyRandom::new(1);
        assert_eq!(r.random(), 0.13436424411240122);
        assert_eq!(r.random(), 0.8474337369372327);
        assert_eq!(r.uniform(-15.0, 15.0), 7.9132385692984215);
        assert_eq!(r.choice(&[0, 1, 2]), 1);
        assert_eq!(r.choice(&[0, 1, 2, 3, 4]), 0);
        assert_eq!(PyRandom::new(-7).random(), 0.32383276483316237);
        assert_eq!(PyRandom::new((1 << 40) + 5).random(), 0.5043802970418443);
    }
}
