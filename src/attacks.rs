use crate::types::{Bitboard, Color, Square};

const fn knight_attacks_from(bb: Bitboard) -> Bitboard {
    Bitboard::union_all(&[
        bb.north().north_east(),
        bb.north().north_west(),
        bb.south().south_east(),
        bb.south().south_west(),
        bb.east().north_east(),
        bb.east().south_east(),
        bb.west().north_west(),
        bb.west().south_west(),
    ])
}

const fn king_attacks_from(bb: Bitboard) -> Bitboard {
    Bitboard::union_all(&[
        bb.north(),
        bb.south(),
        bb.east(),
        bb.west(),
        bb.north_east(),
        bb.north_west(),
        bb.south_east(),
        bb.south_west(),
    ])
}

const fn white_pawn_attacks_from(bb: Bitboard) -> Bitboard {
    bb.north_east().union(bb.north_west())
}

const fn black_pawn_attacks_from(bb: Bitboard) -> Bitboard {
    bb.south_east().union(bb.south_west())
}

/// Macro to generate attack tables. A macro is used in lieu of a function that accepts a function
/// pointer and calls it (e.g. `fn foo(bar: fn()) {bar())}`) cannot be `const`.
/// These tables should be generated at compilation, so a macro must be used.
macro_rules! attack_table {
    ($attacks_from:ident) => {{
        let mut bitboards = [Bitboard::EMPTY; Square::COUNT];
        let mut index = 0;

        while index < Square::COUNT {
            let square_bitboard = Square::from_index(index)
                .expect("square index valid in range 0..64")
                .bitboard();
            bitboards[index] = $attacks_from(square_bitboard);
            index += 1;
        }
        bitboards
    }};
}

static KNIGHT_ATTACKS: [Bitboard; Square::COUNT] = attack_table!(knight_attacks_from);
static KING_ATTACKS: [Bitboard; Square::COUNT] = attack_table!(king_attacks_from);
static PAWN_ATTACKS: [[Bitboard; Square::COUNT]; Color::COUNT] = [
    attack_table!(white_pawn_attacks_from),
    attack_table!(black_pawn_attacks_from),
];

#[inline]
pub fn knight_attacks(square: Square) -> Bitboard {
    KNIGHT_ATTACKS[square.index()]
}

#[inline]
pub fn king_attacks(square: Square) -> Bitboard {
    KING_ATTACKS[square.index()]
}

#[inline]
pub fn pawn_attacks(color: Color, square: Square) -> Bitboard {
    PAWN_ATTACKS[color.index()][square.index()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sq(name: &str) -> Square {
        name.parse().unwrap()
    }

    fn all_squares() -> impl Iterator<Item = Square> {
        (0..Square::COUNT).map(|index| Square::from_index(index).expect("index in 0..64"))
    }

    /// A lookup function under test.
    type Lookup = fn(Square) -> Bitboard;
    /// A reference to a slice of move deltas.
    type Deltas<'a> = &'a [(i8, i8)];

    // (file delta, rank delta) for each piece's attacks.
    const KNIGHT_DELTAS: [(i8, i8); 8] = [
        (1, 2),
        (2, 1),
        (2, -1),
        (1, -2),
        (-1, -2),
        (-2, -1),
        (-2, 1),
        (-1, 2),
    ];
    const KING_DELTAS: [(i8, i8); 8] = [
        (0, 1),
        (1, 1),
        (1, 0),
        (1, -1),
        (0, -1),
        (-1, -1),
        (-1, 0),
        (-1, 1),
    ];
    const WHITE_PAWN_DELTAS: [(i8, i8); 2] = [(-1, 1), (1, 1)];
    const BLACK_PAWN_DELTAS: [(i8, i8); 2] = [(-1, -1), (1, -1)];

    /// Builds the expected attack set by coordinate arithmetic, independently of
    /// the shift methods: apply each delta to the file and rank, keep on-board results.
    fn expected_from_deltas(square: Square, deltas: Deltas) -> Bitboard {
        let mut expected = Bitboard::EMPTY;
        for &(file_delta, rank_delta) in deltas {
            // Stepping off the low end (file or rank below 0) gives None.
            let (Some(file), Some(rank)) = (
                square.file().checked_add_signed(file_delta),
                square.rank().checked_add_signed(rank_delta),
            ) else {
                continue;
            };
            // Stepping off the high end (8 or more) is rejected here.
            if let Some(target) = Square::from_file_rank(file, rank) {
                expected |= target.bitboard();
            }
        }
        expected
    }

    #[test]
    fn leaper_attack_counts() {
        // (square, knight attacks, king attacks)
        let cases = [
            ("a1", 2, 3),
            ("h1", 2, 3),
            ("a8", 2, 3),
            ("h8", 2, 3),
            ("b1", 3, 5),
            ("b2", 4, 8),
            ("a4", 4, 5),
            ("h5", 4, 5),
            ("d4", 8, 8),
        ];
        for (name, knight, king) in cases {
            let square = sq(name);
            assert_eq!(
                knight_attacks(square).count(),
                knight,
                "knight attacks from {name}"
            );
            assert_eq!(
                king_attacks(square).count(),
                king,
                "king attacks from {name}"
            );
        }
    }

    #[test]
    fn attacks_match_coordinate_arithmetic() {
        let cases: [(&str, Lookup, Deltas); 4] = [
            ("knight", knight_attacks, &KNIGHT_DELTAS),
            ("king", king_attacks, &KING_DELTAS),
            (
                "white pawn",
                |s| pawn_attacks(Color::White, s),
                &WHITE_PAWN_DELTAS,
            ),
            (
                "black pawn",
                |s| pawn_attacks(Color::Black, s),
                &BLACK_PAWN_DELTAS,
            ),
        ];
        for (name, attacks, deltas) in cases {
            for square in all_squares() {
                let expected = expected_from_deltas(square, deltas);
                let actual = attacks(square);
                assert_eq!(
                    actual, expected,
                    "{name} attacks from {square}\nexpected:\n{expected}\nactual:\n{actual}"
                );
            }
        }
    }

    #[test]
    fn knight_and_king_attacks_are_symmetric() {
        let pieces: [(&str, Lookup); 2] = [("knight", knight_attacks), ("king", king_attacks)];
        for (name, attacks) in pieces {
            for from in all_squares() {
                for to in all_squares() {
                    assert_eq!(
                        attacks(from).contains(to),
                        attacks(to).contains(from),
                        "{name}: {from} attacks {to} should match {to} attacks {from}"
                    );
                }
            }
        }
    }

    #[test]
    fn pawn_attacks_mirror_between_colors() {
        // is_square_attacked relies on this: to find white pawns attacking a square,
        // look up the *black* pawn table from that square.
        for from in all_squares() {
            for to in all_squares() {
                assert_eq!(
                    pawn_attacks(Color::White, from).contains(to),
                    pawn_attacks(Color::Black, to).contains(from),
                    "white pawn on {from} attacks {to} should match black pawn on {to} attacks {from}"
                );
            }
        }
    }

    #[test]
    fn pawns_on_far_rank_attack_nothing() {
        for file in 0..8 {
            let rank_8 = Square::from_file_rank(file, 7).expect("on board");
            let rank_1 = Square::from_file_rank(file, 0).expect("on board");
            assert_eq!(
                pawn_attacks(Color::White, rank_8),
                Bitboard::EMPTY,
                "white pawn on {rank_8}"
            );
            assert_eq!(
                pawn_attacks(Color::Black, rank_1),
                Bitboard::EMPTY,
                "black pawn on {rank_1}"
            );
        }
    }

    #[test]
    fn edge_file_pawns_attack_one_square() {
        for color in Color::ALL {
            for file in [0, 7] {
                // Ranks 2 to 7: neither color's pawn is on its far rank.
                for rank in 1..7 {
                    let square = Square::from_file_rank(file, rank).expect("on board");
                    assert_eq!(
                        pawn_attacks(color, square).count(),
                        1,
                        "{color} pawn on {square}"
                    );
                }
            }
        }
    }

    #[test]
    fn attack_directions_match_index_convention() {
        // Literals only, so these don't depend on Square's own conversions.
        let a1 = Square::from_index(0).expect("a1");
        let e4 = Square::from_index(28).expect("e4");

        // Knight a1 -> c2 (10), b3 (17).
        assert_eq!(knight_attacks(a1), Bitboard((1 << 10) | (1 << 17)));
        // King a1 -> b1 (1), a2 (8), b2 (9).
        assert_eq!(king_attacks(a1), Bitboard((1 << 1) | (1 << 8) | (1 << 9)));
        // White pawn e4 -> d5 (35), f5 (37).
        assert_eq!(
            pawn_attacks(Color::White, e4),
            Bitboard((1 << 35) | (1 << 37))
        );
        // Black pawn e4 -> d3 (19), f3 (21).
        assert_eq!(
            pawn_attacks(Color::Black, e4),
            Bitboard((1 << 19) | (1 << 21))
        );
    }
}
