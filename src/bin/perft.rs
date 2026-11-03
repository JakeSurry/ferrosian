use anyhow::{Context, Result};
use chess::{board::Board, perft::perft};
use std::{env, io, io::Write};

fn main() -> Result<()> {
    let fen = env::args().skip(1).collect::<Vec<String>>().join(" ");
    let board: Board = fen.parse()?;

    println!("{board}");
    println!("{}", board.fen());

    print!("\ndepth: u32 =? ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("failed to read line")?;

    println!("\nbeginning perft...");

    let depth: u32 = input
        .trim()
        .parse()
        .context("could not parse input as u32")?;

    let leaf_node_count = perft(&board, depth);
    println!("...Complete!\n# Leaf Nodes: {}", leaf_node_count);
    Ok(())
}
