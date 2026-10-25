//! The chess position: piece placement plus side to move, castling rights,
//! en passant square and move clocks.
//!
//! Squares run a1 = 0 through h8 = 63. Piece placement is stored twice, as
//! bitboards (one per color and piece type, plus per-color occupancy) and as a
//! 64-entry mailbox. The two must always agree, so all changes to placement go
//! through `put_piece` and `remove_piece`.

mod fen;
pub use fen::{Fen, FenError};

mod validate;

use crate::attacks;
use crate::types::{Bitboard, CastlingRights, Color, Piece, PieceType, Square};
use std::fmt;

pub const STARTING_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Board {
    bitboards: [[Bitboard; PieceType::COUNT]; Color::COUNT],
    occupancy: [Bitboard; Color::COUNT],
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
            bitboards: [[Bitboard::EMPTY; PieceType::COUNT]; Color::COUNT],
            occupancy: [Bitboard::EMPTY; Color::COUNT],
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
            Bitboard::EMPTY,
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
            Bitboard::EMPTY,
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
    pub fn pieces(&self, color: Color, piece_type: PieceType) -> Bitboard {
        self.bitboards[color.index()][piece_type.index()]
    }

    /// Bitboard for all pieces of `color`.
    pub fn occupancy(&self, color: Color) -> Bitboard {
        self.occupancy[color.index()]
    }

    /// Bitboard for all pieces of both colors.
    pub fn all_occupancy(&self) -> Bitboard {
        self.occupancy(Color::White) | self.occupancy(Color::Black)
    }

    /// The square of `color`'s king, or `None` if that side has no king.
    pub fn king_square(&self, color: Color) -> Option<Square> {
        self.pieces(color, PieceType::King).lsb()
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
            let combined = Bitboard::union_all(&self.bitboards[color.index()]);
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
                    if self.bitboards[color.index()][piece_type.index()] & bit == Bitboard::EMPTY {
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

    pub fn attackers_to(&self, square: Square, by: Color) -> Bitboard {
        let blockers = self.all_occupancy();
        let queen = self.pieces(by, PieceType::Queen);
        Bitboard::union_all(&[
            attacks::king_attacks(square) & self.pieces(by, PieceType::King),
            attacks::pawn_attacks(by.opposite(), square) & self.pieces(by, PieceType::Pawn),
            attacks::knight_attacks(square) & self.pieces(by, PieceType::Knight),
            attacks::bishop_attacks(square, blockers)
                & (self.pieces(by, PieceType::Bishop) | queen),
            attacks::rook_attacks(square, blockers) & (self.pieces(by, PieceType::Rook) | queen),
        ])
    }

    pub fn is_square_attacked(&self, square: Square, by: Color) -> bool {
        !self.attackers_to(square, by).is_empty()
    }

    pub fn is_in_check(&self, color: Color) -> bool {
        self.king_square(color)
            .is_some_and(|square| self.is_square_attacked(square, color.opposite()))
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
    use crate::attacks;
    use crate::board::test_helpers::*;

    // --- empty board ---------------------------------------------------

    #[test]
    fn empty_board_has_no_pieces() {
        let board = Board::empty();
        for index in 0..Square::COUNT {
            assert_eq!(board.piece_at(Square::from_index(index).unwrap()), None);
        }
        for color in Color::ALL {
            assert!(board.occupancy(color).is_empty());
            for piece_type in PieceType::ALL {
                assert!(board.pieces(color, piece_type).is_empty());
            }
        }
        assert!(board.all_occupancy().is_empty());
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
        assert!(board.occupancy(Color::Black).is_empty());
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
        assert_eq!(board.all_occupancy().count(), 4);
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
        assert_eq!(pawns.count(), 7);
        assert_eq!(pawns & sq("e2").bitboard(), Bitboard::EMPTY);
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
        assert!(board.occupancy(Color::Black).is_empty());
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

    // --- Display::fmt works correctly --------------------------------

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

    // --- attacks and check ----------------------------------------------

    fn from_fen(fen: &str) -> Board {
        fen.parse().expect("valid FEN in test")
    }

    const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
    const PERFT_POSITION_3: &str = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";
    const PERFT_POSITION_4: &str =
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1";

    #[test]
    fn is_square_attacked_spot_checks() {
        // (fen, square, attacking color, expected, what it shows)
        let cases = [
            (STARTING_FEN, "e3", Color::White, true, "d2/f2 pawns"),
            (STARTING_FEN, "e6", Color::Black, true, "d7/f7 pawns"),
            (STARTING_FEN, "e4", Color::White, false, "out of reach"),
            (STARTING_FEN, "e4", Color::Black, false, "out of reach"),
            (
                STARTING_FEN,
                "f3",
                Color::White,
                true,
                "g1 knight, e2/g2 pawns",
            ),
            (
                STARTING_FEN,
                "f6",
                Color::Black,
                true,
                "g8 knight, e7/g7 pawns",
            ),
            // Pawn direction, both colors.
            (
                "4k3/8/8/8/4P3/8/8/4K3 w - - 0 1",
                "d5",
                Color::White,
                true,
                "white pawn captures up",
            ),
            (
                "4k3/8/8/8/4P3/8/8/4K3 w - - 0 1",
                "d3",
                Color::White,
                false,
                "white pawn not down",
            ),
            (
                "4k3/8/8/8/4P3/8/8/4K3 w - - 0 1",
                "e5",
                Color::White,
                false,
                "pawn push is not an attack",
            ),
            (
                "4k3/8/8/4p3/8/8/8/4K3 w - - 0 1",
                "d4",
                Color::Black,
                true,
                "black pawn captures down",
            ),
            (
                "4k3/8/8/4p3/8/8/8/4K3 w - - 0 1",
                "d6",
                Color::Black,
                false,
                "black pawn not up",
            ),
            // Own blockers: defended, but nothing behind.
            (
                "4k3/8/8/8/8/8/P7/R3K3 w - - 0 1",
                "a2",
                Color::White,
                true,
                "rook defends own pawn",
            ),
            (
                "4k3/8/8/8/8/8/P7/R3K3 w - - 0 1",
                "a3",
                Color::White,
                false,
                "rook blocked by own pawn",
            ),
            (
                "4k3/8/8/8/8/8/1N6/Q3K3 w - - 0 1",
                "b2",
                Color::White,
                true,
                "queen defends own knight",
            ),
            (
                "4k3/8/8/8/8/8/1N6/Q3K3 w - - 0 1",
                "c3",
                Color::White,
                false,
                "queen blocked by own knight",
            ),
            // Enemy blockers: attacked, but nothing behind.
            (
                "4k3/8/8/8/p7/8/8/R3K3 w - - 0 1",
                "a4",
                Color::White,
                true,
                "rook attacks enemy pawn",
            ),
            (
                "4k3/8/8/8/p7/8/8/R3K3 w - - 0 1",
                "a5",
                Color::White,
                false,
                "rook blocked by enemy pawn",
            ),
            // Queen moves as both rook and bishop, but not as a knight.
            (
                "4k3/8/8/8/3Q4/8/8/4K3 w - - 0 1",
                "d8",
                Color::White,
                true,
                "queen along file",
            ),
            (
                "4k3/8/8/8/3Q4/8/8/4K3 w - - 0 1",
                "h8",
                Color::White,
                true,
                "queen along diagonal",
            ),
            (
                "4k3/8/8/8/3Q4/8/8/4K3 w - - 0 1",
                "e6",
                Color::White,
                false,
                "queen has no knight jump",
            ),
            // King reach.
            (
                "4k3/8/8/8/8/8/8/4K3 w - - 0 1",
                "d2",
                Color::White,
                true,
                "king adjacent",
            ),
            (
                "4k3/8/8/8/8/8/8/4K3 w - - 0 1",
                "d3",
                Color::White,
                false,
                "king two away",
            ),
        ];
        for (fen, square, by, expected, why) in cases {
            assert_eq!(
                from_fen(fen).is_square_attacked(sq(square), by),
                expected,
                "{square} attacked by {by} ({why}) in {fen}"
            );
        }
    }

    #[test]
    fn attackers_to_finds_every_attacker() {
        let board = Board::starting_position();
        let cases = [
            ("f3", Color::White, vec!["e2", "g2", "g1"]),
            ("e3", Color::White, vec!["d2", "f2"]),
            ("c3", Color::White, vec!["b1", "b2", "d2"]),
            ("e4", Color::White, vec![]),
            ("e4", Color::Black, vec![]),
        ];
        for (target, by, attackers) in cases {
            let expected = attackers
                .iter()
                .fold(Bitboard::EMPTY, |acc, &name| acc | sq(name).bitboard());
            let actual = board.attackers_to(sq(target), by);
            assert_eq!(
                actual, expected,
                "attackers of {target} by {by}\nexpected:\n{expected}\nactual:\n{actual}"
            );
        }
    }

    /// Forward route: ask every `by` piece what it attacks and keep the ones that
    /// hit the target. Independent of `attackers_to`, which looks outward from the target.
    fn attackers_brute_force(board: &Board, target: Square, by: Color) -> Bitboard {
        let occupied = board.all_occupancy();
        let mut attackers = Bitboard::EMPTY;
        for index in 0..Square::COUNT {
            let from = Square::from_index(index).expect("index in 0..64");
            let Some(piece) = board.piece_at(from) else {
                continue;
            };
            if piece.color != by {
                continue;
            }
            let attack_set = match piece.piece_type {
                PieceType::Pawn => attacks::pawn_attacks(by, from),
                PieceType::Knight => attacks::knight_attacks(from),
                PieceType::Bishop => attacks::bishop_attacks(from, occupied),
                PieceType::Rook => attacks::rook_attacks(from, occupied),
                PieceType::Queen => attacks::queen_attacks(from, occupied),
                PieceType::King => attacks::king_attacks(from),
            };
            if attack_set.contains(target) {
                attackers |= from.bitboard();
            }
        }
        attackers
    }

    #[test]
    fn attackers_to_matches_brute_force() {
        for fen in [STARTING_FEN, KIWIPETE, PERFT_POSITION_3, PERFT_POSITION_4] {
            let board = from_fen(fen);
            for index in 0..Square::COUNT {
                let target = Square::from_index(index).expect("index in 0..64");
                for by in Color::ALL {
                    let expected = attackers_brute_force(&board, target, by);
                    let actual = board.attackers_to(target, by);
                    assert_eq!(
                        actual, expected,
                        "attackers of {target} by {by} in {fen}\n\
                         expected:\n{expected}\nactual:\n{actual}"
                    );
                    assert_eq!(
                        board.is_square_attacked(target, by),
                        !expected.is_empty(),
                        "{target} attacked by {by} in {fen}"
                    );
                }
            }
        }
    }

    #[test]
    fn in_check_detects_each_side() {
        // (fen, white in check, black in check, what it shows)
        let cases = [
            (STARTING_FEN, false, false, "starting position"),
            (
                "4k3/8/8/8/8/8/8/r3K3 w - - 0 1",
                true,
                false,
                "rook along the rank",
            ),
            (
                "4k3/8/8/8/8/8/8/r1N1K3 w - - 0 1",
                false,
                false,
                "rook check blocked",
            ),
            (
                "4k3/8/3N4/8/8/8/8/4K3 b - - 0 1",
                false,
                true,
                "knight check",
            ),
            ("4k3/3P4/8/8/8/8/8/4K3 b - - 0 1", false, true, "pawn check"),
            (
                "4k3/8/8/8/8/8/8/8 w - - 0 1",
                false,
                false,
                "white king missing",
            ),
        ];
        for (fen, white, black, why) in cases {
            let board = from_fen(fen);
            assert_eq!(
                board.is_in_check(Color::White),
                white,
                "White in check ({why}) in {fen}"
            );
            assert_eq!(
                board.is_in_check(Color::Black),
                black,
                "Black in check ({why}) in {fen}"
            );
        }
    }

    #[test]
    fn empty_board_is_never_in_check() {
        let board = Board::empty();
        for color in Color::ALL {
            assert!(!board.is_in_check(color), "{color} on empty board");
        }
    }
}
