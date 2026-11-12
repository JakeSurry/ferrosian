//! Runs the `engine` binary as a GUI would: UCI lines in on stdin, replies out on stdout.

use chess::board::Board;
use chess::uci::EngineMessage;
use chess::versions::{VERSIONS, newest};
use std::io::Write;
use std::process::{Command, Output, Stdio};

/// Pins the binary to frozen v0 with a fixed seed, so output is exact and stable.
const V0_SEED_1: &[&str] = &["--engine", "v0", "--seed", "1"];

/// The position after 1. e4, where every `bestmove` in these tests must be legal.
const AFTER_E2E4: &str = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";

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
        "{args:?} {input:?}: {}\n{stderr}",
        output.status
    );
    String::from_utf8(output.stdout).expect("stdout is UTF-8")
}

/// Checks that `line` is `bestmove <move>` with a move that is legal after 1. e4.
#[track_caller]
fn assert_legal_after_e2e4(context: &str, line: &str) {
    let Ok(EngineMessage::BestMove { best: Some(mv), .. }) = line.parse() else {
        panic!("{context}: expected `bestmove <move>`, got {line:?}");
    };
    let board: Board = AFTER_E2E4.parse().expect("valid FEN");
    assert!(
        mv.resolve(&board).is_some(),
        "{context}: {mv} is not legal after e2e4"
    );
}

#[test]
fn session() {
    let input = "uci\nisready\nposition startpos moves e2e4\ngo\nquit\n";
    let stdout = stdout_of(V0_SEED_1, input);
    let lines: Vec<&str> = stdout.lines().collect();

    let [id_name, id_author, uciok, readyok, bestmove] = lines.as_slice() else {
        panic!("{input:?}: expected 5 lines, got {lines:#?}");
    };
    let expected = ["id name Ferrosian v0", "id author Jake", "uciok", "readyok"];
    assert_eq!(
        expected,
        [*id_name, *id_author, *uciok, *readyok],
        "{input:?}"
    );
    assert_legal_after_e2e4(input, bestmove);
}

#[test]
fn every_version_runs() {
    let input = "uci\nposition startpos moves e2e4\ngo\nquit\n";
    for version in VERSIONS {
        let args = ["--engine", version.id, "--seed", "1"];
        let stdout = stdout_of(&args, input);
        let lines: Vec<&str> = stdout.lines().collect();
        let context = format!("{args:?} {input:?}");

        assert_eq!(
            Some(&format!("id name Ferrosian {}", version.id).as_str()),
            lines.first(),
            "{context}"
        );
        let bestmove = lines
            .last()
            .unwrap_or_else(|| panic!("{context}: stdout is empty"));
        assert_legal_after_e2e4(&context, bestmove);
    }
}

#[test]
fn default_is_newest() {
    let input = "uci\nquit\n";
    let stdout = stdout_of(&["--seed", "1"], input);
    assert_eq!(
        Some(format!("id name Ferrosian {}", newest().id).as_str()),
        stdout.lines().next(),
        "no --engine: {input:?}"
    );
}

#[test]
fn list_engines() {
    let cases: &[&[&str]] = &[&["--list-engines"], &["--engine", "v0", "--list-engines"]];
    let vlist: String = VERSIONS
        .iter()
        .map(|v| {
            let default = if std::ptr::eq(v, newest()) {
                " (default)"
            } else {
                ""
            };
            format!("\n  {} :  {}{default}", v.id, v.summary)
        })
        .collect();
    let expected = format!("Available Ferrosian versions:{vlist}\n");

    for args in cases {
        let output = run_engine(args, "");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "{args:?}: {}\n{stderr}",
            output.status
        );

        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        assert_eq!(expected, stdout, "{args:?}");
        assert!(
            stdout.starts_with(
                "Available Ferrosian versions:\n  v0 :  makes a uniform random legal move"
            ),
            "{args:?}: first line should be v0's, got {stdout:?}"
        );
        assert_eq!("", stderr, "{args:?}: listing should not print the banner");
    }
}

#[test]
fn end_of_input_exits() {
    let input = "uci\nisready\nposition startpos moves e2e4\ngo\n";
    let stdout = stdout_of(V0_SEED_1, input);
    let bestmove = stdout
        .lines()
        .last()
        .unwrap_or_else(|| panic!("{input:?}: stdout is empty"));
    assert_legal_after_e2e4(input, bestmove);
}

#[test]
fn bad_input_goes_to_stderr() {
    let input = "bogus\n\nisready\nquit\n";
    let output = run_engine(V0_SEED_1, input);
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
    let input = "position startpos\ngo\ngo\ngo\ngo\ngo\n";
    let first = stdout_of(V0_SEED_1, input);
    let second = stdout_of(V0_SEED_1, input);
    assert!(
        !first.is_empty(),
        "{V0_SEED_1:?} {input:?}: stdout is empty"
    );
    assert_eq!(first, second, "{V0_SEED_1:?} {input:?}: two runs differ");
}

#[test]
fn bad_args_fail() {
    let input = "quit\n";
    let cases: &[(&[&str], &str)] = &[
        (&["--seed", "0"], "cannot be 0"),
        (&["--seed", "abc"], "failed to parse"),
        (&["--seed"], "needs a value"),
        (&["--engine"], "needs a value"),
        (&["--engine", "v99"], "unknown engine version \"v99\""),
        (&["--engine", "V0"], "unknown engine version \"V0\""),
        (&["--bogus"], "unknown argument \"--bogus\""),
        (&["v0"], "unknown argument \"v0\""),
    ];
    for (args, fragment) in cases {
        let output = run_engine(args, input);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(fragment),
            "{args:?}: {stderr:?} expected to contain {fragment:?}"
        );
        assert!(
            !output.status.success(),
            "{args:?}: {stderr:?} expected to exit unsuccessfully"
        );
        assert!(
            output.stdout.is_empty(),
            "{args:?}: nothing should reach stdout on a bad argument"
        );
    }
}
