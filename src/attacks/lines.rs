//! Line geometry between two squares: `BETWEEN` and `LINE`, filled at compile time.

use crate::types::{Bitboard, Square};

/// Squares strictly between `a` and `b` if they share a rank, file or diagonal; otherwise empty.
#[inline]
pub fn between(a: Square, b: Square) -> Bitboard {
    TABLES.0[a.index()][b.index()]
}

/// The whole rank, file or diagonal through `a` and `b` (including both); empty if not aligned.
#[inline]
pub fn line(a: Square, b: Square) -> Bitboard {
    TABLES.1[a.index()][b.index()]
}

const DIRECTIONS: usize = 8;

const fn step(bb: Bitboard, dir: usize) -> Bitboard {
    match dir {
        0 => bb.north(),
        1 => bb.south(),
        2 => bb.east(),
        3 => bb.west(),
        4 => bb.north_east(),
        5 => bb.south_west(),
        6 => bb.north_west(),
        7 => bb.south_east(),
        _ => panic!("direction out of range 0..8"),
    }
}

const fn ray(from: Bitboard, dir: usize) -> Bitboard {
    let mut ray = Bitboard::EMPTY;
    let mut next = step(from, dir);
    while !next.is_empty() {
        ray = ray.union(next);
        next = step(next, dir);
    }
    ray
}

type PairTable = [[Bitboard; Square::COUNT]; Square::COUNT];

const fn build_tables() -> (PairTable, PairTable) {
    let mut between = [[Bitboard::EMPTY; Square::COUNT]; Square::COUNT];
    let mut line = [[Bitboard::EMPTY; Square::COUNT]; Square::COUNT];

    let mut a = 0;
    while a < Square::COUNT {
        let from = Square::from_index(a)
            .expect("square index valid in range 0..64")
            .bitboard();

        let mut dir = 0;
        while dir < DIRECTIONS {
            let full_line = ray(from, dir).union(ray(from, dir ^ 1)).union(from);

            let mut passed = Bitboard::EMPTY;
            let mut next = step(from, dir);
            while let Some(b) = next.lsb() {
                between[a][b.index()] = passed;
                line[a][b.index()] = full_line;
                passed = passed.union(next);
                next = step(next, dir);
            }
            dir += 1;
        }
        a += 1;
    }
    (between, line)
}

static TABLES: (PairTable, PairTable) = build_tables();

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::sq;

    type Cases<'a> = &'a [(&'a str, Square, Square, &'a [Square])];

    #[test]
    fn betweens() {
        let cases: Cases = &[
            (
                "long diagonal",
                sq("a1"),
                sq("h8"),
                &[sq("b2"), sq("c3"), sq("d4"), sq("e5"), sq("f6"), sq("g7")],
            ),
            (
                "e file",
                sq("e1"),
                sq("e8"),
                &[sq("e2"), sq("e3"), sq("e4"), sq("e5"), sq("e6"), sq("e7")],
            ),
            (
                "1 rank",
                sq("a1"),
                sq("a8"),
                &[sq("a2"), sq("a3"), sq("a4"), sq("a5"), sq("a6"), sq("a7")],
            ),
            ("knight jump", sq("a1"), sq("b3"), &[]),
            ("adjacent", sq("b2"), sq("b3"), &[]),
        ];

        for &(name, a, b, expected) in cases {
            let actual = between(a, b).into_iter().collect::<Vec<Square>>();
            assert_eq!(expected, actual, "{name}");
        }
    }

    #[test]
    fn lines() {
        let cases: Cases = &[
            (
                "e file from extremes",
                sq("e1"),
                sq("e8"),
                &[
                    sq("e1"),
                    sq("e2"),
                    sq("e3"),
                    sq("e4"),
                    sq("e5"),
                    sq("e6"),
                    sq("e7"),
                    sq("e8"),
                ],
            ),
            (
                "1 rank from adjacent",
                sq("e1"),
                sq("d1"),
                &[
                    sq("a1"),
                    sq("b1"),
                    sq("c1"),
                    sq("d1"),
                    sq("e1"),
                    sq("f1"),
                    sq("g1"),
                    sq("h1"),
                ],
            ),
            (
                "long diagonal",
                sq("c3"),
                sq("g7"),
                &[
                    sq("a1"),
                    sq("b2"),
                    sq("c3"),
                    sq("d4"),
                    sq("e5"),
                    sq("f6"),
                    sq("g7"),
                    sq("h8"),
                ],
            ),
            ("same square", sq("f6"), sq("f6"), &[]),
            ("knight jump", sq("a1"), sq("b3"), &[]),
        ];

        for &(name, a, b, expected) in cases {
            let actual = line(a, b).into_iter().collect::<Vec<Square>>();
            assert_eq!(expected, actual, "{name}");
        }
    }
}
