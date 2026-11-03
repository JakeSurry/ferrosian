//! Perft against the standard Chess Programming Wiki positions.
//! <https://www.chessprogramming.org/Perft_Results>
//!
//! Default tests stop where a `cargo test` build (opt-level 3, debug asserts on)
//! takes a few seconds per position. The `#[ignore]`d `*_deep` tests go one
//! depth further; run them with `cargo test --release -- --ignored`.

use chess::board::Board;
use chess::perft::perft;

const STARTING: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq -";
const POSITION_3: &str = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";
const POSITION_4: &str = "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq -";
/// Position 4 with colors swapped and the board flipped: must give the same counts.
const POSITION_4_MIRRORED: &str =
    "r2q1rk1/pP1p2pp/Q4n2/bbp1p3/Np6/1B3NBn/pPPP1PPP/R3K2R b KQ - 0 1";
const POSITION_5: &str = "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8";
const POSITION_6: &str = "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10";

/// Parses and validates.
#[track_caller]
fn board(fen: &str) -> Board {
    let board: Board = fen.parse().unwrap_or_else(|e| panic!("{fen:?}: {e}"));
    board.validate().unwrap_or_else(|e| panic!("{fen:?}:\n{e}"));
    board
}

/// `expected[0]` is the count at depth 1, `expected[1]` at depth 2, and so on.
#[track_caller]
fn check_perft(fen: &str, expected: &[u64]) {
    let board = board(fen);
    for (depth, &nodes) in (1..).zip(expected) {
        assert_eq!(nodes, perft(&board, depth), "perft({depth}) of {fen}");
    }
}

/// One depth only, for deep tests.
#[track_caller]
fn check_depth(fen: &str, depth: u32, nodes: u64) {
    assert_eq!(nodes, perft(&board(fen), depth), "perft({depth}) of {fen}");
}

#[test]
fn starting_position() {
    check_perft(STARTING, &[20, 400, 8_902, 197_281, 4_865_609]);
}

#[test]
fn kiwipete() {
    check_perft(KIWIPETE, &[48, 2_039, 97_862, 4_085_603]);
}

#[test]
fn position_3() {
    check_perft(POSITION_3, &[14, 191, 2_812, 43_238, 674_624]);
}

#[test]
fn position_4() {
    check_perft(POSITION_4, &[6, 264, 9_467, 422_333]);
}

#[test]
fn position_4_mirrored() {
    check_perft(POSITION_4_MIRRORED, &[6, 264, 9_467, 422_333]);
}

#[test]
fn position_5() {
    check_perft(POSITION_5, &[44, 1_486, 62_379, 2_103_487]);
}

#[test]
fn position_6() {
    check_perft(POSITION_6, &[46, 2_079, 89_890, 3_894_594]);
}

#[test]
#[ignore = "slow: run with cargo test --release -- --ignored"]
fn starting_position_deep() {
    check_depth(STARTING, 6, 119_060_324);
}

#[test]
#[ignore = "slow: run with cargo test --release -- --ignored"]
fn kiwipete_deep() {
    check_depth(KIWIPETE, 5, 193_690_690);
}

#[test]
#[ignore = "slow: run with cargo test --release -- --ignored"]
fn position_3_deep() {
    check_depth(POSITION_3, 6, 11_030_083);
}

#[test]
#[ignore = "slow: run with cargo test --release -- --ignored"]
fn position_4_deep() {
    check_depth(POSITION_4, 5, 15_833_292);
}

#[test]
#[ignore = "slow: run with cargo test --release -- --ignored"]
fn position_4_mirrored_deep() {
    check_depth(POSITION_4_MIRRORED, 5, 15_833_292);
}

#[test]
#[ignore = "slow: run with cargo test --release -- --ignored"]
fn position_5_deep() {
    check_depth(POSITION_5, 5, 89_941_194);
}

#[test]
#[ignore = "slow: run with cargo test --release -- --ignored"]
fn position_6_deep() {
    check_depth(POSITION_6, 5, 164_075_551);
}
