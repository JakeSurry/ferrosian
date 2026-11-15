use anyhow::{Context, Result};
use ferrosian::{board::Board, movegen::MoveList};
use std::env;

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

    let move_list = MoveList::legal(&board);

    print!("\n{} legal moves:\n\t", move_list.len());
    move_list.iter().for_each(|&mv| {
        if opts.san {
            print!("{} ", board.san(mv));
        } else {
            print!("{mv} ");
        }
    });
    if move_list.is_empty() {
        if board.is_in_check(board.side_to_move()) {
            println!("checkmate")
        } else {
            println!("stalemate")
        }
    }
    println!();

    Ok(())
}

fn match_flags(flags: Vec<String>) -> Result<Opts> {
    let mut san = false;
    for flag in flags {
        match flag.as_str() {
            "--san" => san = true,
            f => anyhow::bail!("unkown flag {f}: available flags: --san"),
        }
    }
    Ok(Opts { san })
}
