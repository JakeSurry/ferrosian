//! Helper functions and constants for tests

use crate::board::Board;
use crate::types::Square;

pub const STARTING_FEN: &str = crate::board::STARTING_FEN;
pub const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq -";
pub const PERFT3: &str = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";
pub const PERFT4: &str = "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq -";
pub const PERFT5: &str = "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8";
pub const PERFT6: &str = "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10";

/// Square from its name, e.g. sq("e4")
pub fn sq(name: &str) -> Square {
    name.parse().expect("valid square in test")
}

/// Board from FEN
pub fn from_fen(fen: &str) -> Board {
    fen.parse().expect("valid fen in test")
}
