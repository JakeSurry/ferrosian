//! v0: plays a uniformly random legal move. The baseline every later version is measured
//! against. Frozen.

use crate::{
    movegen::MoveList,
    rng::Rng,
    types::Move,
    uci::EngineMessage,
    versions::{SearchRequest, Searcher},
};

/// Picks uniformly among the legal moves, so a fixed seed replays the same game.
struct V0 {
    rng: Rng,
}

impl Searcher for V0 {
    fn search(
        &mut self,
        req: SearchRequest<'_>,
        _report: &mut dyn FnMut(EngineMessage),
    ) -> Option<Move> {
        let mut moves = MoveList::new();
        req.board.legal_moves(&mut moves);
        self.rng.choose(&moves).copied()
    }
}

/// Registry constructor for v0.
pub(super) fn build(rng: Rng) -> Box<dyn Searcher> {
    Box::new(V0 { rng })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        board::Board,
        test_helpers::{STARTING_FEN, TEST_SEED, from_fen, sq},
        types::MoveKind,
        uci::GoParams,
        versions::SearchRequest,
    };
    fn best_move(v0: &mut V0, board: &Board) -> Option<Move> {
        let params = GoParams::default();
        let req = SearchRequest {
            board,
            params: &params,
        };
        v0.search(req, &mut |msg| panic!("v0 reported {msg}"))
    }

    fn v0() -> V0 {
        V0 {
            rng: Rng::new(TEST_SEED),
        }
    }

    #[test]
    fn only_legal_move_returns_only_legal_move() {
        let mv = best_move(&mut v0(), &from_fen("7k/8/8/8/8/8/r7/K1r5 w - - 0 1"));
        assert_eq!(
            Some(Move::new(sq("a1"), sq("a2"), MoveKind::Normal)),
            mv,
            "king in double check, only Kxa2 (a1a2)"
        );
    }

    #[test]
    fn reaches_every_starting_move() {
        let board = from_fen(STARTING_FEN);
        let mut legal = MoveList::new();
        let mut engine = v0();
        board.legal_moves(&mut legal);
        let seen: Vec<Move> = (0..200)
            .map(|_| best_move(&mut engine, &board).expect("starting position has moves"))
            .collect();
        let missing: Vec<&Move> = legal.iter().filter(|mv| !seen.contains(mv)).collect();
        assert_eq!(
            Vec::<&Move>::new(),
            missing,
            "moves never drawn from the starting position"
        );
    }

    #[test]
    fn same_seed_same_moves() {
        let mut engine1 = v0();
        let mut engine2 = v0();
        let board = Board::starting_position();
        for mv_num in 0..10 {
            let mv1 = best_move(&mut engine1, &board);
            let mv2 = best_move(&mut engine2, &board);
            assert_eq!(mv1, mv2, "move {mv_num}");
        }
    }
}
