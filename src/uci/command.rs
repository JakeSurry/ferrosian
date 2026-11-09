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
    Debug(bool),
    Position(PositionCommand),
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
            "debug" => parse_debug(&mut tokens)?,
            "position" => parse_position(&mut tokens)?,
            "setoption" => parse_setoption(&mut tokens)?,
            other => return Err(CommandError::UnknownCommand(other.to_string())),
        };

        expect_end(tokens)?;
        Ok(command)
    }
}

fn expect_end(mut tokens: SplitWhitespace<'_>) -> Result<(), CommandError> {
    match tokens.next() {
        Some(token) => Err(CommandError::UnexpectedToken(token.to_string())),
        None => Ok(()),
    }
}

/// Splits `words` at the first `keyword`:
/// the words before it, and the words after it if they are present.
fn split_at_keyword<'a>(
    words: &'a [&'a str],
    keyword: &str,
) -> (&'a [&'a str], Option<&'a [&'a str]>) {
    match words.iter().position(|&w| w == keyword) {
        Some(i) => (&words[..i], Some(&words[i + 1..])),
        None => (words, None),
    }
}

/// Parses each word as a `UciMove`, failing on the first bad one.
fn parse_moves(words: &[&str]) -> Result<Vec<UciMove>, CommandError> {
    words
        .iter()
        .map(|w| {
            w.parse::<UciMove>()
                .map_err(|source| CommandError::InvalidMove {
                    text: w.to_string(),
                    source,
                })
        })
        .collect()
}

fn parse_debug(tokens: &mut SplitWhitespace) -> Result<GuiCommand, CommandError> {
    match tokens.next() {
        None => Err(CommandError::MissingValue {
            keyword: "debug".to_string(),
            args: "on/off".to_string(),
        }),
        Some("on") => Ok(GuiCommand::Debug(true)),
        Some("off") => Ok(GuiCommand::Debug(false)),
        Some(w) => Err(CommandError::UnexpectedToken(w.to_string())),
    }
}

fn parse_position(tokens: &mut SplitWhitespace) -> Result<GuiCommand, CommandError> {
    let words: Vec<&str> = tokens.collect();
    let (base_words, move_words) = split_at_keyword(&words, "moves");
    let base = match base_words {
        ["startpos"] => PositionBase::StartPos,
        ["fen", fields @ ..] if !fields.is_empty() => {
            let board = fields
                .join(" ")
                .parse::<Board>()
                .map_err(|source| CommandError::InvalidFen { source })?;
            PositionBase::Fen(Box::new(board))
        }
        ["fen"] => Err(CommandError::MissingValue {
            keyword: "fen".to_string(),
            args: "<fenstring>".to_string(),
        })?,
        [] => Err(CommandError::MissingValue {
            keyword: "position".to_string(),
            args: "[fen <fenstring> | startpos ] moves <move1> .... <movei>".to_string(),
        })?,
        ["startpos", extra, ..] | [extra, ..] => {
            Err(CommandError::UnexpectedToken(extra.to_string()))?
        }
    };
    let moves = parse_moves(move_words.unwrap_or_default())?;
    Ok(GuiCommand::Position(PositionCommand { base, moves }))
}

fn parse_setoption(tokens: &mut SplitWhitespace) -> Result<GuiCommand, CommandError> {
    let words: Vec<&str> = tokens.collect();
    let ["name", rest @ ..] = words.as_slice() else {
        return Err(match words.first() {
            Some(w) => CommandError::UnexpectedToken(w.to_string()),
            None => CommandError::MissingValue {
                keyword: "setoption".to_string(),
                args: "name".to_string(),
            },
        });
    };

    match split_at_keyword(rest, "value") {
        (&[], _) => Err(CommandError::MissingValue {
            keyword: "setoption".to_string(),
            args: "name".to_string(),
        }),
        (_, Some(&[])) => Err(CommandError::MissingValue {
            keyword: "setoption".to_string(),
            args: "value".to_string(),
        }),
        (name, value) => Ok(GuiCommand::SetOption {
            name: name.join(" "),
            value: value.map(|v| v.join(" ")),
        }),
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
            Self::Debug(b) => write!(f, "debug {}", if *b { "on" } else { "off" }),
            Self::Position(pc) => write!(f, "{pc}"),
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

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PositionCommand {
    pub base: PositionBase,
    pub moves: Vec<UciMove>,
}

impl fmt::Display for PositionCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "position {}", self.base)?;

        if !self.moves.is_empty() {
            write_moves(f, self.moves.as_slice())?;
        }
        Ok(())
    }
}

fn write_moves(f: &mut fmt::Formatter<'_>, moves: &[UciMove]) -> fmt::Result {
    write!(f, " moves")?;
    for mv in moves {
        write!(f, " {mv}")?;
    }
    Ok(())
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PositionBase {
    StartPos,
    Fen(Box<Board>),
}

impl fmt::Display for PositionBase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StartPos => write!(f, "startpos"),
            Self::Fen(board) => write!(f, "fen {}", board.fen()),
        }
    }
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

    #[error("keyword {keyword:?} missing args {args:?}")]
    MissingValue { keyword: String, args: String },

    #[error("{text}")]
    InvalidMove { text: String, source: UciMoveError },

    #[error("command error")]
    InvalidFen { source: FenError },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ParseError;
    use PositionBase::*;

    fn position(base: PositionBase, moves: &[&str]) -> GuiCommand {
        let mvs: Vec<UciMove> = moves
            .iter()
            .map(|mv| mv.parse::<UciMove>().unwrap())
            .collect();
        GuiCommand::Position(PositionCommand { base, moves: mvs })
    }

    fn custom_board() -> Board {
        "4k3/8/8/8/8/8/4P3/4K3 w - - 0 1".parse().unwrap()
    }

    #[test]
    fn gui_command_round_trip() {
        let cases = [
            ("uci", GuiCommand::Uci),
            ("isready", GuiCommand::IsReady),
            ("ucinewgame", GuiCommand::UciNewGame),
            ("stop", GuiCommand::Stop),
            ("ponderhit", GuiCommand::PonderHit),
            ("quit", GuiCommand::Quit),
            ("debug on", GuiCommand::Debug(true)),
            ("debug off", GuiCommand::Debug(false)),
            ("position startpos", position(StartPos, &[])),
            (
                "position startpos moves e2e4 e7e5",
                position(StartPos, &["e2e4", "e7e5"]),
            ),
            (
                "position startpos moves e2e5",
                position(StartPos, &["e2e5"]),
            ),
            (
                "position fen 4k3/8/8/8/8/8/4P3/4K3 w - - 0 1",
                position(Fen(Box::new(custom_board())), &[]),
            ),
            (
                "position fen 4k3/8/8/8/8/8/4P3/4K3 w - - 0 1 moves e2e4 e8d7",
                position(Fen(Box::new(custom_board())), &["e2e4", "e8d7"]),
            ),
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
            (
                "empty moves list",
                "position startpos moves",
                position(StartPos, &[]),
            ),
            (
                "extra spaces",
                "position   startpos  moves   e2e4",
                position(StartPos, &["e2e4"]),
            ),
            (
                "four-field fen prints with `0 1`",
                "position fen 4k3/8/8/8/8/8/4P3/4K3 w - -",
                position(Fen(Box::new(custom_board())), &[]),
            ),
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
            (
                "missing debug opts",
                "debug",
                CommandError::MissingValue {
                    keyword: "debug".to_string(),
                    args: "on/off".to_string(),
                },
            ),
            (
                "bad debug opt",
                "debug maybe",
                CommandError::UnexpectedToken("maybe".to_string()),
            ),
            (
                "multiple debug opts",
                "debug on off",
                CommandError::UnexpectedToken("off".to_string()),
            ),
            (
                "missing position opts",
                "position",
                CommandError::MissingValue {
                    keyword: "position".to_string(),
                    args: "[fen <fenstring> | startpos ] moves <move1> .... <movei>".to_string(),
                },
            ),
            (
                "missing fen opts",
                "position fen",
                CommandError::MissingValue {
                    keyword: "fen".to_string(),
                    args: "<fenstring>".to_string(),
                },
            ),
            (
                "no fields before moves",
                "position fen moves e2e4",
                CommandError::MissingValue {
                    keyword: "fen".to_string(),
                    args: "<fenstring>".to_string(),
                },
            ),
            (
                "bad position opt",
                "position start",
                CommandError::UnexpectedToken("start".to_string()),
            ),
            (
                "forgot moves keyword",
                "position startpos e2e4",
                CommandError::UnexpectedToken("e2e4".to_string()),
            ),
            (
                "invalid fen",
                "position fen 8/8 w",
                CommandError::InvalidFen {
                    source: FenError::WrongFieldCount(2),
                },
            ),
            (
                "first bad move is reported",
                "position startpos moves e2e4 e7e9",
                CommandError::InvalidMove {
                    text: "e7e9".to_string(),
                    source: UciMoveError::InvalidSquare {
                        text: "e9".to_string(),
                        source: ParseError::InvalidSquare,
                    },
                },
            ),
        ];
        for (name, case, expected) in cases {
            assert_eq!(expected, case.parse::<GuiCommand>().unwrap_err(), "{name}");
        }
    }
}
