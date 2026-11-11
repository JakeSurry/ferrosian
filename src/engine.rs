use crate::board::Board;
use crate::rng::Rng;
use crate::search::random_move;
use crate::uci::{
    EngineMessage, GoParams, GuiCommand, PositionCommand, PositionCommandError, UciMove,
};
use std::ops::ControlFlow;

pub struct Engine {
    board: Option<Board>,
    rng: Rng,
}

impl Engine {
    const NAME: &str = "Ferrosian 0.1.0";
    const AUTHOR: &str = "Jake";

    pub fn new(rng: Rng) -> Self {
        Self {
            board: Some(Board::starting_position()),
            rng,
        }
    }

    /// Handles one command. Sends replies through `out`; Break means "quit".
    pub fn handle(
        &mut self,
        cmd: GuiCommand,
        out: &mut impl FnMut(EngineMessage),
    ) -> ControlFlow<()> {
        match cmd {
            GuiCommand::Uci => Self::handle_uci(out),
            GuiCommand::IsReady => Self::handle_isready(out),
            GuiCommand::UciNewGame => self.handle_ucinewgame(),
            GuiCommand::Position(pc) => self.handle_position(pc, out),
            GuiCommand::Go(gp) => self.handle_go(gp, out),
            GuiCommand::Stop => Self::handle_stop(out),
            GuiCommand::PonderHit => Self::handle_ponderhit(out),
            GuiCommand::Debug(b) => Self::handle_debug(out),
            GuiCommand::SetOption { name, value } => Self::handle_setoption(name, value, out),
            GuiCommand::Quit => return ControlFlow::Break(()),
        };
        ControlFlow::Continue(())
    }

    fn handle_uci(out: &mut impl FnMut(EngineMessage)) {
        out(EngineMessage::IdName(Self::NAME.to_string()));
        out(EngineMessage::IdAuthor(Self::AUTHOR.to_string()));
        out(EngineMessage::UciOk);
    }

    fn handle_isready(out: &mut impl FnMut(EngineMessage)) {
        out(EngineMessage::ReadyOk);
    }

    fn handle_ucinewgame(&mut self) {
        self.board = Some(Board::starting_position());
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

    fn handle_go(&mut self, _gp: GoParams, out: &mut impl FnMut(EngineMessage)) {
        let best = match &self.board {
            Some(board) => random_move(board, &mut self.rng),
            None => {
                out(EngineMessage::InfoString("no valid position".to_string()));
                None
            }
        };
        out(EngineMessage::BestMove {
            best: best.map(UciMove::from),
            ponder: None,
        });
    }

    fn handle_stop(out: &mut impl FnMut(EngineMessage)) {
        todo!()
    }

    fn handle_ponderhit(out: &mut impl FnMut(EngineMessage)) {
        todo!()
    }

    fn handle_debug(out: &mut impl FnMut(EngineMessage)) {
        todo!()
    }

    fn handle_setoption(name: String, value: Option<String>, out: &mut impl FnMut(EngineMessage)) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{TEST_SEED, sq};
    use crate::uci::{
        EngineMessage::{BestMove, IdAuthor, IdName, ReadyOk, UciOk},
        UciMove,
    };

    fn run(lines: &[&str]) -> Vec<EngineMessage> {
        let mut engine = Engine::new(Rng::new(TEST_SEED));
        let mut out: Vec<EngineMessage> = vec![];
        let mut send_out = |em| out.push(em);
        for line in lines {
            if let ControlFlow::Break(()) = engine.handle(line.parse().unwrap(), &mut send_out) {
                break;
            }
        }
        out
    }

    #[test]
    fn handle_exact_cases() {
        let cases: &[(&[&str], Vec<EngineMessage>)] = &[
            (
                &["uci"],
                vec![
                    IdName("Ferrosian 0.1.0".to_string()),
                    IdAuthor("Jake".to_string()),
                    UciOk,
                ],
            ),
            (&["isready"], vec![ReadyOk]),
            (&["ucinewgame"], vec![]),
            (&["position startpos moves e2e4 e7e5"], vec![]),
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
    fn handle_quit() {
        let mut engine = Engine::new(Rng::new(TEST_SEED));
        let mut result: Vec<ControlFlow<()>> = ["isready", "quit", "uci"]
            .iter()
            .map(|s| engine.handle(s.parse().unwrap(), &mut |_| ()))
            .collect();
        assert_eq!(ControlFlow::Break(()), result.remove(1));
        assert!(!result.contains(&ControlFlow::Break(())));
    }
}
