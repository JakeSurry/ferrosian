//! Leaper attacks (knight, king, pawn): one `static` table per piece, filled at compile time
//! by `attack_table!` from the guarded one-step shifts on [`Bitboard`].

use crate::types::{Bitboard, Color, Square};

/// Knight jumps from every square in `bb`, each composed of two guarded one-step shifts.
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

/// The eight neighbours of every square in `bb`.
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

/// The two forward diagonals (towards rank 8) of every square in `bb`.
const fn white_pawn_attacks_from(bb: Bitboard) -> Bitboard {
    bb.north_east().union(bb.north_west())
}

/// The two forward diagonals (towards rank 1) of every square in `bb`.
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

/// Knight attacks, indexed by square.
static KNIGHT_ATTACKS: [Bitboard; Square::COUNT] = attack_table!(knight_attacks_from);

/// King attacks, indexed by square.
static KING_ATTACKS: [Bitboard; Square::COUNT] = attack_table!(king_attacks_from);

/// Pawn attacks, indexed by color, then square.
static PAWN_ATTACKS: [[Bitboard; Square::COUNT]; Color::COUNT] = [
    attack_table!(white_pawn_attacks_from),
    attack_table!(black_pawn_attacks_from),
];

/// The squares a knight on `square` attacks.
#[inline]
pub fn knight_attacks(square: Square) -> Bitboard {
    KNIGHT_ATTACKS[square.index()]
}

/// The squares a king on `square` attacks (castling is not an attack).
#[inline]
pub fn king_attacks(square: Square) -> Bitboard {
    KING_ATTACKS[square.index()]
}

/// The squares a `color` pawn on `square` attacks: its forward diagonals, empty on the far rank.
///
/// To find the `color` pawns attacking a square, look up the opposite color from that square:
/// a white pawn on a attacks b exactly when a black pawn on b would attack a.
#[inline]
pub fn pawn_attacks(color: Color, square: Square) -> Bitboard {
    PAWN_ATTACKS[color.index()][square.index()]
}
