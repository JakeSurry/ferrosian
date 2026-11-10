//! xorshift64 pseudo-random generator. Deterministic for a given seed. Not cryptographic.

/// xorshift64 pseudo-random generator. Deterministic for a given seed. Not cryptographic.
#[derive(Debug)]
pub struct Rng(u64);

impl Rng {
    /// Generate a new Rng from a given seed.
    ///
    /// # Panics
    ///
    /// Panics if `seed` is `0`, as zero is xorshift's stuck state.
    pub const fn new(seed: u64) -> Self {
        assert!(
            0 != seed,
            "cannot seed with 0; zero is xorshift's stuck state"
        );
        Self(seed)
    }

    /// Generate next pseudo-random number.
    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// Return a pseudo-random number in `0..n`.
    ///
    /// # Panics
    ///
    /// Panics if `n` is `0`, as there are no values in range `0..0`.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(
            0 != n,
            "cannot call `below` with 0; no value in range `0..0`"
        );
        ((u128::from(self.next_u64()) * n as u128) >> 64) as usize
    }

    /// Returns a pseudo-random item from `items`.
    /// Returns `None` if `items` is empty.
    pub fn choose<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        match items.len() {
            0 => None,
            n => Some(&items[self.below(n)]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::TEST_SEED;

    #[test]
    fn is_deterministic_and_never_zero() {
        let mut first = Rng::new(TEST_SEED);
        let mut second = Rng::new(TEST_SEED);
        for draw in 0..100 {
            let value = first.next_u64();
            assert_eq!(value, second.next_u64(), "draw {draw}");
            assert_ne!(0, value, "draw {draw}");
        }
    }

    #[test]
    fn bits_look_random() {
        let mut rng = Rng::new(TEST_SEED);
        let dense: u32 = (0..1000).map(|_| rng.next_u64().count_ones()).sum();
        assert!(
            (28_000..36_000).contains(&dense),
            "next_u64 total bits {dense}"
        );
    }

    #[test]
    fn below_stays_below_n() {
        let cases = [1, 2, 3, 218];
        let mut rng = Rng::new(TEST_SEED);
        for n in cases {
            for draw in 0..100 {
                assert!(n > rng.below(n), "n {n}, draw {draw}");
            }
        }
    }

    #[test]
    fn below_hits_every_value_to_n() {
        const N: usize = 4;
        let mut rng = Rng::new(TEST_SEED);
        let mut hit = [false; N];
        for _ in 0..100 {
            hit[rng.below(N)] = true;
        }
        assert!(hit.iter().all(|&h| h), "{hit:?}")
    }

    #[test]
    fn choose_returns_an_item() {
        let mut rng = Rng::new(TEST_SEED);
        let cases: [(&str, &[u8]); 3] = [
            ("empty", &[]),
            ("one element", &[1]),
            ("many elements", &[1, 2, 3, 4, 5]),
        ];
        for (why, items) in cases {
            for draw in 0..100 {
                match rng.choose(items) {
                    None => assert!(items.is_empty(), "{why}, draw {draw}"),
                    Some(e) => assert!(items.contains(e), "{why}, draw {draw}"),
                }
            }
        }
    }

    #[test]
    #[should_panic(expected = "cannot seed with 0; zero is xorshift's stuck state")]
    fn seed_zero_panics() {
        Rng::new(0);
    }

    #[test]
    #[should_panic(expected = "cannot call `below` with 0; no value in range `0..0`")]
    fn below_zero_panics() {
        let mut rng = Rng::new(TEST_SEED);
        rng.below(0);
    }
}
