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

use anyhow::{Context, bail};
use chess::{
    attacks::load_magics,
    engine::Engine,
    rng::Rng,
    uci::{CommandError, GuiCommand},
    versions::{VERSIONS, Version, find, newest},
};
use std::{
    env, io,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

fn main() -> anyhow::Result<()> {
    let mut engine = match parse_args(env::args().skip(1))? {
        Mode::Run { version, seed } => Engine::new(version, Rng::new(seed)),
        Mode::List => {
            println!("Available Ferrosian versions: ");
            VERSIONS.iter().for_each(|v| println!("  - {}\n", v.id));
            return Ok(());
        }
    };

    preload(engine.name());

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

enum Mode {
    Run {
        version: &'static Version,
        seed: u64,
    },
    List,
}

fn parse_args(mut args: impl Iterator<Item = String>) -> anyhow::Result<Mode> {
    let mut version: &'static Version = newest();
    let mut seed: u64 = (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64)
        .max(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--engine" => {
                let vnum = args.next().context("--engine needs a value")?;
                version = find(&vnum).context("no engine version {vnum:?}")?;
            }
            "--seed" => {
                seed = args
                    .next()
                    .context("--seed needs a value")?
                    .parse::<u64>()
                    .context("--seed value failed to parse into 64")?;
                if 0 == seed {
                    bail!("--seed value cannot be 0")
                }
            }
            "--list" => return Ok(Mode::List),
            other => bail!("unkown argument {other:?}"),
        }
    }
    Ok(Mode::Run { version, seed })
}

/// Builds the magic tables before the first command, so the first `go` doesn't pay for it.
/// Logs the time taken to stderr.
fn preload(name: String) {
    let start = Instant::now();
    eprintln!("Starting {} . . .", name);
    load_magics();
    eprintln!(
        ". . . ready in {:.3} seconds!\n",
        start.elapsed().as_secs_f32()
    )
}
