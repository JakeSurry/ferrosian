use super::{UciMove, UciMoveError};
use crate::board::{Board, FenError};
use std::{
    fmt,
    num::ParseIntError,
    str::{FromStr, SplitWhitespace},
    time::Duration,
};
use thiserror::Error;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum GuiCommand {
    Uci,
    IsReady,
    UciNewGame,
    Stop,
    PonderHit,
    Quit,
    SetOption { name: String, value: Option<String> },
}

impl FromStr for GuiCommand {
    type Err = CommandError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tokens = s.split_whitespace();
        let Some(name) = tokens.next() else {
            return Err(CommandError::Empty);
        };
        let command = match name {
            "uci" => Self::Uci,
            "isready" => Self::IsReady,
            "ucinewgame" => Self::UciNewGame,
            "stop" => Self::Stop,
            "ponderhit" => Self::PonderHit,
            "quit" => Self::Quit,
            "setoption" => parse_setoption(&mut tokens)?,
            other => return Err(CommandError::UnknownCommand(other.to_string())),
        };

        expect_end(tokens)?;
        Ok(command)
    }
}

impl fmt::Display for GuiCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uci => f.write_str("uci"),
            Self::IsReady => f.write_str("isready"),
            Self::UciNewGame => f.write_str("ucinewgame"),
            Self::Stop => f.write_str("stop"),
            Self::PonderHit => f.write_str("ponderhit"),
            Self::Quit => f.write_str("quit"),
            Self::SetOption { name, value } => {
                write!(f, "setoption name {name}")?;
                match value {
                    Some(v) => write!(f, "value {v}"),
                    None => Ok(()),
                }
            }
        }
    }
}

fn split_at_keyword<'a>(
    words: &'a [&'a str],
    keyword: &str,
) -> (&'a [&'a str], Option<&'a [&'a str]>) {
    match words.iter().position(|&w| w == keyword) {
        Some(i) => (&words[..i], Some(&words[i + 1..])),
        None => (words, None),
    }
}

fn parse_setoption(tokens: &mut SplitWhitespace) -> Result<GuiCommand, CommandError> {
    let words: Vec<&str> = tokens.collect();
    let ["name", rest @ ..] = words.as_slice() else {
        return Err(match words.first() {
            Some(w) => CommandError::UnexpectedToken(w.to_string()),
            None => CommandError::MissingValue {
                command: "setoption".to_string(),
                keyword: "name".to_string(),
            },
        });
    };

    match split_at_keyword(rest, "value") {
        (&[], _) => Err(CommandError::MissingValue {
            command: "setoption".to_string(),
            keyword: "name".to_string(),
        }),
        (_, Some(&[])) => Err(CommandError::MissingValue {
            command: "setoption".to_string(),
            keyword: "value".to_string(),
        }),
        (name, value) => Ok(GuiCommand::SetOption {
            name: name.join(" "),
            value: value.map(|v| v.join(" ")),
        }),
    }
}

fn expect_end(mut tokens: SplitWhitespace<'_>) -> Result<(), CommandError> {
    match tokens.next() {
        Some(token) => Err(CommandError::UnexpectedToken(token.to_string())),
        None => Ok(()),
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PositionCommand {
    pub base: PositionBase,
    pub moves: Vec<UciMove>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PositionBase {
    StartPos,
    Fen(Board),
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct GoParams {
    pub wtime: Option<Duration>,
    pub btime: Option<Duration>,
    pub winc: Option<Duration>,
    pub binc: Option<Duration>,
    pub moves_to_go: Option<i32>,
    pub depth: Option<u32>,
    pub nodes: Option<u64>,
    pub mate: Option<u32>,
    pub move_time: Option<Duration>,
    pub infinite: bool,
    pub ponder: bool,
    pub search_moves: Vec<UciMove>,
}

pub enum EngineMessage {
    IdName(String),
    IdAuthor(String),
    UciOk,
    ReadyOk,
    BestMove {
        best: Option<UciMove>,
        ponder: Option<UciMove>,
    },
    InfoString(String),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CommandError {
    #[error("got empty command")]
    Empty,

    #[error("unknown command {0:?}")]
    UnknownCommand(String),

    #[error("unexpected token {0:?}")]
    UnexpectedToken(String),

    #[error("")]
    MissingValue { command: String, keyword: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_command_round_trip() {
        let cases = [
            ("uci", GuiCommand::Uci),
            ("isready", GuiCommand::IsReady),
            ("ucinewgame", GuiCommand::UciNewGame),
            ("stop", GuiCommand::Stop),
            ("ponderhit", GuiCommand::PonderHit),
            ("quit", GuiCommand::Quit),
        ];
        for (text, command) in cases {
            assert_eq!(command, text.parse().unwrap(), "{text} -> {command}");
            assert_eq!(text, command.to_string(), "{command} -> {text}");
        }
    }

    #[test]
    fn gui_command_lenient_parse() {
        let cases = [
            ("surrounding spaces", "  isready  ", GuiCommand::IsReady),
            ("windows line ending", "quit\r", GuiCommand::Quit),
            ("tabs are whitespace", "\tuci", GuiCommand::Uci),
        ];
        for (name, case, expected) in cases {
            assert_eq!(expected, case.parse().unwrap(), "{name}");
        }
    }

    #[test]
    fn gui_command_errors() {
        let cases = [
            ("nothing to dispatch on", "", CommandError::Empty),
            ("nothing to dispatch on", "  ", CommandError::Empty),
            (
                "commands are case-sensitive",
                "UCI",
                CommandError::UnknownCommand("UCI".to_string()),
            ),
            (
                "engine message is not a command",
                "uciok",
                CommandError::UnknownCommand("uciok".to_string()),
            ),
            (
                "leftover token",
                "isready now",
                CommandError::UnexpectedToken("now".to_string()),
            ),
        ];
        for (name, case, expected) in cases {
            assert_eq!(expected, case.parse::<GuiCommand>().unwrap_err(), "{name}");
        }
    }
}
