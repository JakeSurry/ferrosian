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

type Step = fn(Bitboard) -> Bitboard;

const ROOK_STEPS: [Step; 4] = [
    Bitboard::north,
    Bitboard::south,
    Bitboard::east,
    Bitboard::west,
];
const BISHOP_STEPS: [Step; 4] = [
    Bitboard::north_east,
    Bitboard::north_west,
    Bitboard::south_east,
    Bitboard::south_west,
];

fn ray(start: Bitboard, blockers: Bitboard, step: Step) -> Bitboard {
    let mut current = step(start);
    let mut attacks = Bitboard::EMPTY;

    while !current.is_empty() {
        attacks |= current;
        if current.overlaps(blockers) {
            break;
        }
        current = step(current);
    }
    attacks
}

fn slider_attacks(square: Square, blockers: Bitboard, steps: &[Step]) -> Bitboard {
    let start = square.bitboard();
    steps.iter().fold(Bitboard::EMPTY, |acc, &step| {
        acc | ray(start, blockers, step)
    })
}

#[inline]
pub fn rook_attacks(square: Square, blockers: Bitboard) -> Bitboard {
    slider_attacks(square, blockers, &ROOK_STEPS)
}

#[inline]
pub fn bishop_attacks(square: Square, blockers: Bitboard) -> Bitboard {
    slider_attacks(square, blockers, &BISHOP_STEPS)
}

#[inline]
pub fn queen_attacks(square: Square, blockers: Bitboard) -> Bitboard {
    rook_attacks(square, blockers) | bishop_attacks(square, blockers)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sq(name: &str) -> Square {
        name.parse().unwrap()
    }

    fn all_squares() -> impl Iterator<Item = Square> {
        Bitboard::FULL.into_iter()
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

    /// A sliding-attack function under test.
    type Slider = fn(Square, Bitboard) -> Bitboard;

    // (file delta, rank delta) for each ray direction. The queen's directions are
    // the king's deltas, so `KING_DELTAS` is reused for it.
    const ROOK_DIRECTIONS: [(i8, i8); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
    const BISHOP_DIRECTIONS: [(i8, i8); 4] = [(1, 1), (-1, 1), (1, -1), (-1, -1)];

    /// Occupancy patterns for the reference comparison, named for failure messages.
    const OCCUPANCIES: [(&str, Bitboard); 6] = [
        ("empty", Bitboard::EMPTY),
        ("full", Bitboard::FULL),
        ("starting position", Bitboard(0xFFFF_0000_0000_FFFF)),
        ("checkerboard", Bitboard(0x55AA_55AA_55AA_55AA)),
        ("centre", Bitboard(0x0000_0018_1800_0000)),
        ("scattered", Bitboard(0x0042_0010_0800_2400)),
    ];

    /// Reference sliding attacks by coordinate arithmetic: walk each direction one
    /// square at a time, including the first occupied square, stopping at the edge.
    fn expected_slider(square: Square, blockers: Bitboard, directions: Deltas) -> Bitboard {
        let mut expected = Bitboard::EMPTY;
        for &(file_delta, rank_delta) in directions {
            let (mut file, mut rank) = (square.file(), square.rank());
            loop {
                let (Some(next_file), Some(next_rank)) = (
                    file.checked_add_signed(file_delta),
                    rank.checked_add_signed(rank_delta),
                ) else {
                    break;
                };
                let Some(target) = Square::from_file_rank(next_file, next_rank) else {
                    break;
                };
                expected |= target.bitboard();
                if blockers.contains(target) {
                    break;
                }
                (file, rank) = (next_file, next_rank);
            }
        }
        expected
    }

    fn sliders() -> [(&'static str, Slider, Deltas<'static>); 3] {
        [
            ("rook", rook_attacks, &ROOK_DIRECTIONS),
            ("bishop", bishop_attacks, &BISHOP_DIRECTIONS),
            ("queen", queen_attacks, &KING_DELTAS),
        ]
    }

    #[test]
    fn empty_board_rook_attacks_are_file_and_rank() {
        for square in all_squares() {
            let file = Bitboard::FILES[usize::from(square.file())];
            let rank = Bitboard::RANKS[usize::from(square.rank())];
            let expected = (file | rank) & !square.bitboard();
            let actual = rook_attacks(square, Bitboard::EMPTY);
            assert_eq!(
                actual, expected,
                "rook on {square}, empty board\nexpected:\n{expected}\nactual:\n{actual}"
            );
            assert_eq!(actual.count(), 14, "rook on {square}, empty board");
        }
    }

    #[test]
    fn empty_board_bishop_attack_counts() {
        let cases = [
            ("a1", 7),
            ("h1", 7),
            ("a4", 7),
            ("h8", 7),
            ("b2", 9),
            ("c3", 11),
            ("d4", 13),
            ("e5", 13),
        ];
        for (name, count) in cases {
            assert_eq!(
                bishop_attacks(sq(name), Bitboard::EMPTY).count(),
                count,
                "bishop on {name}, empty board"
            );
        }
    }

    #[test]
    fn slider_attacks_match_reference() {
        for (occupancy_name, blockers) in OCCUPANCIES {
            for (name, attacks, directions) in sliders() {
                for square in all_squares() {
                    let expected = expected_slider(square, blockers, directions);
                    let actual = attacks(square, blockers);
                    assert_eq!(
                        actual, expected,
                        "{name} on {square}, {occupancy_name} occupancy\n\
                         blockers:\n{blockers}\nexpected:\n{expected}\nactual:\n{actual}"
                    );
                }
            }
        }
    }

    #[test]
    fn queen_on_full_board_attacks_like_king() {
        // Every ray stops after one step, in all eight directions.
        for square in all_squares() {
            let expected = king_attacks(square);
            let actual = queen_attacks(square, Bitboard::FULL);
            assert_eq!(
                actual, expected,
                "queen on {square}, full board\nexpected:\n{expected}\nactual:\n{actual}"
            );
        }
    }

    #[test]
    fn slider_square_does_not_block_itself() {
        // In a real position the slider's own square is occupied (by the slider).
        for (occupancy_name, blockers) in OCCUPANCIES {
            for (name, attacks, _) in sliders() {
                for square in all_squares() {
                    let with_own = attacks(square, blockers | square.bitboard());
                    let without_own = attacks(square, blockers & !square.bitboard());
                    assert_eq!(
                        with_own, without_own,
                        "{name} on {square}, {occupancy_name} occupancy\n\
                         with own square:\n{with_own}\nwithout own square:\n{without_own}"
                    );
                }
            }
        }
    }

    #[test]
    fn slider_blockers_are_included_and_stop_the_ray() {
        // Literals only, so these don't depend on Square's own conversions.
        let a1 = Square::from_index(0).expect("a1");
        let c1 = Square::from_index(2).expect("c1");

        // Rook a1, blockers a4 (24) and f1 (5):
        // b1..f1 (bits 1-5 = 0x3E), a2 (8), a3 (16), a4 (24).
        let blockers = Bitboard((1 << 24) | (1 << 5));
        assert_eq!(
            rook_attacks(a1, blockers),
            Bitboard(0x3E | (1 << 8) | (1 << 16) | (1 << 24))
        );

        // Bishop c1, blocker e3 (20): b2 (9), a3 (16), d2 (11), e3 (20).
        let blockers = Bitboard(1 << 20);
        assert_eq!(
            bishop_attacks(c1, blockers),
            Bitboard((1 << 9) | (1 << 11) | (1 << 16) | (1 << 20))
        );
    }
}
