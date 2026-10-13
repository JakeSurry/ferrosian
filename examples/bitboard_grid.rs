use anyhow::Context;
use chess::types::Bitboard;
use std::env;

fn main() -> anyhow::Result<()> {
    let input = env::args().skip(1).collect::<Vec<_>>().join(" ");
    let num: u64 = if let Some(hex) = input.strip_prefix("0x") {
        u64::from_str_radix(&hex.replace('_', ""), 16)
            .with_context(|| format!("expected a u64, got 0x{hex:?}"))?
    } else {
        input
            .parse()
            .with_context(|| format!("expected a u64, got {input:?}"))?
    };
    println!("{}", Bitboard(num));
    Ok(())
}
