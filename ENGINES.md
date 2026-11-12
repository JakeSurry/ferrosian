# Engine versions

Every Ferrosian version stays runnable, so each improvement is measured by a match against
the version before it. Versions live in `src/versions/`; the registry `VERSIONS` in
`src/versions/mod.rs` is the source of truth.

```sh
cargo run --release --bin engine -- --list-engines           # what exists
cargo run --release --bin engine -- --engine v0 --seed 1     # run one version over UCI
```

Matches use cutechess-cli with the `noob_3moves.epd` opening suite (each opening played
twice, colours swapped).

```sh
cutechess-cli `
    -engine cmd=target/release/engine arg=--engine arg=v0 name=v0 `
    -engine cmd=stockfish name=sf-skill0 "option.Skill Level=0" `
    -each proto=uci tc=10+0.1 `
    -openings file=books/noob_3moves.epd format=epd order=random -repeat `
    -games 2 -rounds 50 -pgnout games/games.pgn
```

## Log

Newest first. "vs previous" is the new version's score against the one before it.

| id | tag | change | vs previous (W/D/L) | Elo ± 95% | tc | openings | games |
| --- | --- | --- | --- | --- | --- | --- | --- |
| v0 | `engine-v0` | uniformly random legal move (baseline) | n/a | n/a | n/a | n/a | n/a |

## Notes

- A core change that speeds up every version (movegen, make-move) is accepted; it will
  be noted here with its commit.

