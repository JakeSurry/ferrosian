//! xorshift64 pseudo-random generator. Deterministic for a given seed.
pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Self {
        debug_assert!(
            0 != seed,
            "cannot seed with 0; zero is xorshift's stuck state"
        );
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SEED: u64 = 0x0123_4567_89AB_CDEF;

    #[test]
    fn rng_is_deterministic_and_never_zero() {
        let mut first = Rng::new(TEST_SEED);
        let mut second = Rng::new(TEST_SEED);
        for draw in 0..100 {
            let value = first.next_u64();
            assert_eq!(value, second.next_u64(), "draw {draw}");
            assert_ne!(0, value, "draw {draw}");
        }
    }

    #[test]
    fn rng_bits_look_random() {
        let mut rng = Rng::new(TEST_SEED);
        let dense: u32 = (0..1000).map(|_| rng.next_u64().count_ones()).sum();
        assert!(
            (28_000..36_000).contains(&dense),
            "next_u64 total bits {dense}"
        );
    }
}
