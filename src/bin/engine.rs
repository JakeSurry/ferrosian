use anyhow::Context;
use chess::{
    attacks::load_magics,
    engine::Engine,
    rng::Rng,
    uci::{CommandError, GuiCommand},
};
use std::{
    env, io,
    time::{SystemTime, UNIX_EPOCH},
};

fn main() -> anyhow::Result<()> {
    preload();

    let mut engine = Engine::new(Rng::new(seed()));
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

fn preload() {
    let earlier = SystemTime::now();
    println!("Starting {} . . .", Engine::NAME);
    load_magics();
    println!(
        ". . . ready in {:.3} seconds!\n",
        SystemTime::now()
            .duration_since(earlier)
            .unwrap()
            .as_secs_f32()
    )
}

fn seed() -> u64 {
    let args: Vec<String> = env::args().collect();
    if let Some(pos) = args.iter().position(|arg| arg == "--seed")
        && let Ok(arg_seed) = args[pos + 1].parse::<u64>()
    {
        arg_seed
    } else {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}
