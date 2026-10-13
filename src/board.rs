//! The chess position: piece placement plus side to move, castling rights,
//! en passant square and move clocks.
//!
//! Squares run a1 = 0 through h8 = 63. Piece placement is stored twice, as
//! bitboards (one per color and piece type, plus per-color occupancy) and as a
//! 64-entry mailbox. The two must always agree, so all changes to placement go
//! through `put_piece` and `remove_piece`.

mod fen;
pub use fen::{Fen, FenError};

use crate::types::{CastlingRights, Color, Piece, PieceType, Square};
use std::fmt;

pub const STARTING_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Board {
    bitboards: [[u64; PieceType::COUNT]; Color::COUNT],
    occupancy: [u64; Color::COUNT],
    mailbox: [Option<Piece>; Square::COUNT],
    side_to_move: Color,
    castling_rights: CastlingRights,
    en_passant_square: Option<Square>,
    halfmove_clock: u16,
    fullmove_count: u16,
}

impl Board {
    /// The empty board, at move 1 with white to move.
    pub const fn empty() -> Self {
        Self {
            bitboards: [[0; PieceType::COUNT]; Color::COUNT],
            occupancy: [0; Color::COUNT],
            mailbox: [None; Square::COUNT],
            side_to_move: Color::White,
            castling_rights: CastlingRights::NONE,
            en_passant_square: None,
            halfmove_clock: 0,
            fullmove_count: 1,
        }
    }

    /// The standard starting position.
    pub fn starting_position() -> Self {
        STARTING_FEN.parse().expect("starting fen is valid")
    }

    /// Places `piece` on `square`, updating its bitboard, its color's
    /// occupancy and the mailbox. The square must be empty.
    fn put_piece(&mut self, square: Square, piece: Piece) {
        let bit = square.bitboard();
        debug_assert_eq!(
            self.all_occupancy() & bit,
            0,
            "put_piece on occupied square {square}"
        );
        debug_assert!(self.mailbox[square.index()].is_none());

        let (color, piece_type) = (piece.color.index(), piece.piece_type.index());
        self.bitboards[color][piece_type] |= bit;
        self.occupancy[color] |= bit;
        self.mailbox[square.index()] = Some(piece);
    }

    /// Removes and returns the piece on `square`, clearing its bitboard bit,
    /// its color's occupancy and the mailbox. The square must be occupied.
    fn remove_piece(&mut self, square: Square) -> Piece {
        let bit = square.bitboard();
        let piece = self.mailbox[square.index()]
            .take()
            .expect("remove_piece called on an empty square");

        let (color, piece_type) = (piece.color.index(), piece.piece_type.index());
        debug_assert_ne!(
            self.bitboards[color][piece_type] & bit,
            0,
            "mailbox and bitboards disagree at {square}"
        );
        self.bitboards[color][piece_type] &= !bit;
        self.occupancy[color] &= !bit;
        piece
    }

    /// Piece at `square`, if any.
    pub fn piece_at(&self, square: Square) -> Option<Piece> {
        self.mailbox[square.index()]
    }

    /// Bitboard of all `color` pieces of type `piece_type`.
    pub fn pieces(&self, color: Color, piece_type: PieceType) -> u64 {
        self.bitboards[color.index()][piece_type.index()]
    }

    /// Bitboard for all pieces of `color`.
    pub fn occupancy(&self, color: Color) -> u64 {
        self.occupancy[color.index()]
    }

    /// Bitboard for all pieces of both colors.
    pub fn all_occupancy(&self) -> u64 {
        self.occupancy(Color::White) | self.occupancy(Color::Black)
    }

    /// The square of `color`'s king, or `None` if that side has no king.
    pub fn king_square(&self, color: Color) -> Option<Square> {
        let kings = self.pieces(color, PieceType::King);
        debug_assert!(kings.count_ones() <= 1, "{color:?} has more than one king");
        if kings == 0 {
            None
        } else {
            Square::from_index(kings.trailing_zeros() as usize)
        }
    }

    /// Side to move
    pub fn side_to_move(&self) -> Color {
        self.side_to_move
    }

    /// Castling Rights
    pub fn castling_rights(&self) -> CastlingRights {
        self.castling_rights
    }

    /// En Passant Square
    pub fn en_passant_square(&self) -> Option<Square> {
        self.en_passant_square
    }

    /// Halfmove Clock
    pub fn halfmove_clock(&self) -> u16 {
        self.halfmove_clock
    }

    /// Fullmove Count
    pub fn fullmove_count(&self) -> u16 {
        self.fullmove_count
    }

    /// Checks that the bitboards, occupancy and mailbox all describe the same
    /// position. Returns a description of the first problem found.
    pub fn is_consistent(&self) -> Result<(), String> {
        // Each color's occupancy must equal the OR of that color's piece bitboards.
        for color in Color::ALL {
            let combined = self.bitboards[color.index()]
                .iter()
                .fold(0, |acc, &bb| acc | bb);
            let stored = self.occupancy[color.index()];
            if combined != stored {
                return Err(format!(
                    "{color:?} occupancy is {stored:#018x}, but its pieces combine to {combined:#018x}"
                ));
            }
        }
        // Every square must be in at most one piece bitboard, and that piece
        // (or emptiness) must match the mailbox.
        for index in 0..Square::COUNT {
            let square = Square::from_index(index).expect("index is below Square::COUNT");
            let bit = square.bitboard();

            let mut found: Option<Piece> = None;
            for color in Color::ALL {
                for piece_type in PieceType::ALL {
                    if self.bitboards[color.index()][piece_type.index()] & bit == 0 {
                        continue;
                    }
                    let piece = Piece::new(color, piece_type);
                    if let Some(other) = found {
                        return Err(format!(
                            "square {square} is in both the {} and {} bitboards",
                            other.name(),
                            piece.name(),
                        ));
                    }
                    found = Some(piece);
                }
            }

            let in_mailbox = self.mailbox[index];
            if found != in_mailbox {
                return Err(format!(
                    "square {square}: bitboards say {}, mailbox says {}",
                    found.map_or_else(|| "empty".to_string(), Piece::name),
                    in_mailbox.map_or_else(|| "empty".to_string(), Piece::name),
                ));
            }
        }

        Ok(())
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for rank in (0..8).rev() {
            write!(f, "{}", rank + 1)?;
            for file in 0..8 {
                let square = Square::from_file_rank(file, rank).expect("file and rank are in 0..8");
                let c = self.mailbox[square.index()].map_or('.', Piece::fen_char);
                write!(f, " {c}")?;
            }
            writeln!(f)?;
        }
        writeln!(f, "  a b c d e f g h")?;
        writeln!(f)?;

        writeln!(f, "Side to move: {}", self.side_to_move)?;
        writeln!(f, "Castling: {}", self.castling_rights)?;
        match self.en_passant_square {
            Some(square) => writeln!(f, "En passant: {square}")?,
            None => writeln!(f, "En passant: -")?,
        }
        writeln!(f, "Halfmove clock: {}", self.halfmove_clock)?;
        write!(f, "Fullmove number: {}", self.fullmove_count)
    }
}

#[cfg(test)]
mod test_helpers {
    use super::*;

    /// Square from its name, e.g. `sq("e4")`.
    pub fn sq(name: &str) -> Square {
        name.parse().unwrap()
    }

    /// Piece from its FEN letter, e.g. `p('N')` for a white knight.
    pub fn p(c: char) -> Piece {
        Piece::from_fen_char(c).unwrap()
    }

    /// Builds a board from (square, FEN letter) pairs using `put_piece`.
    pub fn board_with(pieces: &[(&str, char)]) -> Board {
        let mut board = Board::empty();
        for &(square, c) in pieces {
            board.put_piece(sq(square), p(c));
        }
        board
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::test_helpers::*;

    // --- empty board ---------------------------------------------------

    #[test]
    fn empty_board_has_no_pieces() {
        let board = Board::empty();
        for index in 0..Square::COUNT {
            assert_eq!(board.piece_at(Square::from_index(index).unwrap()), None);
        }
        for color in Color::ALL {
            assert_eq!(board.occupancy(color), 0);
            for piece_type in PieceType::ALL {
                assert_eq!(board.pieces(color, piece_type), 0);
            }
        }
        assert_eq!(board.all_occupancy(), 0);
        assert_eq!(board.is_consistent(), Ok(()));
    }

    #[test]
    fn empty_board_state() {
        let board = Board::empty();
        assert_eq!(board.side_to_move(), Color::White);
        assert_eq!(board.castling_rights(), CastlingRights::NONE);
        assert_eq!(board.en_passant_square(), None);
        assert_eq!(board.halfmove_clock(), 0);
        assert_eq!(board.fullmove_count(), 1);
    }

    // --- put_piece -----------------------------------------------------

    #[test]
    fn put_piece_updates_everything() {
        let board = board_with(&[("e4", 'N')]);
        let bit = sq("e4").bitboard();

        assert_eq!(board.piece_at(sq("e4")), Some(p('N')));
        assert_eq!(board.pieces(Color::White, PieceType::Knight), bit);
        assert_eq!(board.occupancy(Color::White), bit);
        assert_eq!(board.occupancy(Color::Black), 0);
        assert_eq!(board.all_occupancy(), bit);
        assert_eq!(board.is_consistent(), Ok(()));
    }

    #[test]
    fn put_piece_keeps_colors_and_types_separate() {
        let board = board_with(&[("a1", 'R'), ("h1", 'R'), ("a8", 'r'), ("d4", 'Q')]);

        assert_eq!(
            board.pieces(Color::White, PieceType::Rook),
            sq("a1").bitboard() | sq("h1").bitboard()
        );
        assert_eq!(
            board.pieces(Color::Black, PieceType::Rook),
            sq("a8").bitboard()
        );
        assert_eq!(
            board.pieces(Color::White, PieceType::Queen),
            sq("d4").bitboard()
        );
        assert_eq!(board.occupancy(Color::Black), sq("a8").bitboard());
        assert_eq!(board.all_occupancy().count_ones(), 4);
        assert_eq!(board.is_consistent(), Ok(()));
    }

    #[test]
    fn put_piece_on_corners() {
        let board = board_with(&[("a1", 'K'), ("h1", 'B'), ("a8", 'k'), ("h8", 'b')]);
        assert_eq!(board.piece_at(sq("a1")), Some(p('K')));
        assert_eq!(board.piece_at(sq("h1")), Some(p('B')));
        assert_eq!(board.piece_at(sq("a8")), Some(p('k')));
        assert_eq!(board.piece_at(sq("h8")), Some(p('b')));
        assert_eq!(board.is_consistent(), Ok(()));
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "occupied")]
    fn put_piece_on_occupied_square_panics() {
        let mut board = board_with(&[("e4", 'N')]);
        board.put_piece(sq("e4"), p('p'));
    }

    // --- remove_piece --------------------------------------------------

    #[test]
    fn remove_piece_returns_piece_and_clears_everything() {
        let mut board = board_with(&[("e4", 'N')]);
        assert_eq!(board.remove_piece(sq("e4")), p('N'));
        assert_eq!(board, Board::empty());
    }

    #[test]
    fn remove_one_of_several_same_pieces() {
        // All eight white pawns, then remove one from the middle.
        let mut board = Board::empty();
        for file in 0..8 {
            board.put_piece(Square::from_file_rank(file, 1).unwrap(), p('P'));
        }
        assert_eq!(board.remove_piece(sq("e2")), p('P'));

        let pawns = board.pieces(Color::White, PieceType::Pawn);
        assert_eq!(pawns.count_ones(), 7);
        assert_eq!(pawns & sq("e2").bitboard(), 0);
        assert_eq!(board.piece_at(sq("e2")), None);
        assert_eq!(board.piece_at(sq("d2")), Some(p('P')));
        assert_eq!(board.is_consistent(), Ok(()));
    }

    #[test]
    fn remove_leaves_other_pieces_alone() {
        let mut board = board_with(&[("e1", 'K'), ("e8", 'k'), ("d5", 'p')]);
        board.remove_piece(sq("d5"));
        assert_eq!(board, board_with(&[("e1", 'K'), ("e8", 'k')]));
    }

    #[test]
    fn put_and_remove_round_trip() {
        let placements = [
            ("a1", 'R'),
            ("h8", 'r'),
            ("e4", 'P'),
            ("d5", 'p'),
            ("g1", 'N'),
        ];
        let mut board = board_with(&placements);
        for &(square, c) in &placements {
            assert_eq!(board.remove_piece(sq(square)), p(c));
            assert_eq!(board.is_consistent(), Ok(()));
        }
        assert_eq!(board, Board::empty());
    }

    #[test]
    fn capture_by_remove_then_put() {
        // How a capture will look in makemove: remove the victim, move the attacker.
        let mut board = board_with(&[("e4", 'N'), ("d6", 'b')]);
        let captured = board.remove_piece(sq("d6"));
        let knight = board.remove_piece(sq("e4"));
        board.put_piece(sq("d6"), knight);

        assert_eq!(captured, p('b'));
        assert_eq!(board, board_with(&[("d6", 'N')]));
        assert_eq!(board.occupancy(Color::Black), 0);
    }

    #[test]
    #[should_panic(expected = "empty square")]
    fn remove_from_empty_square_panics() {
        Board::empty().remove_piece(sq("e4"));
    }

    // --- king_square ---------------------------------------------------

    #[test]
    fn king_square_finds_kings() {
        let board = board_with(&[("e1", 'K'), ("e8", 'k')]);
        assert_eq!(board.king_square(Color::White), Some(sq("e1")));
        assert_eq!(board.king_square(Color::Black), Some(sq("e8")));
    }

    #[test]
    fn king_square_on_corners() {
        let board = board_with(&[("a1", 'K'), ("h8", 'k')]);
        assert_eq!(board.king_square(Color::White), Some(sq("a1")));
        assert_eq!(board.king_square(Color::Black), Some(sq("h8")));
    }

    #[test]
    fn king_square_without_king() {
        let board = board_with(&[("e1", 'K')]);
        assert_eq!(board.king_square(Color::Black), None);
        assert_eq!(Board::empty().king_square(Color::White), None);
    }

    // --- is_consistent detects corruption --------------------------------

    fn sample() -> Board {
        board_with(&[
            ("e1", 'K'),
            ("e8", 'k'),
            ("e4", 'N'),
            ("a1", 'R'),
            ("h8", 'r'),
        ])
    }

    #[test]
    fn detects_bit_without_mailbox_entry() {
        let mut board = sample();
        board.mailbox[sq("e4").index()] = None;
        assert!(board.is_consistent().is_err());
    }

    #[test]
    fn detects_mailbox_entry_without_bit() {
        let mut board = sample();
        board.mailbox[sq("d5").index()] = Some(p('p'));
        assert!(board.is_consistent().is_err());
    }

    #[test]
    fn detects_wrong_piece_in_mailbox() {
        let mut board = sample();
        board.mailbox[sq("e4").index()] = Some(p('B'));
        assert!(board.is_consistent().is_err());
    }

    #[test]
    fn detects_same_color_overlap() {
        let mut board = sample();
        board.bitboards[Color::White.index()][PieceType::Bishop.index()] |= sq("e4").bitboard();
        let err = board.is_consistent().unwrap_err();
        assert!(err.contains("both"), "{err}");
    }

    #[test]
    fn detects_cross_color_overlap() {
        let mut board = sample();
        let black = Color::Black.index();
        board.bitboards[black][PieceType::Pawn.index()] |= sq("e4").bitboard();
        board.occupancy[black] |= sq("e4").bitboard();
        assert!(board.is_consistent().is_err());
    }

    #[test]
    fn detects_occupancy_mismatch() {
        let mut board = sample();
        board.occupancy[Color::White.index()] |= sq("h1").bitboard();
        let err = board.is_consistent().unwrap_err();
        assert!(err.contains("occupancy"), "{err}");
    }

    #[test]
    fn display_board() {
        let board = board_with(&[("e1", 'K'), ("e8", 'k'), ("a2", 'P'), ("h7", 'p')]);
        let expected = "\
8 . . . . k . . .
7 . . . . . . . p
6 . . . . . . . .
5 . . . . . . . .
4 . . . . . . . .
3 . . . . . . . .
2 P . . . . . . .
1 . . . . K . . .
  a b c d e f g h

Side to move: White
Castling: -
En passant: -
Halfmove clock: 0
Fullmove number: 1";
        assert_eq!(board.to_string(), expected);
    }
}
