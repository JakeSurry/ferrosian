//! Generate pseudo-legal moves in a position; the fixed-capacity `MoveList`.

use crate::types::{Move, MoveKind, Square};
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
