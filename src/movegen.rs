//! Pseudo-legal move generation: [`MoveList`] and [`Board::pseudo_legal_moves`]

use crate::attacks::{
    bishop_attacks, king_attacks, knight_attacks, pawn_attacks, queen_attacks, rook_attacks,
};
use crate::board::Board;
use crate::types::{Bitboard, CastleSide, Color, Move, MoveKind, Piece, PieceType, Square};
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

    pub fn retain(&mut self, mut keep: impl FnMut(Move) -> bool) {
        let mut kept = 0;

        for i in 0..self.len {
            if keep(self.moves[i]) {
                self.moves[kept] = self.moves[i];
                kept += 1;
            }
        }

        self.len = kept;
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
    /// Fills `moves` with the side to move's pseudo-legal moves, clearing it first.
    ///
    /// Generates moves for every piece type: ordinary moves and captures, pawn double
    /// pushes, en passant, promotions (one move per piece in [`PieceType::PROMOTIONS`])
    /// and castling. The order of the moves is unspecified.
    ///
    /// The moves are "pseudo-legal": they follow how each piece moves but may leave the
    /// mover's own king in check. Castling is the exception: the king's start, transit and
    /// destination squares are already checked for attacks, since a filter that only
    /// looks at the position after the move can't see the first two.
    ///
    /// Assumes a position that passes [`Board::validate`]. In particular, a castling right
    /// is trusted to mean the king and rook are on their home squares (debug builds assert it).
    pub fn pseudo_legal_moves(&self, moves: &mut MoveList) {
        moves.clear();

        self.push_piece_moves(PieceType::King, king_attacks, moves);
        self.push_piece_moves(PieceType::Knight, knight_attacks, moves);

        let blockers = self.all_occupancy();
        self.push_piece_moves(
            PieceType::Bishop,
            |from| bishop_attacks(from, blockers),
            moves,
        );
        self.push_piece_moves(PieceType::Rook, |from| rook_attacks(from, blockers), moves);
        self.push_piece_moves(
            PieceType::Queen,
            |from| queen_attacks(from, blockers),
            moves,
        );
        self.push_pawn_moves(moves);
        self.push_castling_moves(moves);
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

    fn push_pawn_moves(&self, moves: &mut MoveList) {
        let us = self.side_to_move();
        let enemies = self.occupancy(us.opposite());
        let empty = !self.all_occupancy();
        let ep_square = self.en_passant_square();

        let (forward, double_push_rank, promotion_rank): (
            fn(Bitboard) -> Bitboard,
            Bitboard,
            Bitboard,
        ) = match us {
            Color::White => (Bitboard::north, Bitboard::RANK_3, Bitboard::RANK_8),
            Color::Black => (Bitboard::south, Bitboard::RANK_6, Bitboard::RANK_1),
        };

        for from in self.pieces(us, PieceType::Pawn) {
            let single = forward(from.bitboard()) & empty;
            let double = forward(single & double_push_rank) & empty;

            let attacks = pawn_attacks(us, from);
            let captures = attacks & enemies;

            for to in single | captures {
                Self::push_pawn_move(from, to, promotion_rank, moves);
            }

            for to in double {
                moves.push(Move::new(from, to, MoveKind::DoublePush));
            }

            if let Some(ep) = ep_square
                && attacks.contains(ep)
            {
                moves.push(Move::new(from, ep, MoveKind::EnPassant));
            }
        }
    }

    fn push_pawn_move(from: Square, to: Square, promotion_rank: Bitboard, moves: &mut MoveList) {
        if promotion_rank.contains(to) {
            for piece_type in PieceType::PROMOTIONS {
                moves.push(Move::new(from, to, MoveKind::Promotion(piece_type)));
            }
        } else {
            moves.push(Move::new(from, to, MoveKind::Normal));
        }
    }

    fn push_castling_moves(&self, moves: &mut MoveList) {
        let us = self.side_to_move();
        let them = us.opposite();
        let occupied = self.all_occupancy();

        for side in CastleSide::ALL {
            let path = side.path(us);

            if self.castling_rights().has(us, side)
                && !path.must_be_empty.overlaps(occupied)
                && !path
                    .must_be_safe
                    .into_iter()
                    .any(|s| self.is_square_attacked(s, them))
            {
                debug_assert_eq!(
                    Some(Piece::new(us, PieceType::King)),
                    self.piece_at(path.king_from),
                    "{us} holds the {side} right but its king is not on {}",
                    path.king_from
                );
                debug_assert_eq!(
                    Some(Piece::new(us, PieceType::Rook)),
                    self.piece_at(path.rook_from),
                    "{us} holds the {side} right but its rook is not on {}",
                    path.rook_from
                );

                moves.push(Move::new(
                    path.king_from,
                    path.king_to,
                    MoveKind::Castle(side),
                ));
            }
        }
    }

    /// Checks all moves to see if they will end with the player who moved in check.
    pub fn legal_moves(&self, moves: &mut MoveList) {
        self.pseudo_legal_moves(moves);
        let us = self.side_to_move();

        moves.retain(|mv| {
            let mut next = *self;
            next.make_move(mv);
            !next.is_in_check(us)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;
    use PieceType::{Bishop, King, Knight, Pawn, Queen, Rook};

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
    fn pseudo_legal_moves_leapers() {
        let cases: Cases = &[
            // -- leapers -----
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
            (
                "Kiwipete king (castles both ways)",
                KIWIPETE,
                King,
                &["e1c1", "e1d1", "e1f1", "e1g1"],
            ),
        ];

        for &(name, fen, piece_type, expected) in cases {
            let actual = generated(fen, piece_type);
            assert_eq!(expected, actual, "{name}");
        }
    }

    #[test]
    fn pseudo_legal_moves_sliders() {
        let cases: Cases = &[
            (
                "Start, white bishops boxed in",
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                Bishop,
                &[],
            ),
            (
                "Start, white rooks boxed in",
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                Rook,
                &[],
            ),
            (
                "Start, white queen boxed in",
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                Queen,
                &[],
            ),
            (
                "Start, black queen boxed in",
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1",
                Queen,
                &[],
            ),
            (
                "Lone rook on an empty board",
                "8/8/8/8/8/8/8/R7 w - - 0 1",
                Rook,
                &[
                    "a1a2", "a1a3", "a1a4", "a1a5", "a1a6", "a1a7", "a1a8", "a1b1", "a1c1", "a1d1",
                    "a1e1", "a1f1", "a1g1", "a1h1",
                ],
            ),
            (
                "Bishop: own piece stops the ray, enemy is captured",
                "8/8/1p3P2/8/3B4/8/8/8 w - - 0 1",
                Bishop,
                &[
                    "d4a1", "d4b2", "d4b6", "d4c3", "d4c5", "d4e3", "d4e5", "d4f2", "d4g1",
                ],
            ),
            (
                "Rook: own piece stops the ray, enemy is captured",
                "8/8/8/3p4/8/8/3R1P2/8 w - - 0 1",
                Rook,
                &[
                    "d2a2", "d2b2", "d2c2", "d2d1", "d2d3", "d2d4", "d2d5", "d2e2",
                ],
            ),
            (
                "Black queen in a corner",
                "7q/8/8/8/8/8/8/8 b - - 0 1",
                Queen,
                &[
                    "h8a1", "h8a8", "h8b2", "h8b8", "h8c3", "h8c8", "h8d4", "h8d8", "h8e5", "h8e8",
                    "h8f6", "h8f8", "h8g7", "h8g8", "h8h1", "h8h2", "h8h3", "h8h4", "h8h5", "h8h6",
                    "h8h7",
                ],
            ),
            (
                "Kiwipete bishops (e2a6 captures)",
                KIWIPETE,
                Bishop,
                &[
                    "d2c1", "d2e3", "d2f4", "d2g5", "d2h6", "e2a6", "e2b5", "e2c4", "e2d1", "e2d3",
                    "e2f1",
                ],
            ),
            (
                "Kiwipete rooks",
                KIWIPETE,
                Rook,
                &["a1b1", "a1c1", "a1d1", "h1f1", "h1g1"],
            ),
            (
                "Kiwipete queen (f3f6 and f3h3 capture)",
                KIWIPETE,
                Queen,
                &[
                    "f3d3", "f3e3", "f3f4", "f3f5", "f3f6", "f3g3", "f3g4", "f3h3", "f3h5",
                ],
            ),
            (
                "Kiwipete, black bishops (a6e2 captures)",
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R b KQkq -",
                Bishop,
                &[
                    "a6b5", "a6b7", "a6c4", "a6c8", "a6d3", "a6e2", "g7f8", "g7h6",
                ],
            ),
            (
                "Kiwipete, black rooks (h-file stops at own pawn on h3)",
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R b KQkq -",
                Rook,
                &[
                    "a8b8", "a8c8", "a8d8", "h8f8", "h8g8", "h8h4", "h8h5", "h8h6", "h8h7",
                ],
            ),
            (
                "Kiwipete, black queen",
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R b KQkq -",
                Queen,
                &["e7c5", "e7d6", "e7d8", "e7f8"],
            ),
        ];

        for &(name, fen, piece_type, expected) in cases {
            let actual = generated(fen, piece_type);
            assert_eq!(expected, actual, "{name}");
        }
    }

    #[test]
    fn pseudo_legal_moves_pawns() {
        let cases: Cases = &[
            (
                "Start, white pawns",
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                Pawn,
                &[
                    "a2a3", "a2a4", "b2b3", "b2b4", "c2c3", "c2c4", "d2d3", "d2d4", "e2e3", "e2e4",
                    "f2f3", "f2f4", "g2g3", "g2g4", "h2h3", "h2h4",
                ],
            ),
            (
                "Start, black pawns",
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1",
                Pawn,
                &[
                    "a7a5", "a7a6", "b7b5", "b7b6", "c7c5", "c7c6", "d7d5", "d7d6", "e7e5", "e7e6",
                    "f7f5", "f7f6", "g7g5", "g7g6", "h7h5", "h7h6",
                ],
            ),
            (
                "White pawn blocked directly, no jump",
                "4k3/8/8/8/8/4n3/4P3/4K3 w - - 0 1",
                Pawn,
                &[],
            ),
            (
                "White pawn, second square blocked: single push only",
                "4k3/8/8/8/3p4/8/3P4/4K3 w - - 0 1",
                Pawn,
                &["d2d3"],
            ),
            (
                "Black pawn blocked directly, no jump",
                "4k3/3p4/3N4/8/8/8/8/4K3 b - - 0 1",
                Pawn,
                &[],
            ),
            (
                "Black pawn, second square blocked: single push only",
                "4k3/3p4/8/3N4/8/8/8/4K3 b - - 0 1",
                Pawn,
                &["d7d6"],
            ),
            (
                "White pawn: push doesn't capture, own piece not captured",
                "4k3/8/2pnN3/3P4/8/8/8/4K3 w - - 0 1",
                Pawn,
                &["d5c6"],
            ),
            (
                "Black pawn: push doesn't capture, own piece not captured",
                "4k3/8/8/8/3p4/2PNn3/8/4K3 b - - 0 1",
                Pawn,
                &["d4c3"],
            ),
            (
                "Edge-file pawns capture inward only",
                "4k3/8/8/8/8/1p4p1/P6P/4K3 w - - 0 1",
                Pawn,
                &["a2a3", "a2a4", "a2b3", "h2g3", "h2h3", "h2h4"],
            ),
            (
                "White promotion by push and capture",
                "2n1k3/1P6/8/8/8/8/8/4K3 w - - 0 1",
                Pawn,
                &[
                    "b7b8b", "b7b8n", "b7b8q", "b7b8r", "b7c8b", "b7c8n", "b7c8q", "b7c8r",
                ],
            ),
            (
                "Black promotion by push and both captures",
                "4k3/8/8/8/8/8/6p1/4KN1N b - - 0 1",
                Pawn,
                &[
                    "g2f1b", "g2f1n", "g2f1q", "g2f1r", "g2g1b", "g2g1n", "g2g1q", "g2g1r",
                    "g2h1b", "g2h1n", "g2h1q", "g2h1r",
                ],
            ),
            (
                "Blocked promotion, nothing to capture",
                "n3k3/P7/8/8/8/8/8/4K3 w - - 0 1",
                Pawn,
                &[],
            ),
            (
                "White en passant",
                "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1",
                Pawn,
                &["e5d6", "e5e6"],
            ),
            (
                "Black en passant",
                "4k3/8/8/8/3Pp3/8/8/4K3 b - d3 0 1",
                Pawn,
                &["e4d3", "e4e3"],
            ),
            (
                "En passant from both sides",
                "4k3/8/8/2PpP3/8/8/8/4K3 w - d6 0 1",
                Pawn,
                &["c5c6", "c5d6", "e5d6", "e5e6"],
            ),
            (
                "En passant square no pawn can reach",
                "4k3/8/8/3p3P/8/8/8/4K3 w - d6 0 1",
                Pawn,
                &["h5h6"],
            ),
            (
                "Kiwipete pawns (d5e6 and g2h3 capture)",
                KIWIPETE,
                Pawn,
                &[
                    "a2a3", "a2a4", "b2b3", "d5d6", "d5e6", "g2g3", "g2g4", "g2h3",
                ],
            ),
            (
                "Kiwipete, black pawns (b4c3, e6d5 and h3g2 capture)",
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R b KQkq -",
                Pawn,
                &[
                    "b4b3", "b4c3", "c7c5", "c7c6", "d7d6", "e6d5", "g6g5", "h3g2",
                ],
            ),
            (
                "Perft 4 pawns (a7 blocked from promoting)",
                PERFT4,
                Pawn,
                &[
                    "c4c5", "d2d3", "d2d4", "e4e5", "g2g3", "g2g4", "h2h3", "h2h4",
                ],
            ),
            (
                "Perft 4, black pawns (b2 promotes by push and capture)",
                "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 b kq -",
                Pawn,
                &[
                    "b2a1b", "b2a1n", "b2a1q", "b2a1r", "b2b1b", "b2b1n", "b2b1q", "b2b1r", "c7c5",
                    "c7c6", "d7d5", "d7d6", "g7h6",
                ],
            ),
        ];

        for &(name, fen, piece_type, expected) in cases {
            let actual = generated(fen, piece_type);
            assert_eq!(expected, actual, "{name}");
        }
    }

    /// The kind of the generated move whose UCI string is `uci`, or `None` if it wasn't generated.
    fn kind_of(fen: &str, uci: &str) -> Option<MoveKind> {
        let board: Board = fen.parse().expect("valid fen in test");
        let mut moves = MoveList::new();

        board.pseudo_legal_moves(&mut moves);
        moves
            .iter()
            .find(|mv| mv.to_string() == uci)
            .map(|mv| mv.kind())
    }

    #[test]
    fn pseudo_legal_moves_pawn_kinds() {
        type KindCases<'a> = &'a [(&'a str, &'a str, &'a str, Option<MoveKind>)];

        let cases: KindCases = &[
            (
                "White single push",
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                "e2e3",
                Some(MoveKind::Normal),
            ),
            (
                "White double push",
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                "e2e4",
                Some(MoveKind::DoublePush),
            ),
            (
                "Black single push",
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1",
                "e7e6",
                Some(MoveKind::Normal),
            ),
            (
                "Black double push",
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1",
                "e7e5",
                Some(MoveKind::DoublePush),
            ),
            (
                "Ordinary capture is Normal",
                "4k3/8/2pnN3/3P4/8/8/8/4K3 w - - 0 1",
                "d5c6",
                Some(MoveKind::Normal),
            ),
            (
                "White en passant",
                "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1",
                "e5d6",
                Some(MoveKind::EnPassant),
            ),
            (
                "Push beside an en passant chance stays Normal",
                "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1",
                "e5e6",
                Some(MoveKind::Normal),
            ),
            (
                "Black en passant",
                "4k3/8/8/8/3Pp3/8/8/4K3 b - d3 0 1",
                "e4d3",
                Some(MoveKind::EnPassant),
            ),
            (
                "White promotion by push",
                "2n1k3/1P6/8/8/8/8/8/4K3 w - - 0 1",
                "b7b8q",
                Some(MoveKind::Promotion(Queen)),
            ),
            (
                "White promotion by capture",
                "2n1k3/1P6/8/8/8/8/8/4K3 w - - 0 1",
                "b7c8n",
                Some(MoveKind::Promotion(Knight)),
            ),
            (
                "Black promotion by push",
                "4k3/8/8/8/8/8/6p1/4KN1N b - - 0 1",
                "g2g1b",
                Some(MoveKind::Promotion(Bishop)),
            ),
            (
                "Black promotion by capture",
                "4k3/8/8/8/8/8/6p1/4KN1N b - - 0 1",
                "g2f1r",
                Some(MoveKind::Promotion(Rook)),
            ),
            (
                "Pawn can't move three squares",
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                "e2e5",
                None,
            ),
        ];

        for &(name, fen, uci, expected) in cases {
            let actual = kind_of(fen, uci);
            assert_eq!(expected, actual, "{name}: {uci}");
        }
    }

    #[test]
    fn pseudo_legal_moves_castling() {
        let cases: Cases = &[
            (
                "White castles both ways (rooks attacked down open files don't matter)",
                "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
                King,
                &["e1c1", "e1d1", "e1d2", "e1e2", "e1f1", "e1f2", "e1g1"],
            ),
            (
                "Black castles both ways (rooks attacked down open files don't matter)",
                "r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1",
                King,
                &["e8c8", "e8d7", "e8d8", "e8e7", "e8f7", "e8f8", "e8g8"],
            ),
            (
                "White without rights",
                "r3k2r/8/8/8/8/8/8/R3K2R w - - 0 1",
                King,
                &["e1d1", "e1d2", "e1e2", "e1f1", "e1f2"],
            ),
            (
                "Black without rights",
                "r3k2r/8/8/8/8/8/8/R3K2R b - - 0 1",
                King,
                &["e8d7", "e8d8", "e8e7", "e8f7", "e8f8"],
            ),
            (
                "White holds kingside right only",
                "r3k2r/8/8/8/8/8/8/R3K2R w Kq - 0 1",
                King,
                &["e1d1", "e1d2", "e1e2", "e1f1", "e1f2", "e1g1"],
            ),
            (
                "Black holds queenside right only",
                "r3k2r/8/8/8/8/8/8/R3K2R b Kq - 0 1",
                King,
                &["e8c8", "e8d7", "e8d8", "e8e7", "e8f7", "e8f8"],
            ),
            (
                "White kingside blocked by own knight on g1",
                "r3k2r/8/8/8/8/8/8/R3K1NR w KQkq - 0 1",
                King,
                &["e1c1", "e1d1", "e1d2", "e1e2", "e1f1", "e1f2"],
            ),
            (
                "Black kingside blocked by own bishop on f8",
                "r3kb1r/8/8/8/8/8/8/R3K2R b KQkq - 0 1",
                King,
                &["e8c8", "e8d7", "e8d8", "e8e7", "e8f7"],
            ),
            (
                "White queenside blocked by enemy knight on b1",
                "r3k2r/8/8/8/8/8/8/Rn2K2R w KQkq - 0 1",
                King,
                &["e1d1", "e1d2", "e1e2", "e1f1", "e1f2", "e1g1"],
            ),
            (
                "Black queenside blocked by enemy knight on b8",
                "rN2k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1",
                King,
                &["e8d7", "e8d8", "e8e7", "e8f7", "e8f8", "e8g8"],
            ),
            (
                "White in check can't castle",
                "r3k2r/8/8/8/4r3/8/8/R3K2R w KQkq - 0 1",
                King,
                &["e1d1", "e1d2", "e1e2", "e1f1", "e1f2"],
            ),
            (
                "Black in check can't castle",
                "r3k2r/8/8/4R3/8/8/8/R3K2R b KQkq - 0 1",
                King,
                &["e8d7", "e8d8", "e8e7", "e8f7", "e8f8"],
            ),
            (
                "White transit square f1 attacked",
                "r3k2r/8/8/8/5r2/8/8/R3K2R w KQkq - 0 1",
                King,
                &["e1c1", "e1d1", "e1d2", "e1e2", "e1f1", "e1f2"],
            ),
            (
                "Black transit square d8 attacked",
                "r3k2r/8/8/8/3R4/8/8/R3K2R b KQkq - 0 1",
                King,
                &["e8d7", "e8d8", "e8e7", "e8f7", "e8f8", "e8g8"],
            ),
            (
                "White destination g1 attacked",
                "r3k2r/8/8/8/6r1/8/8/R3K2R w KQkq - 0 1",
                King,
                &["e1c1", "e1d1", "e1d2", "e1e2", "e1f1", "e1f2"],
            ),
            (
                "Black destination c8 attacked",
                "r3k2r/8/8/8/2R5/8/8/R3K2R b KQkq - 0 1",
                King,
                &["e8d7", "e8d8", "e8e7", "e8f7", "e8f8", "e8g8"],
            ),
            (
                "White b1 attacked but empty: queenside still allowed",
                "r3k2r/8/8/8/1r6/8/8/R3K2R w KQkq - 0 1",
                King,
                &["e1c1", "e1d1", "e1d2", "e1e2", "e1f1", "e1f2", "e1g1"],
            ),
            (
                "Black b8 attacked but empty: queenside still allowed",
                "r3k2r/8/8/8/1R6/8/8/R3K2R b KQkq - 0 1",
                King,
                &["e8c8", "e8d7", "e8d8", "e8e7", "e8f7", "e8f8", "e8g8"],
            ),
            (
                "White pawn on e2 attacks d1 and f1: neither side",
                "r3k2r/8/8/8/8/8/4p3/R3K2R w KQkq - 0 1",
                King,
                &["e1d1", "e1d2", "e1e2", "e1f1", "e1f2"],
            ),
            (
                "Black pawn on e7 attacks d8 and f8: neither side",
                "r3k2r/4P3/8/8/8/8/8/R3K2R b KQkq - 0 1",
                King,
                &["e8d7", "e8d8", "e8e7", "e8f7", "e8f8"],
            ),
            (
                "Kiwipete, black king castles both ways",
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R b KQkq -",
                King,
                &["e8c8", "e8d8", "e8f8", "e8g8"],
            ),
        ];

        for &(name, fen, piece_type, expected) in cases {
            let actual = generated(fen, piece_type);
            assert_eq!(expected, actual, "{name}");
        }
    }

    #[test]
    fn pseudo_legal_moves_castle_kinds() {
        use CastleSide::{KingSide, QueenSide};
        type KindCases<'a> = &'a [(&'a str, &'a str, &'a str, Option<MoveKind>)];

        let cases: KindCases = &[
            (
                "White kingside",
                "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
                "e1g1",
                Some(MoveKind::Castle(KingSide)),
            ),
            (
                "White queenside",
                "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
                "e1c1",
                Some(MoveKind::Castle(QueenSide)),
            ),
            (
                "Black kingside",
                "r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1",
                "e8g8",
                Some(MoveKind::Castle(KingSide)),
            ),
            (
                "Black queenside",
                "r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1",
                "e8c8",
                Some(MoveKind::Castle(QueenSide)),
            ),
            (
                "One-step king move beside a castle stays Normal",
                "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
                "e1f1",
                Some(MoveKind::Normal),
            ),
            (
                "No right, no castle",
                "r3k2r/8/8/8/8/8/8/R3K2R w - - 0 1",
                "e1g1",
                None,
            ),
        ];

        for &(name, fen, uci, expected) in cases {
            let actual = kind_of(fen, uci);
            assert_eq!(expected, actual, "{name}: {uci}");
        }
    }

    #[test]
    fn retain_keeps_matching_moves_in_order() {
        type Keep = fn(Move) -> bool;
        let moves = [
            mv("a1", "a2"),
            mv("b1", "b2"),
            mv("c1", "c2"),
            mv("d1", "d2"),
        ];

        let cases: &[(&str, Keep, Vec<Move>)] = &[
            ("keep all", |_| true, moves.to_vec()),
            ("keep none", |_| false, vec![]),
            (
                "keep files a and c",
                |m| m.from().file() % 2 == 0,
                vec![moves[0], moves[2]],
            ),
            ("keep only the last", |m| m.to().file() == 3, vec![moves[3]]),
        ];

        for (name, keep, expected) in cases {
            let mut move_list = MoveList::new();
            push_moves(&mut move_list, &moves);

            move_list.retain(*keep);

            assert_eq!(expected.as_slice(), move_list.as_slice(), "{name}");
        }
    }

    #[test]
    fn retain_on_an_empty_list() {
        let mut move_list = MoveList::new();
        move_list.retain(|_| true);
        assert!(move_list.is_empty());
    }

    #[test]
    fn retain_calls_keep_once_per_move_in_order() {
        // A closure that records what it sees is why `retain` takes `FnMut`:
        // it mutates `seen`, which an `Fn` closure couldn't.
        let moves = [mv("a1", "a2"), mv("b1", "b2"), mv("c1", "c2")];
        let mut move_list = MoveList::new();
        push_moves(&mut move_list, &moves);

        let mut seen = Vec::new();
        move_list.retain(|m| {
            seen.push(m);
            true
        });

        assert_eq!(moves.as_slice(), seen.as_slice());
    }

    /// The side to move's legal moves, as sorted UCI strings.
    fn legal(fen: &str) -> Vec<String> {
        let board = from_fen(fen);
        let mut moves = MoveList::new();

        board.legal_moves(&mut moves);
        let mut ucis: Vec<String> = moves.iter().map(|mv| mv.to_string()).collect();

        ucis.sort();
        ucis
    }

    #[test]
    fn legal_move_counts_match_perft_depth_1() {
        let cases = [
            ("start", STARTING_FEN, 20),
            ("Kiwipete", KIWIPETE, 48),
            ("perft 3", PERFT3, 14),
            ("perft 4", PERFT4, 6),
            ("perft 5", PERFT5, 44),
            ("perft 6", PERFT6, 46),
        ];

        for (name, fen, expected) in cases {
            let mut moves = MoveList::new();
            from_fen(fen).legal_moves(&mut moves);
            assert_eq!(expected, moves.len(), "{name}: {:?}", legal(fen));
        }
    }

    #[test]
    fn legal_moves_clears_the_list_first() {
        let mut moves = MoveList::new();
        push_moves(&mut moves, &[mv("a1", "a2"), mv("b1", "b2")]);

        from_fen(STARTING_FEN).legal_moves(&mut moves);

        assert_eq!(20, moves.len());
        assert!(!moves.contains(&mv("a1", "a2")), "stale moves are gone");
    }

    #[test]
    fn legal_moves_rows() {
        // The `PieceType` column of `Cases` is unused here, so a local row type.
        let cases: &[(&str, &str, &[&str])] = &[
            // pins
            (
                "White knight pinned on the e-file can't move",
                "4k3/4r3/8/8/8/8/4N3/4K3 w - - 0 1",
                &["e1d1", "e1d2", "e1f1", "e1f2"],
            ),
            (
                "Black knight pinned on the e-file can't move",
                "4k3/4n3/8/8/8/8/4R3/4K3 b - - 0 1",
                &["e8d7", "e8d8", "e8f7", "e8f8"],
            ),
            (
                "White rook pinned on the e-file moves along the pin",
                "4k3/4r3/8/8/8/8/4R3/4K3 w - - 0 1",
                &[
                    "e1d1", "e1d2", "e1f1", "e1f2", "e2e3", "e2e4", "e2e5", "e2e6", "e2e7",
                ],
            ),
            (
                "Black bishop pinned on a diagonal moves along the pin",
                "4k3/5b2/8/8/8/8/8/B3K3 b - - 0 1",
                &[
                    "e8d7", "e8d8", "e8e7", "e8f8", "f7a2", "f7b3", "f7c4", "f7d5", "f7e6", "f7g6",
                    "f7g8", "f7h5",
                ],
            ),
            // king safety
            (
                "White king can't step into attack or take a defended rook",
                "4k3/8/8/8/8/2b5/3r4/4K3 w - - 0 1",
                &["e1f1"],
            ),
            (
                "Black king can't step into attack or take a defended rook",
                "4k3/3R4/2B5/8/8/8/8/4K3 b - - 0 1",
                &["e8f8"],
            ),
            // check
            (
                "White in single check: capture, block or king move",
                "4k3/8/8/8/8/8/1B6/r3K3 w - - 0 1",
                &["b2a1", "b2c1", "e1d2", "e1e2", "e1f2"],
            ),
            (
                "Black in single check: capture, block or king move",
                "R3k3/1b6/8/8/8/8/8/4K3 b - - 0 1",
                &["b7a8", "b7c8", "e8d7", "e8e7", "e8f7"],
            ),
            (
                "White in double check: king moves only",
                "4k3/8/8/8/1b6/8/2N5/r3K3 w - - 0 1",
                &["e1e2", "e1f2"],
            ),
            (
                "Black in double check: king moves only",
                "R3k3/2n5/8/1B6/8/8/8/4K3 b - - 0 1",
                &["e8e7", "e8f7"],
            ),
            // en passant
            (
                "White en passant that exposes the king on the rank is illegal",
                "8/8/8/KPp4r/8/8/8/7k w - c6 0 2",
                &["a5a4", "a5a6", "a5b6", "b5b6"],
            ),
            (
                "Black en passant that exposes the king on the rank is illegal",
                "7K/8/8/8/R4pPk/8/8/8 b - g3 0 1",
                &["f4f3", "h4g3", "h4g4", "h4g5", "h4h3"],
            ),
            (
                "White en passant may capture the checking pawn",
                "4k3/8/8/3pP3/4K3/8/8/8 w - d6 0 2",
                &[
                    "e4d3", "e4d4", "e4d5", "e4e3", "e4f3", "e4f4", "e4f5", "e5d6",
                ],
            ),
            (
                "Black en passant may capture the checking pawn",
                "8/8/8/2k5/3Pp3/8/8/4K3 b - d3 0 1",
                &[
                    "c5b4", "c5b5", "c5b6", "c5c4", "c5c6", "c5d4", "c5d5", "c5d6", "e4d3",
                ],
            ),
            // no legal moves
            ("White checkmated", "6k1/8/8/8/8/8/5PPP/r5K1 w - - 0 1", &[]),
            ("Black checkmated", "R5k1/5ppp/8/8/8/8/8/6K1 b - - 0 1", &[]),
            ("White stalemated", "8/8/8/8/8/1q6/8/K6k w - - 0 1", &[]),
            ("Black stalemated", "7k/5Q2/6K1/8/8/8/8/8 b - - 0 1", &[]),
        ];

        for &(name, fen, expected) in cases {
            assert_eq!(expected, legal(fen), "{name}");
        }
    }

    #[test]
    fn legal_moves_lead_to_valid_positions() {
        // Invariant: every legal move from a valid position gives a valid
        // position. `validate` catches a mover left in check
        // (SideNotToMoveInCheck), a missed castling-rights update
        // (InvalidCastlingRights) and a bad en passant square.
        let fens = [STARTING_FEN, KIWIPETE, PERFT3, PERFT4, PERFT5, PERFT6];

        for fen in fens {
            let board = from_fen(fen);
            let mut moves = MoveList::new();
            board.legal_moves(&mut moves);

            for &mv in &moves {
                let mut next = board;
                next.make_move(mv);
                assert_eq!(Ok(()), next.is_consistent(), "{mv} from {fen}");
                if let Err(errors) = next.validate() {
                    panic!("{mv} from {fen} gives {}:\n{errors}", next.to_fen());
                }
            }
        }
    }
}
