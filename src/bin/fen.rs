use chess::board::Board;
use std::env;

fn main() -> anyhow::Result<()> {
    let fen = env::args().skip(1).collect::<Vec<String>>().join(" ");
    let board: Board = fen.parse()?;
    println!("{board}");
    println!("{}", board.fen());
    Ok(())
}
