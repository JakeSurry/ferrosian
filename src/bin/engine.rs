//! The UCI engine binary: reads commands from stdin and writes replies to stdout.
//!
//! All protocol behaviour lives in [`chess::engine::Engine`]; this file only runs the loop.
//! stdout carries UCI messages only, and everything else (the startup banner, unparsable
//! lines) goes to stderr. The loop ends on `quit` or at the end of input.
//!
//! `--seed <n>` fixes the random seed so a game can be replayed; without it the seed comes
//! from the clock. Example match against itself, from the repository root:
//!
//! ```text
//! cutechess-cli -engine cmd=target/release/engine -engine cmd=target/release/engine \
//!     -each proto=uci tc=10+0.1 -games 10 -pgnout games/games.pgn
//! ```

use anyhow::Context;
use chess::{
    attacks::load_magics,
    engine::Engine,
    rng::Rng,
    uci::{CommandError, GuiCommand},
};
use std::{
    env, io,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

fn main() -> anyhow::Result<()> {
    preload();

    let mut engine = Engine::new(Rng::new(seed()?));

    for line in io::stdin().lines() {
        let line = line.context("reading stdin")?;
        let cmd = match line.parse::<GuiCommand>() {
            Ok(cmd) => cmd,
            Err(CommandError::Empty) => continue,
            Err(err) => {
                eprintln!("{line:?}: {err}");
                continue;
            }
        };

        if engine.handle(cmd, &mut |msg| println!("{msg}")).is_break() {
            break;
        }
    }

    Ok(())
}

/// Builds the magic tables before the first command, so the first `go` doesn't pay for it.
/// Logs the time taken to stderr.
fn preload() {
    let start = Instant::now();
    eprintln!("Starting {} . . .", Engine::NAME);
    load_magics();
    eprintln!(
        ". . . ready in {:.3} seconds!\n",
        start.elapsed().as_secs_f32()
    )
}

/// The seed from `--seed <n>`, or the clock in nanoseconds if the flag is absent.
///
/// Fails if the value is missing, not a `u64`, or 0 (which [`Rng::new`] rejects).
fn seed() -> anyhow::Result<u64> {
    let args: Vec<String> = env::args().collect();
    if let Some(pos) = args.iter().position(|arg| arg == "--seed") {
        let user_seed = args
            .get(pos + 1)
            .context("--seed needs a value")?
            .parse::<u64>()
            .context("--seed value failed to parse into u64")?;
        if 0 == user_seed {
            anyhow::bail!("--seed value cannot be 0")
        }
        Ok(user_seed)
    } else {
        Ok((SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64)
            .max(1))
    }
}
