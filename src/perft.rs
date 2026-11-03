//! Perft (performance test) counts the leaf nodes of the legal move tree to a fixed depth.
//! From the starting position there are 20 positions after 1 ply, 400 after 2 and 8,902 after 3.
//! Divide is the same count split by root move (e2e4: 600, g1f3: 440, ...),
//! so a wrong total can be narrowed to the one move whose subtree is off

use crate::board::Board;
use crate::movegen::MoveList;
use crate::types::Move;

/// Number of leaf nodes of the legal move tree `depth` plies deep.
/// `perft(board, 0)` is 1: the position itself.
pub fn perft(board: &Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }

    let mut moves = MoveList::new();
    board.legal_moves(&mut moves);

    if depth == 1 {
        return moves.len() as u64;
    }

    moves
        .iter()
        .map(|&mv| {
            let mut next = *board;
            next.make_move(mv);
            perft(&next, depth - 1)
        })
        .sum()
}

/// The perft count below each legal root move, in move generation order.
/// `divide(board, 0)` is empty: there is no root move to split by.
pub fn divide(board: &Board, depth: u32) -> Vec<(Move, u64)> {
    if depth == 0 {
        return Vec::new();
    }

    let mut moves = MoveList::new();
    board.legal_moves(&mut moves);

    moves
        .iter()
        .map(|&mv| {
            let mut next = *board;
            next.make_move(mv);
            (mv, perft(&next, depth - 1))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{KIWIPETE, PERFT3, PERFT4, PERFT5, PERFT6, STARTING_FEN, from_fen};

    /// Every helper FEN, named for assertion messages.
    const POSITIONS: &[(&str, &str)] = &[
        ("start", STARTING_FEN),
        ("kiwipete", KIWIPETE),
        ("position 3", PERFT3),
        ("position 4", PERFT4),
        ("position 5", PERFT5),
        ("position 6", PERFT6),
    ];

    #[test]
    fn perft_starting_position_shallow() {
        let board = Board::starting_position();
        // `(0..).zip(...)` numbers the rows; `depth` is inferred as u32 from `perft`.
        for (depth, expected) in (0..).zip([1, 20, 400, 8_902]) {
            assert_eq!(expected, perft(&board, depth), "start, depth {depth}");
        }
    }

    #[test]
    fn perft_depth_zero_is_one() {
        for &(name, fen) in POSITIONS {
            assert_eq!(1, perft(&from_fen(fen), 0), "{name}");
        }
    }

    #[test]
    fn divide_depth_zero_is_empty() {
        for &(name, fen) in POSITIONS {
            let split = divide(&from_fen(fen), 0);
            assert!(split.is_empty(), "{name}: {split:?}");
        }
    }

    #[test]
    fn divide_sums_to_perft() {
        for &(name, fen) in POSITIONS {
            let board = from_fen(fen);
            for depth in 1..=3 {
                let sum: u64 = divide(&board, depth).iter().map(|&(_, nodes)| nodes).sum();
                assert_eq!(perft(&board, depth), sum, "{name}, depth {depth}");
            }
        }
    }

    #[test]
    fn divide_moves_are_the_legal_moves_in_order() {
        for &(name, fen) in POSITIONS {
            let board = from_fen(fen);
            let mut legal = MoveList::new();
            board.legal_moves(&mut legal);
            for depth in 1..=2 {
                let moves: Vec<Move> = divide(&board, depth).iter().map(|&(mv, _)| mv).collect();
                assert_eq!(legal.as_slice(), moves, "{name}, depth {depth}");
            }
        }
    }

    #[test]
    fn divide_depth_one_counts_each_move_once() {
        for &(name, fen) in POSITIONS {
            for (mv, nodes) in divide(&from_fen(fen), 1) {
                assert_eq!(1, nodes, "{name}: {mv}");
            }
        }
    }

    /// Exact split from Stockfish's `go perft 3` on the starting position.
    #[test]
    fn divide_starting_position_matches_stockfish() {
        let mut expected = vec![
            ("a2a3", 380),
            ("b2b3", 420),
            ("c2c3", 420),
            ("d2d3", 539),
            ("e2e3", 599),
            ("f2f3", 380),
            ("g2g3", 420),
            ("h2h3", 380),
            ("a2a4", 420),
            ("b2b4", 421),
            ("c2c4", 441),
            ("d2d4", 560),
            ("e2e4", 600),
            ("f2f4", 401),
            ("g2g4", 421),
            ("h2h4", 420),
            ("b1a3", 400),
            ("b1c3", 440),
            ("g1f3", 440),
            ("g1h3", 400),
        ];
        expected.sort();

        // Divide's order is generation order; sort by move text to compare.
        let mut actual: Vec<(String, u64)> = divide(&Board::starting_position(), 3)
            .into_iter()
            .map(|(mv, nodes)| (mv.to_string(), nodes))
            .collect();
        actual.sort();

        // `(&str, u64)` and `(String, u64)` don't compare directly, so map one side.
        let expected: Vec<(String, u64)> = expected
            .into_iter()
            .map(|(mv, nodes)| (mv.to_string(), nodes))
            .collect();
        assert_eq!(expected, actual);
    }
}
