//! Castling geometry, exposed as [CastleSide::path].

use crate::types::{Bitboard, CastleSide, Color, Square};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CastlingPath {
    pub king_from: Square,
    pub king_to: Square,
    pub rook_from: Square,
    pub rook_to: Square,
    pub must_be_empty: Bitboard,
    pub must_be_safe: Bitboard,
}

const fn sq(f: u8, r: u8) -> Square {
    Square::from_file_rank(f, r).expect("file and rank in 0..8")
}

const WHITE_KINGSIDE: CastlingPath = CastlingPath {
    king_from: sq(4, 0),
    king_to: sq(6, 0),
    rook_from: sq(7, 0),
    rook_to: sq(5, 0),
    must_be_empty: Bitboard(0b0110_0000),
    must_be_safe: Bitboard(0b0111_0000),
};

const WHITE_QUEENSIDE: CastlingPath = CastlingPath {
    king_from: sq(4, 0),
    king_to: sq(2, 0),
    rook_from: sq(0, 0),
    rook_to: sq(3, 0),
    must_be_empty: Bitboard(0b0000_1110),
    must_be_safe: Bitboard(0b0001_1100),
};

static PATHS: [[CastlingPath; CastleSide::COUNT]; Color::COUNT] = [
    [WHITE_KINGSIDE, WHITE_QUEENSIDE],
    [WHITE_KINGSIDE.flipped(), WHITE_QUEENSIDE.flipped()],
];

impl CastleSide {
    #[inline]
    pub fn path(self, color: Color) -> &'static CastlingPath {
        &PATHS[color.index()][self.index()]
    }
}

impl CastlingPath {
    const fn flipped(self) -> Self {
        Self {
            king_from: self.king_from.flip_rank(),
            king_to: self.king_to.flip_rank(),
            rook_from: self.rook_from.flip_rank(),
            rook_to: self.rook_to.flip_rank(),
            must_be_empty: self.must_be_empty.flip_vertical(),
            must_be_safe: self.must_be_safe.flip_vertical(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use CastleSide::*;
    use Color::*;

    fn square(name: &str) -> Square {
        name.parse().expect("test square names are valid")
    }

    /// Bitboard of the named squares, built through `Square` rather than bit
    /// literals, so it can't share a bit-order mistake with the table.
    fn squares(names: &[&str]) -> Bitboard {
        names
            .iter()
            .fold(Bitboard::EMPTY, |acc, &name| acc | square(name).bitboard())
    }

    /// Squares on `rank` from file `a` to file `b` inclusive, in either order.
    fn rank_span(rank: u8, a: u8, b: u8) -> Bitboard {
        (a.min(b)..=a.max(b)).fold(Bitboard::EMPTY, |acc, file| {
            let s = Square::from_file_rank(file, rank).expect("file and rank are below 8");
            acc | s.bitboard()
        })
    }

    fn all_paths() -> impl Iterator<Item = (Color, CastleSide, &'static CastlingPath)> {
        Color::ALL.into_iter().flat_map(|color| {
            CastleSide::ALL
                .into_iter()
                .map(move |side| (color, side, side.path(color)))
        })
    }

    #[track_caller]
    fn assert_mask(label: &str, expected: Bitboard, actual: Bitboard) {
        assert_eq!(
            actual, expected,
            "{label}\nexpected:\n{expected}\nactual:\n{actual}"
        );
    }

    #[test]
    fn paths_match_named_squares() {
        // (color, side, [king_from, king_to, rook_from, rook_to], must_be_empty, must_be_safe)
        type Row<'a> = (
            Color,
            CastleSide,
            [&'a str; 4],
            &'a [&'a str],
            &'a [&'a str],
        );
        let cases: &[Row] = &[
            (
                White,
                KingSide,
                ["e1", "g1", "h1", "f1"],
                &["f1", "g1"],
                &["e1", "f1", "g1"],
            ),
            (
                White,
                QueenSide,
                ["e1", "c1", "a1", "d1"],
                &["b1", "c1", "d1"],
                &["c1", "d1", "e1"],
            ),
            (
                Black,
                KingSide,
                ["e8", "g8", "h8", "f8"],
                &["f8", "g8"],
                &["e8", "f8", "g8"],
            ),
            (
                Black,
                QueenSide,
                ["e8", "c8", "a8", "d8"],
                &["b8", "c8", "d8"],
                &["c8", "d8", "e8"],
            ),
        ];

        for &(color, side, names, empty, safe) in cases {
            let path = side.path(color);
            let actual =
                [path.king_from, path.king_to, path.rook_from, path.rook_to].map(|s| s.to_string());
            assert_eq!(
                actual, names,
                "{color} {side}: king_from, king_to, rook_from, rook_to"
            );
            assert_mask(
                &format!("{color} {side} must_be_empty"),
                squares(empty),
                path.must_be_empty,
            );
            assert_mask(
                &format!("{color} {side} must_be_safe"),
                squares(safe),
                path.must_be_safe,
            );
        }
    }

    #[test]
    fn every_square_is_on_the_back_rank() {
        for (color, side, path) in all_paths() {
            let back_rank = match color {
                White => 0,
                Black => 7,
            };
            let fields = [
                ("king_from", path.king_from),
                ("king_to", path.king_to),
                ("rook_from", path.rook_from),
                ("rook_to", path.rook_to),
            ];
            for (field, s) in fields {
                assert_eq!(s.rank(), back_rank, "{color} {side} {field} is {s}");
            }
        }
    }

    #[test]
    fn king_moves_two_files_and_rook_lands_on_the_square_it_crosses() {
        for (color, side, path) in all_paths() {
            let from = path.king_from.file();
            let to = path.king_to.file();
            assert_eq!(from.abs_diff(to), 2, "{color} {side}: king moves two files");
            assert_eq!(
                path.rook_to.file(),
                (from + to) / 2,
                "{color} {side}: rook lands on the square the king crosses"
            );
        }
    }

    #[test]
    fn must_be_empty_is_strictly_between_king_and_rook() {
        for (color, side, path) in all_paths() {
            let span = rank_span(
                path.king_from.rank(),
                path.king_from.file(),
                path.rook_from.file(),
            );
            let between = span & !(path.king_from.bitboard() | path.rook_from.bitboard());
            assert_mask(
                &format!("{color} {side} must_be_empty"),
                between,
                path.must_be_empty,
            );
        }
    }

    #[test]
    fn must_be_safe_is_the_kings_path() {
        for (color, side, path) in all_paths() {
            let walk = rank_span(
                path.king_from.rank(),
                path.king_from.file(),
                path.king_to.file(),
            );
            assert_mask(
                &format!("{color} {side} must_be_safe"),
                walk,
                path.must_be_safe,
            );
        }
    }

    /// Queenside, the b-file square must be empty but needn't be safe; the
    /// king's start must be safe but obviously isn't empty. Copying one mask
    /// into the other breaks this.
    #[test]
    fn king_start_is_the_only_safe_square_that_need_not_be_empty() {
        for (color, side, path) in all_paths() {
            assert_mask(
                &format!("{color} {side} must_be_safe minus must_be_empty"),
                path.king_from.bitboard(),
                path.must_be_safe & !path.must_be_empty,
            );
        }
    }
}
