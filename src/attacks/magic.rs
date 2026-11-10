//! Magic bitboard lookup for sliders. Tables are built from the ray walk in [`super::rays`].
//!
//! All public attack functions check the corresponding magic bitboard table.
//! All tables are private, lazy statics that are initialized at run-time on their first call.
//! This is done as building them with `const` would be annoying (and slow down compilation).
//! Also adds bitboard method [`Bitboard::magic_index`].

use super::rays::{BISHOP_STEPS, ROOK_STEPS, Step, slider_rays};
use crate::rng::Rng;
use crate::types::{Bitboard, Square};
use std::sync::LazyLock;

/// Rook attacks from `square`. `blockers` is the full occupancy; the first blocker of either
/// color in each direction is included.
#[inline]
pub fn rook_attacks(square: Square, blockers: Bitboard) -> Bitboard {
    ROOK_TABLE.attacks(square, blockers)
}

/// Bishop attacks from `square`. `blockers` is the full occupancy; the first blocker of either
/// color in each direction is included.
#[inline]
pub fn bishop_attacks(square: Square, blockers: Bitboard) -> Bitboard {
    BISHOP_TABLE.attacks(square, blockers)
}

/// Queen attacks: rook attacks plus bishop attacks.
#[inline]
pub fn queen_attacks(square: Square, blockers: Bitboard) -> Bitboard {
    rook_attacks(square, blockers) | bishop_attacks(square, blockers)
}

/// The squares a slider passes through in one direction, stopping before the last on-board square.
fn relevant_ray(start: Bitboard, step: Step) -> Bitboard {
    let mut current = step(start);
    let mut mask = Bitboard::EMPTY;

    while !step(current).is_empty() {
        mask |= current;
        current = step(current);
    }

    mask
}

/// The squares a slider passes through in all `Step` directions, excludes the starting square.
fn relevant_mask(square: Square, steps: &[Step]) -> Bitboard {
    let start = square.bitboard();
    steps.iter().fold(Bitboard::EMPTY, |acc, &step| {
        acc | relevant_ray(start, step)
    })
}

impl Bitboard {
    /// `(self * magic) >> shift` with a wrapping multiply: the index of this blocker pattern
    /// in a magic attack table. `self` must already be masked to the relevant squares.
    #[inline]
    pub const fn magic_index(self, magic: u64, shift: u32) -> usize {
        (self.0.wrapping_mul(magic) >> shift) as usize
    }
}

fn sparse_u64(rng: &mut Rng) -> u64 {
    rng.next_u64() & rng.next_u64() & rng.next_u64()
}

/// One square's entry in a magic table.
#[derive(Clone, Copy, Debug)]
struct Magic {
    mask: Bitboard,
    magic: u64,
    shift: u32,
    offset: usize,
}

impl Magic {
    #[inline]
    fn index(self, blockers: Bitboard) -> usize {
        self.offset + (blockers & self.mask).magic_index(self.magic, self.shift)
    }
}

/// Searches for a magic for `square` and returns it (with offset 0) together with the square's
/// attack table, indexed by `magic_index`.
fn find_magic(square: Square, steps: &[Step], rng: &mut Rng) -> (Magic, Vec<Bitboard>) {
    let mask = relevant_mask(square, steps);
    let bits = u32::from(mask.count());
    let shift = 64 - bits;

    // every blocker pattern on the mask, with the attacks it produces
    let patterns: Vec<(Bitboard, Bitboard)> = mask
        .subsets()
        .map(|blockers| (blockers, slider_rays(square, blockers, steps)))
        .collect();
    let mut table: Vec<Option<Bitboard>> = vec![None; 1 << bits];
    let mut epoch_table: Vec<u32> = vec![0; 1 << bits];
    let mut epoch: u32 = 0;

    loop {
        epoch += 1;

        let magic = sparse_u64(rng);

        // cheap rejection: a useful magic moves several mask bits into the top byte
        if mask.magic_index(magic, 56).count_ones() < 6 {
            continue;
        }

        let fits = patterns.iter().all(|&(blockers, attacks)| {
            let index = blockers.magic_index(magic, shift);
            let slot = &mut table[index];

            if epoch_table[index] == epoch {
                *slot == Some(attacks)
            } else {
                *slot = Some(attacks);
                epoch_table[index] = epoch;
                true
            }
        });

        if fits {
            let magic = Magic {
                mask,
                magic,
                shift,
                offset: 0,
            };
            // slots left empty by constructive collisions are never looked up
            let attacks = table.into_iter().map(Option::unwrap_or_default).collect();
            return (magic, attacks);
        }
    }
}

/// Magics for every square and one packed attack table shared by all 64 squares.
struct MagicTable {
    magics: [Magic; Square::COUNT],
    attacks: Box<[Bitboard]>,
}

impl MagicTable {
    fn build(steps: &[Step], seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let mut attacks = Vec::new();

        // `from_fn` calls the closure in index order, so offsets grow with the square index
        let magics = std::array::from_fn(|index| {
            let square = Square::from_index(index).expect("square index valid in range 0..64");
            let (magic, table) = find_magic(square, steps, &mut rng);
            let magic = Magic {
                offset: attacks.len(),
                ..magic
            };
            attacks.extend(table);
            magic
        });

        Self {
            magics,
            attacks: attacks.into_boxed_slice(),
        }
    }

    #[inline]
    fn attacks(&self, square: Square, blockers: Bitboard) -> Bitboard {
        self.attacks[self.magics[square.index()].index(blockers)]
    }
}

const ROOK_SEED: u64 = 0x2545_F491_4F6C_DD1D;
const BISHOP_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

static ROOK_TABLE: LazyLock<MagicTable> =
    LazyLock::new(|| MagicTable::build(&ROOK_STEPS, ROOK_SEED));
static BISHOP_TABLE: LazyLock<MagicTable> =
    LazyLock::new(|| MagicTable::build(&BISHOP_STEPS, BISHOP_SEED));

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::sq;

    /// A public lookup under test.
    type Lookup = fn(Square, Bitboard) -> Bitboard;
    /// (name, steps, public lookup) for each slider.
    type Piece<'a> = (&'a str, &'a [Step], Lookup);

    const PIECES: [Piece; 2] = [
        ("rook", &ROOK_STEPS, rook_attacks),
        ("bishop", &BISHOP_STEPS, bishop_attacks),
    ];

    const TEST_SEED: u64 = 0x0123_4567_89AB_CDEF;

    fn all_squares() -> impl Iterator<Item = Square> {
        Bitboard::FULL.into_iter()
    }

    fn squares(names: &[&str]) -> Bitboard {
        names.iter().map(|&name| sq(name)).collect()
    }

    #[test]
    fn relevant_masks_exact() {
        let cases: &[(&str, &[Step], &str, &[&str])] = &[
            (
                "rook",
                &ROOK_STEPS,
                "a1",
                &[
                    "a2", "a3", "a4", "a5", "a6", "a7", "b1", "c1", "d1", "e1", "f1", "g1",
                ],
            ),
            (
                "rook",
                &ROOK_STEPS,
                "d4",
                &["d2", "d3", "d5", "d6", "d7", "b4", "c4", "e4", "f4", "g4"],
            ),
            (
                "bishop",
                &BISHOP_STEPS,
                "d4",
                &["e5", "f6", "g7", "c5", "b6", "e3", "f2", "c3", "b2"],
            ),
            (
                "bishop",
                &BISHOP_STEPS,
                "a1",
                &["b2", "c3", "d4", "e5", "f6", "g7"],
            ),
        ];
        for &(name, steps, square, names) in cases {
            let expected = squares(names);
            let actual = relevant_mask(sq(square), steps);
            assert_eq!(
                expected, actual,
                "{name} on {square}\nexpected:\n{expected}\nactual:\n{actual}"
            );
        }
    }

    #[test]
    fn rook_mask_count_follows_from_coordinates() {
        for square in all_squares() {
            let along_rank = if matches!(square.file(), 0 | 7) { 6 } else { 5 };
            let along_file = if matches!(square.rank(), 0 | 7) { 6 } else { 5 };
            let expected = along_rank + along_file;
            let actual = relevant_mask(square, &ROOK_STEPS).count();
            assert_eq!(expected, actual, "rook mask count on {square}");
        }
    }

    #[test]
    fn relevant_masks_are_inside_empty_board_attacks() {
        for (name, steps, _) in PIECES {
            for square in all_squares() {
                let mask = relevant_mask(square, steps);
                let empty_board = slider_rays(square, Bitboard::EMPTY, steps);
                assert!(
                    !mask.contains(square),
                    "{name} mask on {square} holds its own square\n{mask}"
                );
                assert_eq!(
                    Bitboard::EMPTY,
                    mask & !empty_board,
                    "{name} mask on {square} outside its attacks\nmask:\n{mask}"
                );
            }
        }
    }

    #[test]
    fn relevant_mask_table_sizes() {
        let cases = [
            ("rook", &ROOK_STEPS, 102_400),
            ("bishop", &BISHOP_STEPS, 5_248),
        ];
        for (name, steps, expected) in cases {
            let actual: usize = all_squares()
                .map(|square| 1usize << relevant_mask(square, steps).count())
                .sum();
            assert_eq!(expected, actual, "{name} table size");
        }
    }

    #[test]
    fn built_tables_have_packed_sizes() {
        assert_eq!(102_400, ROOK_TABLE.attacks.len(), "rook table");
        assert_eq!(5_248, BISHOP_TABLE.attacks.len(), "bishop table");
    }

    #[test]
    fn masked_blockers_give_same_attacks() {
        let mut rng = Rng::new(TEST_SEED);
        for (name, steps, _) in PIECES {
            for square in all_squares() {
                let mask = relevant_mask(square, steps);
                let random = (0..200).map(|_| Bitboard(rng.next_u64()));
                for blockers in [Bitboard::EMPTY, Bitboard::FULL].into_iter().chain(random) {
                    let expected = slider_rays(square, blockers, steps);
                    let actual = slider_rays(square, blockers & mask, steps);
                    assert_eq!(
                        expected, actual,
                        "{name} on {square}\nblockers:\n{blockers}\nexpected:\n{expected}\nactual:\n{actual}"
                    );
                }
            }
        }
    }

    #[test]
    fn sparse_rng_bits_are_sparse() {
        let mut rng = Rng::new(TEST_SEED);
        let sparse: u32 = (0..1000).map(|_| sparse_u64(&mut rng).count_ones()).sum();
        assert!(
            (6_000..10_000).contains(&sparse),
            "sparse_u64 total bits {sparse}"
        );
    }

    #[test]
    fn find_magic_table_matches_rays() {
        let mut rng = Rng::new(TEST_SEED);
        for (name, steps, _) in PIECES {
            for square in [sq("a1"), sq("d4")] {
                let (magic, table) = find_magic(square, steps, &mut rng);
                assert_eq!(0, magic.offset, "{name} on {square}: offset");
                for blockers in magic.mask.subsets() {
                    let expected = slider_rays(square, blockers, steps);
                    let actual = table[magic.index(blockers)];
                    assert_eq!(
                        expected, actual,
                        "{name} on {square}\nblockers:\n{blockers}\nexpected:\n{expected}\nactual:\n{actual}"
                    );
                }
            }
        }
    }

    #[test]
    fn lookups_match_rays_for_every_mask_subset() {
        for (name, steps, lookup) in PIECES {
            for square in all_squares() {
                for blockers in relevant_mask(square, steps).subsets() {
                    let expected = slider_rays(square, blockers, steps);
                    let actual = lookup(square, blockers);
                    assert_eq!(
                        expected, actual,
                        "{name} on {square}\nblockers:\n{blockers}\nexpected:\n{expected}\nactual:\n{actual}"
                    );
                }
            }
        }
    }

    #[test]
    fn lookups_match_rays_for_random_blockers() {
        // Full-board blockers, including bits outside the mask that the lookup must ignore.
        let mut rng = Rng::new(TEST_SEED);
        for (name, steps, lookup) in PIECES {
            for square in all_squares() {
                for _ in 0..500 {
                    let blockers = Bitboard(rng.next_u64());
                    let expected = slider_rays(square, blockers, steps);
                    let actual = lookup(square, blockers);
                    assert_eq!(
                        expected, actual,
                        "{name} on {square}\nblockers:\n{blockers}\nexpected:\n{expected}\nactual:\n{actual}"
                    );
                }
            }
        }
    }
}
