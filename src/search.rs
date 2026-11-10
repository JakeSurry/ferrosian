//! Choosing a move to play.
//!
//! Currently this is just [`random_move`]: a uniformly random legal move.
//! Real search (alpha-beta, iterative deepening) will be added here later.
//!
//! Search only returns its result, and the caller turns it into UCI
//! output. A `None` move (no legal moves) becomes `bestmove 0000`. The caller also
//! passes in the [`Rng`](crate::rng::Rng), so results are reproducible for a given
//! seed.

use crate::board::Board;
use crate::movegen::MoveList;
use crate::rng::Rng;
use crate::types::Move;

/// Produce a random legal move in `board`. Assumes a valid position.
pub fn random_move(board: &Board, rng: &mut Rng) -> Option<Move> {
    let mut moves = MoveList::new();
    board.legal_moves(&mut moves);
    rng.choose(&moves).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_helpers::{
            KIWIPETE, PERFT3, PERFT4, PERFT5, PERFT6, STARTING_FEN, TEST_SEED, from_fen, sq,
        },
        types::MoveKind,
    };

    #[test]
    fn random_move_always_legal() {
        let cases = [STARTING_FEN, KIWIPETE, PERFT3, PERFT4, PERFT5, PERFT6];
        let mut rng = Rng::new(TEST_SEED);
        for fen in cases {
            for draw in 0..100 {
                let mv = random_move(&from_fen(fen), &mut rng);
                assert!(mv.is_some(), "move {mv:?}, draw {draw}");
            }
        }
    }

    #[test]
    fn no_moves_give_none() {
        let cases = [
            (
                "checkmate with white to move",
                "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3",
            ),
            (
                "stalemate with black to move",
                "7k/5Q2/6K1/8/8/8/8/8 b - - 0 1",
            ),
        ];
        let mut rng = Rng::new(TEST_SEED);
        for (why, fen) in cases {
            for draw in 0..10 {
                let mv = random_move(&from_fen(fen), &mut rng);
                assert!(mv.is_none(), "{why}: move {mv:?}, draw {draw}");
            }
        }
    }

    #[test]
    fn only_legal_move_returns_only_legal_move() {
        let mut rng = Rng::new(TEST_SEED);
        let mv = random_move(&from_fen("7k/8/8/8/8/8/r7/K1r5 w - - 0 1"), &mut rng);
        assert_eq!(
            Some(Move::new(sq("a1"), sq("a2"), MoveKind::Normal)),
            mv,
            "expected: a1a2, actual: {mv:?}"
        );
    }

    #[test]
    fn reaches_every_starting_move() {
        let mut rng = Rng::new(TEST_SEED);
        let board = from_fen(STARTING_FEN);
        let mut seen: Vec<Option<Move>> = vec![];
        for _ in 0..200 {
            seen.push(random_move(&board, &mut rng));
        }
        let mut legal = MoveList::new();
        board.legal_moves(&mut legal);
        assert!(legal.iter().all(|&mv| seen.contains(&Some(mv))))
    }
}
