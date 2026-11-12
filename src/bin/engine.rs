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

fn preload() {
    let start = Instant::now();
    eprintln!("Starting {} . . .", Engine::NAME);
    load_magics();
    eprintln!(
        ". . . ready in {:.3} seconds!\n",
        start.elapsed().as_secs_f32()
    )
}

fn seed() -> anyhow::Result<u64> {
    let args: Vec<String> = env::args().collect();
    if let Some(pos) = args.iter().position(|arg| arg == "--seed") {
        let user_seed = args
            .get(pos + 1)
            .expect("--seed needs a value")
            .parse::<u64>()?;
        assert_ne!(0, user_seed, "--seed value cannot be 0");
        Ok(user_seed)
    } else {
        Ok(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .max(1) as u64)
    }
}
