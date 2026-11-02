//! Pseudo-legal move generation: [`MoveList`] and [`Board::pseudo_legal_moves`]

use crate::attacks::{king_attacks, knight_attacks};
use crate::board::Board;
use crate::types::{Bitboard, Move, MoveKind, PieceType, Square};
use std::{fmt, ops::Deref};

/// Maximum number of moves in a MoveList.
pub const MAX_MOVES: usize = 256;

/// Filler (default) value of a MoveList entry outside of len.
const FILLER: Move = Move::new(Square::A1, Square::A1, MoveKind::Normal);

/// An ordered ArrayVec of Moves.
#[derive(Clone)]
pub struct MoveList {
    moves: [Move; MAX_MOVES],
    len: usize,
}

impl MoveList {
    pub const fn new() -> Self {
        Self {
            moves: [FILLER; MAX_MOVES],
            len: 0,
        }
    }

    pub fn push(&mut self, mv: Move) {
        assert!(
            self.len < MAX_MOVES,
            "attempted to push into full `MoveList`"
        );

        self.moves[self.len] = mv;
        self.len += 1;
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn as_slice(&self) -> &[Move] {
        &self.moves[..self.len]
    }
}

impl Default for MoveList {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for MoveList {
    type Target = [Move];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<'a> IntoIterator for &'a MoveList {
    type Item = &'a Move;
    type IntoIter = std::slice::Iter<'a, Move>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl PartialEq for MoveList {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Eq for MoveList {}

impl fmt::Debug for MoveList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl Board {
    pub fn pseudo_legal_moves(&self, moves: &mut MoveList) {
        moves.clear();

        self.push_piece_moves(PieceType::King, king_attacks, moves);
        self.push_piece_moves(PieceType::Knight, knight_attacks, moves);
        // TODO: slider moves
        // TODO: pawn moves
        // TODO: castling moves
    }

    fn push_piece_moves(
        &self,
        piece_type: PieceType,
        attacks_from: impl Fn(Square) -> Bitboard,
        moves: &mut MoveList,
    ) {
        let us = self.side_to_move();
        let own = self.occupancy(us);

        for from in self.pieces(us, piece_type) {
            let targets = attacks_from(from) & !own;

            for to in targets {
                moves.push(Move::new(from, to, MoveKind::Normal));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::KIWIPETE;
    use PieceType::{King, Knight};

    fn mv(from: &str, to: &str) -> Move {
        Move::new(
            from.parse().expect("valid squares in test"),
            to.parse().expect("valid squares in test"),
            MoveKind::Normal,
        )
    }

    fn push_moves(move_list: &mut MoveList, moves: &[Move]) {
        for m in moves {
            move_list.push(*m);
        }
    }

    #[test]
    fn new_is_empty() {
        let cases = [("new", MoveList::new()), ("default", MoveList::default())];

        for (name, move_list) in cases {
            assert_eq!(move_list.len(), 0, "{name} has len of 0");
            assert_eq!(move_list.as_slice(), &[], "{name} as_slice() is &[]");
            assert!(move_list.is_empty(), "{name} is_empty() is true");
        }
    }

    #[test]
    fn push_keeps_order() {
        let mut move_list = MoveList::new();
        let moves = [mv("a1", "a2"), mv("a2", "a3"), mv("a3", "a4")];

        push_moves(&mut move_list, &moves);

        assert_eq!(&moves, move_list.as_slice());
    }

    #[test]
    fn iterates_in_push_order() {
        let mut move_list = MoveList::new();
        let moves = [mv("a1", "a2"), mv("a2", "a3"), mv("a3", "a4")];

        push_moves(&mut move_list, &moves);

        let mut actual: Vec<Move> = Vec::new();

        for &m in &move_list {
            actual.push(m);
        }

        assert_eq!(&moves, actual.as_slice());
    }

    #[test]
    fn clears_empties_and_reuses() {
        let mut move_list = MoveList::new();
        let moves = [mv("a1", "a2"), mv("a2", "a3"), mv("a3", "a4")];

        push_moves(&mut move_list, &moves);

        move_list.clear();

        assert!(move_list.is_empty(), "cleared move list is empty");

        push_moves(&mut move_list, &moves);

        assert_eq!(&moves, move_list.as_slice(), "cleared move list is reused");
    }

    #[test]
    fn fills_to_capacity() {
        let mut move_list = MoveList::new();
        let m = mv("a1", "a2");

        for _ in 0..MAX_MOVES {
            move_list.push(m);
        }

        assert_eq!(256, move_list.len());
    }

    #[test]
    #[should_panic(expected = "attempted to push into full `MoveList`")]
    fn push_past_capacity_panics() {
        let mut move_list = MoveList::new();
        let m = mv("a1", "a2");

        for _ in 0..(MAX_MOVES + 1) {
            move_list.push(m);
        }
    }

    #[test]
    fn equality_ignores_stale_slots() {
        let mut cleared_move_list = MoveList::new();
        let moves = [mv("a1", "a2"), mv("a2", "a3"), mv("a3", "a4")];

        push_moves(&mut cleared_move_list, &moves);
        cleared_move_list.clear();
        cleared_move_list.push(moves[0]);

        let mut new_move_list = MoveList::new();
        new_move_list.push(moves[0]);

        assert_eq!(new_move_list, cleared_move_list);
    }

    #[test]
    fn debug_shows_only_live_moves() {
        let mut move_list = MoveList::new();
        let moves = [mv("a1", "a2"), mv("a2", "a3"), mv("a3", "a4")];

        push_moves(&mut move_list, &moves);
        move_list.clear();
        move_list.push(moves[0]);

        assert_eq!(format!("{move_list:?}"), format!("{:?}", [moves[0]]));
    }

    /// The side to move's pseudo-legal moves made by `piece_type`, as sorted UCI strings.
    fn generated(fen: &str, piece_type: PieceType) -> Vec<String> {
        let board: Board = fen.parse().expect("valid fen in test");
        let mut moves = MoveList::new();

        board.pseudo_legal_moves(&mut moves);
        let mut ucis: Vec<String> = moves
            .iter()
            .filter(|mv| {
                board
                    .piece_at(mv.from())
                    .is_some_and(|piece| piece.piece_type == piece_type)
            })
            .map(|mv| mv.to_string())
            .collect();

        ucis.sort();
        ucis
    }

    type Cases<'a> = &'a [(&'a str, &'a str, PieceType, &'a [&'a str])];

    #[test]
    fn pseudo_legal_moves() {
        let cases: Cases = &[
            (
                "Start, white knights",
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                Knight,
                &["b1a3", "b1c3", "g1f3", "g1h3"],
            ),
            (
                "Start, black knights",
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1",
                Knight,
                &["b8a6", "b8c6", "g8f6", "g8h6"],
            ),
            (
                "Start, king boxed in",
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                King,
                &[],
            ),
            (
                "Corner knight",
                "8/8/8/8/8/8/8/N7 w - - 0 1",
                Knight,
                &["a1b3", "a1c2"],
            ),
            (
                "Own piece excluded, enemy included",
                "8/8/8/3p4/2P5/4N3/8/8 w - - 0 1",
                Knight,
                &["e3c2", "e3d1", "e3d5", "e3f1", "e3f5", "e3g2", "e3g4"],
            ),
            (
                "Black king in a corner",
                "7k/8/8/8/8/8/8/8 b - - 0 1",
                King,
                &["h8g7", "h8g8", "h8h7"],
            ),
            (
                "King may step into attack (pseudo-legal)",
                "8/8/8/8/8/8/r7/4K3 w - - 0 1",
                King,
                &["e1d1", "e1d2", "e1e2", "e1f1", "e1f2"],
            ),
            ("No king, no panic", "8/8/8/8/8/8/8/N7 w - - 0 1", King, &[]),
            (
                "Kiwipete knights (captures on d7, f7, g6)",
                KIWIPETE,
                Knight,
                &[
                    "c3a4", "c3b1", "c3b5", "c3d1", "e5c4", "e5c6", "e5d3", "e5d7", "e5f7", "e5g4",
                    "e5g6",
                ],
            ),
            ("Kiwipete king", KIWIPETE, King, &["e1d1", "e1f1"]),
        ];

        for &(name, fen, piece_type, expected) in cases {
            let actual = generated(fen, piece_type);
            assert_eq!(expected, actual, "{name}");
        }
    }
}
