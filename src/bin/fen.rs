use anyhow::{Context, Result};
use chess::{board::Board, movegen::MoveList};
use std::env;

fn main() -> Result<()> {
    let fen = env::args().skip(1).collect::<Vec<String>>().join(" ");
    let board: Board = fen.parse()?;
    board.validate().context("invalid position")?;

    println!("{board}");
    println!("{}", board.fen());

    let mut move_list = MoveList::new();
    board.legal_moves(&mut move_list);

    println!("\n{} legal moves:\n\t{}\n", move_list.len(), move_list);

    Ok(())
}
