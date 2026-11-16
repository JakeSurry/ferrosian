//! Prints a position, its FEN and its legal moves.
//!
//! ```text
//! cargo run --bin fen -- [--san] <FEN>
//! ```
//!
//! Moves are UCI by default; `--san` prints them as SAN.

use anyhow::{Context, Result};
use ferrosian::{board::Board, movegen::MoveList};
use std::env;

/// Command-line flags.
struct Opts {
    pub san: bool,
}

fn main() -> Result<()> {
    let (flags, fen): (Vec<String>, Vec<String>) =
        env::args().skip(1).partition(|arg| arg.starts_with("--"));
    let opts = match_flags(flags)?;

    let board: Board = fen.join(" ").parse().context("invalid fen")?;
    board.validate().context("invalid position")?;

    println!("{board}");
    println!("{}", board.fen());

    let moves: Vec<String> = MoveList::legal(&board)
        .iter()
        .map(|&mv| {
            if opts.san {
                board.san(mv).to_string()
            } else {
                mv.to_string()
            }
        })
        .collect();

    print!("\n{} legal moves:\n\t{}", moves.len(), moves.join(" "));
    if moves.is_empty() {
        if board.is_in_check(board.side_to_move()) {
            println!("checkmate")
        } else {
            println!("stalemate")
        }
    }
    println!();

    Ok(())
}

/// Parses the flags, rejecting any it doesn't know.
fn match_flags(flags: Vec<String>) -> Result<Opts> {
    let mut san = false;
    for flag in flags {
        match flag.as_str() {
            "--san" => san = true,
            f => anyhow::bail!("unknown flag {f}: available flags: --san"),
        }
    }
    Ok(Opts { san })
}
