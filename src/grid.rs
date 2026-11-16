use crate::{
    board::Board,
    types::{Color, Piece, PieceType, Square},
};
use std::{
    fmt::{self, Display},
    str::FromStr,
};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum GlyphError {
    #[error("unknown glyph set")]
    UnknownGlyphs,
}

/// How [`Grid`] draws pieces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyphs {
    Ascii,
    Unicode,
}

impl Glyphs {
    pub const ALL: [Self; 2] = [Self::Ascii, Self::Unicode];

    fn name(self) -> &'static str {
        match self {
            Self::Ascii => "ascii",
            Self::Unicode => "unicode",
        }
    }

    /// The character that draws `piece` in this glyph set.
    ///
    /// Nerd Font icons have one shape for both colors,
    /// so [`Grid`] differentiates them with ANSI colors instead.
    fn piece(self, piece: Piece) -> char {
        use Color::{Black, White};
        use PieceType::{Bishop, King, Knight, Pawn, Queen, Rook};

        match self {
            Glyphs::Ascii => piece.fen_char(),
            Glyphs::Unicode => match (piece.color, piece.piece_type) {
                (White, King) => '\u{2654}',   // ♔
                (White, Queen) => '\u{2655}',  // ♕
                (White, Rook) => '\u{2656}',   // ♖
                (White, Bishop) => '\u{2657}', // ♗
                (White, Knight) => '\u{2658}', // ♘
                (White, Pawn) => '\u{2659}',   // ♙
                (Black, King) => '\u{265A}',   // ♚
                (Black, Queen) => '\u{265B}',  // ♛
                (Black, Rook) => '\u{265C}',   // ♜
                (Black, Bishop) => '\u{265D}', // ♝
                (Black, Knight) => '\u{265E}', // ♞
                (Black, Pawn) => '\u{265F}',   // ♟
            },
        }
    }
}

impl fmt::Display for Glyphs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl FromStr for Glyphs {
    type Err = GlyphError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .iter()
            .find(|g| g.name() == s)
            .ok_or(GlyphError::UnknownGlyphs)
            .copied()
    }
}

/// A board drawn with a chosen glyph set, from [`Board::grid`].
pub struct Grid<'a> {
    board: &'a Board,
    glyphs: Glyphs,
}

impl Display for Grid<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for rank in (0..8).rev() {
            write!(f, "{}", rank + 1)?;
            for file in 0..8 {
                let square = Square::from_file_rank(file, rank).expect("file and rank are in 0..8");
                let glyph = if let Some(piece) = self.board.piece_at(square) {
                    self.glyphs.piece(piece)
                } else {
                    '.'
                };
                write!(f, " {}", glyph)?;
            }
            writeln!(f)?;
        }
        writeln!(f, " a b c d e f g h")
    }
}

impl Board {
    pub fn grid(&self, glyphs: Glyphs) -> Grid<'_> {
        Grid {
            board: self,
            glyphs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{KIWIPETE, PERFT3, PERFT4, PERFT5, PERFT6, STARTING_FEN, from_fen};

    const POSITIONS: [&str; 6] = [STARTING_FEN, KIWIPETE, PERFT3, PERFT4, PERFT5, PERFT6];

    #[test]
    fn piece_glyphs() {
        type Cases = &'static [(Glyphs, char, char)];
        let cases: Cases = &[
            (Glyphs::Ascii, 'K', 'K'),
            (Glyphs::Ascii, 'k', 'k'),
            (Glyphs::Ascii, 'N', 'N'),
            (Glyphs::Ascii, 'p', 'p'),
            (Glyphs::Unicode, 'K', '♔'),
            (Glyphs::Unicode, 'Q', '♕'),
            (Glyphs::Unicode, 'R', '♖'),
            (Glyphs::Unicode, 'B', '♗'),
            (Glyphs::Unicode, 'N', '♘'),
            (Glyphs::Unicode, 'P', '♙'),
            (Glyphs::Unicode, 'k', '♚'),
            (Glyphs::Unicode, 'q', '♛'),
            (Glyphs::Unicode, 'r', '♜'),
            (Glyphs::Unicode, 'b', '♝'),
            (Glyphs::Unicode, 'n', '♞'),
            (Glyphs::Unicode, 'p', '♟'),
        ];
        for &(glyphs, fen_char, expected) in cases {
            let piece = Piece::from_fen_char(fen_char).expect("valid FEN letter");
            assert_eq!(
                expected,
                glyphs.piece(piece),
                "{glyphs:?} glyph for {fen_char:?}"
            );
        }
    }

    #[test]
    fn ascii_starting_position() {
        let expected = "\
8 r n b q k b n r
7 p p p p p p p p
6 . . . . . . . .
5 . . . . . . . .
4 . . . . . . . .
3 . . . . . . . .
2 P P P P P P P P
1 R N B Q K B N R
  a b c d e f g h
";
        let board = from_fen(STARTING_FEN);
        assert_eq!(expected, board.grid(Glyphs::Ascii).to_string());
    }

    /// Oracle: `Board`'s `Display`, which is tested on its own and shares no
    /// code with `Grid`.
    #[test]
    fn ascii_matches_board_display() {
        for fen in POSITIONS {
            let board = from_fen(fen);
            let grid = board.grid(Glyphs::Ascii).to_string();
            let display = board.to_string();
            assert!(
                display.starts_with(&grid),
                "{fen}: grid\n{grid}\nis not the start of Display\n{display}"
            );
        }
    }

    #[test]
    fn unicode_small_position() {
        let expected = "\
8 . . . . ♚ . . .
7 . . . . . . . .
6 . . . . . . . .
5 . . . . . . . .
4 . . . . . . . .
3 . . . . . . . .
2 . . . . ♙ . . .
1 . . . . ♔ . . .
  a b c d e f g h
";
        let board = from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1");
        assert_eq!(expected, board.grid(Glyphs::Unicode).to_string());
    }
}
