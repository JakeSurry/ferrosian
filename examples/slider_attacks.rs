use anyhow::Context;
use chess::{
    attacks,
    types::{Bitboard, Square},
};
use std::env;

fn main() -> anyhow::Result<()> {
    let input = env::args().skip(1).collect::<Vec<_>>().join(" ");
    if input.len() != 2 {
        anyhow::bail!("expected a rank and file (e.g. `e4`), got {input:?}")
    }

    let square: Square = input
        .parse()
        .with_context(|| format!("expected a rank and file (e.g. `e4`), got {input:?}"))?;

    let blockers = Bitboard::FILE_D;

    println!("Blockers:\n{}", blockers);
    println!(
        "Rook attacks from {}:\n{}",
        square,
        attacks::rook_attacks(square, blockers)
    );
    println!(
        "Bishop attacks from {}:\n{}",
        square,
        attacks::bishop_attacks(square, blockers)
    );
    println!(
        "Queen attacks from {}:\n{}",
        square,
        attacks::queen_attacks(square, blockers)
    );
    Ok(())
}
