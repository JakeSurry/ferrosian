use anyhow::{Context, Result};
use chess::{board::Board, movegen::MoveList};
use std::env;

fn main() -> Result<()> {
    let fen = env::args().skip(1).collect::<Vec<String>>().join(" ");
    let board: Board = fen.parse().context("invalid fen")?;
    board.validate().context("invalid position")?;

    println!("{board}");
    println!("{}", board.fen());

    let mut move_list = MoveList::new();
    board.legal_moves(&mut move_list);

    println!("\n{} legal moves:\n\t{}", move_list.len(), move_list);
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
