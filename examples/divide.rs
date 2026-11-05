//! Perft split by root move, in the format of Stockfish's `go perft`.
//!
//! ```text
//! cargo run --release --example divide -- <depth> [FEN]
//! cargo run --release --example divide -- 3 "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq -"
//! ```
//!
//! With no FEN, uses the starting position. Moves are sorted by their UCI text so
//! Timing goes to stderr, so `> out.txt` captures only the diffable part.

use anyhow::{Context, Result};
use chess::board::{Board, STARTING_FEN};
use chess::perft::divide;
use std::env;
use std::time::Instant;

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let depth_arg = args.next().context("usage: divide <depth> [FEN]")?;
    let depth: u32 = depth_arg
        .parse()
        .with_context(|| format!("invalid depth {depth_arg:?}"))?;

    let fen = args.collect::<Vec<String>>().join(" ");
    let fen = if fen.is_empty() { STARTING_FEN } else { &fen };
    let board: Board = fen
        .parse()
        .with_context(|| format!("invalid FEN {fen:?}"))?;
    board.validate().context("invalid position")?;

    // println!("{board}\n");

    let start = Instant::now();
    let split = divide(&board, depth);
    let elapsed = start.elapsed();

    let mut lines: Vec<(String, u64)> = split
        .into_iter()
        .map(|(mv, nodes)| (mv.to_string(), nodes))
        .collect();
    lines.sort();

    for (mv, nodes) in &lines {
        println!("{mv}: {nodes}");
    }
    let total: u64 = lines.iter().map(|&(_, nodes)| nodes).sum();
    println!();
    println!("Nodes searched: {total}");

    let seconds = elapsed.as_secs_f64();
    eprintln!(
        "{seconds:.3} s, {:.0} nodes/s",
        total as f64 / seconds.max(f64::EPSILON)
    );
    Ok(())
}
