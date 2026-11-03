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
    println!(
        "...Complete!\n# Leaf Nodes: {}\n",
        format_output(leaf_node_count)
    );
    Ok(())
}

fn format_output(n: u64) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut result = String::with_capacity(s.len() + s.len() / 3);

    for (i, digit) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(*digit as char);
    }
    result
}
