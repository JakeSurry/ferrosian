//! How to print the position in terminal.
//!
//! Options:    Ascii (e.g. 'p', 'K')
//!             Unicode (e.g. '♙', '♚')

use super::Board;
use crate::types::{Piece, Square};
use std::fmt;

/// How [`Grid`] draws pieces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyphs {
    Ascii,
    Unicode,
}

impl Glyphs {
    /// The character that draws `piece` in this glyph set.
    fn piece(self, piece: Piece) -> char {
        match self {
            Glyphs::Ascii => piece.fen_char(),
            Glyphs::Unicode => piece.unicode_char(),
        }
    }
}

/// A board drawn with a chosen glyph set, from [`Board::grid`].
pub struct Grid<'a> {
    board: &'a Board,
    glyphs: Glyphs,
}

impl fmt::Display for Grid<'_> {
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
                write!(f, " {glyph}")?;
            }
            writeln!(f)?;
        }
        writeln!(f, "  a b c d e f g h")
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
    use crate::test_helpers::{STARTING_FEN, from_fen};

    #[test]
    fn piece_glyphs() {
        type Cases = &'static [(Glyphs, char, char)];
        let cases: Cases = &[
            (Glyphs::Ascii, 'K', 'K'),
            (Glyphs::Ascii, 'k', 'k'),
            (Glyphs::Ascii, 'N', 'N'),
            (Glyphs::Ascii, 'p', 'p'),
            (Glyphs::Unicode, 'k', '♔'),
            (Glyphs::Unicode, 'q', '♕'),
            (Glyphs::Unicode, 'r', '♖'),
            (Glyphs::Unicode, 'b', '♗'),
            (Glyphs::Unicode, 'n', '♘'),
            (Glyphs::Unicode, 'p', '♙'),
            (Glyphs::Unicode, 'K', '♚'),
            (Glyphs::Unicode, 'Q', '♛'),
            (Glyphs::Unicode, 'R', '♜'),
            (Glyphs::Unicode, 'B', '♝'),
            (Glyphs::Unicode, 'N', '♞'),
            (Glyphs::Unicode, 'P', '♟'),
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

    #[test]
    fn unicode_small_position() {
        let expected = "\
8 . . . . ♔ . . .
7 . . . . . . . .
6 . . . . . . . .
5 . . . . . . . .
4 . . . . . . . .
3 . . . . . . . .
2 . . . . ♟ . . .
1 . . . . ♚ . . .
  a b c d e f g h
";
        let board = from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1");
        assert_eq!(expected, board.grid(Glyphs::Unicode).to_string());
    }
}
