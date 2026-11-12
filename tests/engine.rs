//! Runs the `engine` binary as a GUI would: UCI lines in on stdin, replies out on stdout.

use chess::board::Board;
use chess::engine::Engine;
use chess::uci::EngineMessage;
use std::io::Write;
use std::process::{Command, Output, Stdio};

/// Runs the engine with `args`, feeds it `input`, closes stdin and waits for it to exit.
#[track_caller]
fn run_engine(args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_engine"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("engine starts");
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(input.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("engine runs")
}

#[track_caller]
fn stdout_of(args: &[&str], input: &str) -> String {
    let output = run_engine(args, input);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "{input:?}: {}\n{stderr}",
        output.status
    );
    String::from_utf8(output.stdout).expect("stdout is UTF-8")
}

#[test]
fn session() {
    let input = "uci\nisready\nposition startpos moves e2e4\ngo\nquit\n";
    let stdout = stdout_of(&["--seed", "1"], input);
    let lines: Vec<&str> = stdout.lines().collect();

    let [id_name, id_author, uciok, readyok, bestmove] = lines.as_slice() else {
        panic!("{input:?}: expected 5 lines, got {lines:#?}");
    };
    let expected = [
        format!("id name {}", Engine::NAME),
        format!("id author {}", Engine::AUTHOR),
        "uciok".to_string(),
        "readyok".to_string(),
    ];
    assert_eq!(
        expected,
        [*id_name, *id_author, *uciok, *readyok],
        "{input:?}"
    );

    let Ok(EngineMessage::BestMove { best: Some(mv), .. }) = bestmove.parse() else {
        panic!("{input:?}: expected `bestmove <move>`, got {bestmove:?}");
    };
    let after_e2e4: Board = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"
        .parse()
        .expect("valid FEN");
    assert!(
        mv.resolve(&after_e2e4).is_some(),
        "{input:?}: {mv} is not legal after e2e4"
    );
}

#[test]
fn end_of_input_exits() {
    let input = "uci\nisready\nposition startpos moves e2e4\ngo\n";
    let stdout = stdout_of(&["--seed", "1"], input);
    let lines: Vec<&str> = stdout.lines().collect();

    let Some(bestmove) = lines.last() else {
        panic!("{input:?}: stdout is empty")
    };
    let Ok(EngineMessage::BestMove { best: Some(_), .. }) = bestmove.parse() else {
        panic!("{input:?}: expected `bestmove <move>`, got {bestmove:?}");
    };
}

#[test]
fn bad_input_goes_to_stderr() {
    let input = "bogus\n\nisready\nquit\n";
    let output = run_engine(&["--seed", "1"], input);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "{input:?}: {}\n{stderr}",
        output.status
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert_eq!("readyok\n", stdout, "{input:?}");
    assert!(
        stderr.contains("bogus"),
        "{input:?}: {stderr:?} expected to contain \"bogus\""
    )
}

#[test]
fn same_seed_same_output() {
    let input = "position startpos\ngo\ngo\ngo\ngo\ngo";
    let stdout = stdout_of(&["--seed", "1"], input);
    assert!(
        !stdout.is_empty(),
        "{input:?} --seed 1: stdout {stdout:?} is empty",
    );
}

#[test]
fn bad_seed_fails() {
    let input = "quit\n";
    let cases: &[(&[&str], &str)] = &[
        (&["--seed", "0"], "cannot be 0"),
        (&["--seed", "abc"], "failed to parse"),
        (&["--seed"], "needs a value"),
    ];
    for (args, fragment) in cases {
        let output = run_engine(args, input);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(fragment),
            "{input:?}: {args:?}: {stderr:?} expected to contain {fragment:?}"
        );
        assert!(
            !output.status.success(),
            "{input:?}: {args:?}: {stderr:?} expected to exit unsuccessfully"
        );
    }
}
