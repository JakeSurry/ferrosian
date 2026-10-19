use anyhow::Context;
use chess::{attacks, types::Square};
use std::env;

fn main() -> anyhow::Result<()> {
    let input = env::args().skip(1).collect::<Vec<_>>().join(" ");
    if input.len() != 2 {
        anyhow::bail!("expected a rank and file (e.g. `e4`), got {input:?}")
    }

    let square: Square = input
        .parse()
        .with_context(|| format!("expected a rank and file (e.g. `e4`), got {input:?}"))?;

    println!("{}", attacks::knight_attacks(square));
    Ok(())
}
