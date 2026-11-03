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

pub(super) fn knight_leaps(square: Square) -> Bitboard {
    KNIGHT_ATTACKS[square.index()]
}

pub(super) fn king_leaps(square: Square) -> Bitboard {
    KING_ATTACKS[square.index()]
}

pub(super) fn pawn_leaps(color: Color, square: Square) -> Bitboard {
    PAWN_ATTACKS[color.index()][square.index()]
}
