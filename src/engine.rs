//! The UCI engine's state machine: [`Engine`] turns each [`GuiCommand`] into the
//! [`EngineMessage`]s it causes.
//!
//! `Engine` does no I/O. The caller parses lines, passes commands to [`Engine::handle`] and
//! prints whatever it sends, which is all `bin/engine.rs` does. That keeps every protocol
//! behaviour testable with typed values, and lets a game loop drive the engine in-process.

use crate::board::Board;
use crate::rng::Rng;
use crate::uci::{
    EngineMessage, GoParams, GuiCommand, PositionCommand, PositionCommandError, UciMove,
};
use crate::versions::{SearchRequest, Searcher, Version};
use std::ops::ControlFlow;

/// A UCI engine. Behaviors are dependent on [`Version`].
///
/// The position is `None` after a `position` command that failed; `go` then answers
/// `bestmove 0000` until a valid `position` or `ucinewgame` arrives.
pub struct Engine {
    board: Option<Board>,
    version: &'static Version,
    searcher: Box<dyn Searcher>,
}

impl Engine {
    /// The author sent in `id author`.
    pub const AUTHOR: &str = "Jake";

    /// Creates an engine on the starting position that draws its moves from `rng`.
    ///
    /// Pass a fixed seed for reproducible games.
    pub fn new(version: &'static Version, rng: Rng) -> Self {
        Self {
            board: Some(Board::starting_position()),
            version,
            searcher: (version.build)(rng),
        }
    }

    /// Returns the version id of the engine.
    pub fn name(&self) -> String {
        format!("Ferrosian {}", self.version.id)
    }

    /// Returns the author of the engine.
    pub fn author(&self) -> &'static str {
        self.version.author
    }

    /// Returns the version of the engine.
    pub fn version(&self) -> &'static Version {
        self.version
    }

    /// Handles one command, sending any replies through `out` in order.
    ///
    /// Returns [`ControlFlow::Break`] for `quit` and [`ControlFlow::Continue`] for every other
    /// command. An invalid `position` sends an `info string` with the error and clears the
    /// position, so the next `go` sends `bestmove 0000` instead of a move for the wrong board.
    /// Commands that need no reply (`ucinewgame`, a valid `position`, `stop`) send nothing.
    pub fn handle(
        &mut self,
        cmd: GuiCommand,
        out: &mut impl FnMut(EngineMessage),
    ) -> ControlFlow<()> {
        match cmd {
            GuiCommand::Uci => self.handle_uci(out),
            GuiCommand::IsReady => Self::handle_isready(out),
            GuiCommand::UciNewGame => self.handle_ucinewgame(),
            GuiCommand::Position(pc) => self.handle_position(pc, out),
            GuiCommand::Go(gp) => self.handle_go(gp, out),
            GuiCommand::Stop => Self::handle_stop(),
            GuiCommand::PonderHit => Self::handle_ponderhit(out),
            GuiCommand::Debug(b) => Self::handle_debug(b, out),
            GuiCommand::SetOption { name, value } => Self::handle_setoption(name, value, out),
            GuiCommand::Quit => return ControlFlow::Break(()),
        };
        ControlFlow::Continue(())
    }

    fn handle_uci(&self, out: &mut impl FnMut(EngineMessage)) {
        out(EngineMessage::IdName(self.name()));
        out(EngineMessage::IdAuthor(self.author().to_string()));
        out(EngineMessage::UciOk);
    }

    fn handle_isready(out: &mut impl FnMut(EngineMessage)) {
        out(EngineMessage::ReadyOk);
    }

    fn handle_ucinewgame(&mut self) {
        self.board = Some(Board::starting_position());
        self.searcher.new_game();
    }

    fn handle_position(&mut self, pc: PositionCommand, out: &mut impl FnMut(EngineMessage)) {
        fn info_line(err: &PositionCommandError) -> String {
            match err {
                PositionCommandError::InvalidPosition { source } => {
                    let details: Vec<String> =
                        source.errors().iter().map(ToString::to_string).collect();
                    format!("{err}: {}", details.join("; "))
                }
                PositionCommandError::IllegalMove { .. } => err.to_string(),
            }
        }
        self.board = match pc.to_board() {
            Ok(board) => Some(board),
            Err(err) => {
                out(EngineMessage::InfoString(info_line(&err)));
                None
            }
        };
    }

    fn handle_go(&mut self, gp: GoParams, out: &mut impl FnMut(EngineMessage)) {
        let best = match &self.board {
            Some(board) => {
                let req = SearchRequest { board, params: &gp };
                self.searcher.search(req, out)
            }
            None => {
                out(EngineMessage::InfoString(
                    "current position none or invalid".to_string(),
                ));
                None
            }
        };
        out(EngineMessage::BestMove {
            best: best.map(UciMove::from),
            ponder: None,
        });
    }

    fn handle_stop() {}

    fn handle_ponderhit(out: &mut impl FnMut(EngineMessage)) {
        out(EngineMessage::InfoString(
            "command `ponderhit` not implemented yet".to_string(),
        ))
    }

    fn handle_debug(_b: bool, out: &mut impl FnMut(EngineMessage)) {
        out(EngineMessage::InfoString(
            "command `debug` not implemented yet".to_string(),
        ))
    }

    fn handle_setoption(name: String, _value: Option<String>, out: &mut impl FnMut(EngineMessage)) {
        out(EngineMessage::InfoString(format!(
            "unknown option {name:?}"
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movegen::MoveList;
    use crate::test_helpers::{STARTING_FEN, TEST_SEED, from_fen, sq};
    use crate::uci::{
        EngineMessage::{BestMove, IdAuthor, IdName, InfoString, ReadyOk, UciOk},
        UciMove,
    };
    use crate::versions::{VERSIONS, find};

    fn run(lines: &[&str]) -> Vec<EngineMessage> {
        let mut engine = Engine::new(
            find("v0").expect("engine version v0 exists"),
            Rng::new(TEST_SEED),
        );
        let mut out: Vec<EngineMessage> = vec![];
        let mut send_out = |em| out.push(em);
        for line in lines {
            if let ControlFlow::Break(()) = engine.handle(
                line.parse().unwrap_or_else(|e| panic!("{line:?}: {e}")),
                &mut send_out,
            ) {
                break;
            }
        }
        out
    }

    #[test]
    fn uci_identifies_every_version() {
        for version in VERSIONS {
            let mut engine = Engine::new(version, Rng::new(TEST_SEED));
            let mut out: Vec<EngineMessage> = vec![];
            let _ = engine.handle(GuiCommand::Uci, &mut |em| out.push(em));
            assert_eq!(
                vec![
                    IdName(format!("Ferrosian {}", version.id)),
                    IdAuthor(version.author.to_string()),
                    UciOk
                ],
                out
            );
        }
    }

    #[test]
    fn handle_command() {
        let cases: &[(&[&str], Vec<EngineMessage>)] = &[
            (
                &["uci"],
                vec![
                    IdName("Ferrosian v0".to_string()),
                    IdAuthor("Jake".to_string()),
                    UciOk,
                ],
            ),
            (&["isready"], vec![ReadyOk]),
            (&["ucinewgame"], vec![]),
            (&["position startpos moves e2e4 e7e5"], vec![]),
            (&["stop"], vec![]),
            (
                &["ponderhit"],
                vec![InfoString(
                    "command `ponderhit` not implemented yet".to_string(),
                )],
            ),
            (
                &["debug on"],
                vec![InfoString(
                    "command `debug` not implemented yet".to_string(),
                )],
            ),
            (
                &["setoption name Hash value 64"],
                vec![InfoString("unknown option \"Hash\"".to_string())],
            ),
        ];
        for (cmd, expected) in cases {
            let cmds = cmd.join(" ");
            assert_eq!(expected, &run(cmd), "cmd: {cmds}");
        }
    }

    #[test]
    fn handle_position_go() {
        let cases: &[(&[&str], Vec<EngineMessage>)] = &[
            (
                &["position fen k7/1Q6/2K5/8/8/8/8/8 b - -", "go"],
                vec![BestMove {
                    best: None,
                    ponder: None,
                }],
            ),
            (
                &["position fen k7/8/1QK5/8/8/8/8/8 b - -", "go"],
                vec![BestMove {
                    best: None,
                    ponder: None,
                }],
            ),
            (
                &["position fen k7/8/2K5/8/8/8/8/1Q6 w - - moves b1b7", "go"],
                vec![BestMove {
                    best: None,
                    ponder: None,
                }],
            ),
            (
                &["position fen k7/8/8/8/8/8/8/8 w - -"],
                vec![InfoString(
                    "invalid FEN position: expected one king per color, found 0 kings for White"
                        .to_string(),
                )],
            ),
            (
                &["position fen k7/8/K7/8/8/8/8/8 w - - moves e2e4", "go"],
                vec![
                    InfoString("illegal move e2e4 at index 0".to_string()),
                    InfoString("current position none or invalid".to_string()),
                    BestMove {
                        best: None,
                        ponder: None,
                    },
                ],
            ),
            (
                &["position fen 8/8/8/8/8/8/8/8 w - - moves e2e4", "go"],
                vec![
                    InfoString("invalid FEN position: expected one king per color, found 0 kings for White; expected one king per color, found 0 kings for Black".to_string()),
                    InfoString("current position none or invalid".to_string()),
                    BestMove {
                        best: None,
                        ponder: None,
                    },
                ],
            ),
            (
                &["position fen k7/8/2K5/1Q6/8/8/8/8 b - -", "go"],
                vec![BestMove {
                    best: Some(UciMove {
                        from: sq("a8"),
                        to: sq("a7"),
                        promotion: None,
                    }),
                    ponder: None,
                }],
            ),
        ];
        for (cmd, expected) in cases {
            let cmds = cmd.join(" ");
            assert_eq!(expected, &run(cmd), "cmd: {cmds}");
        }
    }

    #[test]
    fn go_bestmove_resolves() {
        let cases: &[(&[&str], &str)] = &[
            (
                &["position startpos moves e2e4", "go"],
                "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
            ),
            (
                &["position startpos moves e2e4", "ucinewgame", "go"],
                STARTING_FEN,
            ),
        ];
        for (lines, fen) in cases {
            let &[
                BestMove {
                    best: Some(best),
                    ponder: None,
                },
            ] = run(lines).as_slice()
            else {
                panic!("did not get a `BestMove` command from engine")
            };
            let mut moves = MoveList::new();
            from_fen(fen).legal_moves(&mut moves);

            assert!(
                moves.iter().any(|&mv| UciMove::from(mv) == best),
                "bestmove: {best} not in legal moves"
            )
        }
    }

    #[test]
    fn handle_quit() {
        let cases = [
            ("uci", ControlFlow::Continue(())),
            ("isready", ControlFlow::Continue(())),
            ("ucinewgame", ControlFlow::Continue(())),
            ("position startpos", ControlFlow::Continue(())),
            ("go", ControlFlow::Continue(())),
            ("stop", ControlFlow::Continue(())),
            ("ponderhit", ControlFlow::Continue(())),
            ("debug off", ControlFlow::Continue(())),
            ("setoption name Hash value 32", ControlFlow::Continue(())),
            ("quit", ControlFlow::Break(())),
        ];
        let version = find("v0").expect("engine version v0 exists");
        for (cmd, expected) in cases {
            let mut engine = Engine::new(version, Rng::new(TEST_SEED));
            assert_eq!(
                expected,
                engine.handle(cmd.parse::<GuiCommand>().unwrap(), &mut |_| ()),
                "{cmd:?}"
            )
        }
    }
}
