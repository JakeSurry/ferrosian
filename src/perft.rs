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
pub fn divide(_board: &Board, _depth: u32) -> Vec<(Move, u64)> {
    todo!()
}
