//! Sumber acak kecil untuk pengacakan Fisher-Yates di engine.
//!
//! v2 memakai `Math.random` (atau fungsi `rng` yang disuntikkan pada tes).
//! Core memakai trait `Rng` agar tes bisa memakai sumber deterministik,
//! sesuai SDD Bagian 10 ("pengacakan menerima sumber acak yang bisa disemai").
//! Tidak ada dependensi crate `rand`: xorshift64* sudah cukup untuk
//! mengacak opsi jawaban dan deck.

/// Sumber bilangan acak [0, 1).
pub trait Rng {
    /// Bilangan bulat 64-bit acak.
    fn next_u64(&mut self) -> u64;

    /// Bilangan real acak pada [0, 1).
    fn next_f64(&mut self) -> f64 {
        // 53 bit presisi, sama seperti Math.random.
        ((self.next_u64() >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

/// Xorshift64* — cepat, cukup untuk pengacakan UI/game (bukan kriptografi).
pub struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x9E3779B97F4A7C15 } else { seed },
        }
    }
}

impl Rng for XorShift64 {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
}

/// RNG deterministik yang selalu mengembalikan nilai yang sama — cerminan
/// fungsi `() => 0.1` yang dipakai tes v2 untuk pengacakan.
pub struct ConstRng(pub f64);

impl Rng for ConstRng {
    fn next_u64(&mut self) -> u64 {
        ((self.0.clamp(0.0, 0.999_999_999_999_999_9)) * ((1u64 << 53) as f64)) as u64
    }

    fn next_f64(&mut self) -> f64 {
        self.0
    }
}

/// Pengacakan Fisher-Yates di tempat, sama seperti loop v2
/// (`for i = len-1; i > 0; i-- { j = floor(rng() * (i+1)); swap }`).
pub fn shuffle<T>(items: &mut [T], rng: &mut dyn Rng) {
    let len = items.len();
    if len <= 1 {
        return;
    }
    for i in (1..len).rev() {
        let j = (rng.next_f64() * ((i + 1) as f64)).floor() as usize;
        let j = j.min(i);
        items.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shuffle_with_const_rng_matches_js_math_random_stub() {
        // rng () => 0.1 pada 3 elemen: i=2 -> j=floor(0.1*3)=0 (swap 2<->0);
        // i=1 -> j=floor(0.1*2)=0 (swap 1<->0). Hasil: [B, C, A].
        let mut v = vec!['A', 'B', 'C'];
        shuffle(&mut v, &mut ConstRng(0.1));
        assert_eq!(v, vec!['B', 'C', 'A']);
    }

    #[test]
    fn xorshift_produces_values_in_unit_interval() {
        let mut rng = XorShift64::new(42);
        for _ in 0..100 {
            let v = rng.next_f64();
            assert!(v >= 0.0 && v < 1.0);
        }
    }
}
