use crate::board::Board;
use crate::rng::Rng;
use crate::uci::{EngineMessage, GuiCommand};
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
            GuiCommand::Uci => {
                out(EngineMessage::IdName(Self::NAME.to_string()));
                out(EngineMessage::IdAuthor(Self::AUTHOR.to_string()));
                out(EngineMessage::UciOk);
            }
            GuiCommand::IsReady => out(EngineMessage::ReadyOk),
            GuiCommand::UciNewGame => todo!(),
            GuiCommand::Position(pc) => todo!(),
            GuiCommand::Go(gp) => todo!(),
            GuiCommand::Stop => todo!(),
            GuiCommand::PonderHit => todo!(),
            GuiCommand::Debug(b) => todo!(),
            GuiCommand::SetOption { name, value } => todo!(),
            GuiCommand::Quit => return ControlFlow::Break(()),
        };
        ControlFlow::Continue(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_helpers::TEST_SEED,
        uci::EngineMessage::{IdAuthor, IdName, ReadyOk, UciOk},
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
    fn handle() {
        let cases = [
            (
                &["uci"],
                vec![
                    IdName("Ferrosian 0.1.0".to_string()),
                    IdAuthor("Jake".to_string()),
                    UciOk,
                ],
            ),
            (&["isready"], vec![ReadyOk]),
        ];
        for (cmd, expected) in cases {
            let cmds = cmd.join(" ");
            assert_eq!(expected, run(cmd), "cmd: {cmds}");
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
