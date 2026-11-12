//! Engine-to-GUI UCI messages: what the engine prints and what a UCI client parses.
//!
//! The other direction is [`GuiCommand`](super::GuiCommand). Only the messages a random mover
//! needs exist so far; `info` search fields and `option` declarations come with the search and
//! the first engine option.

use super::uci_move::{UciMove, UciMoveError};
use std::{
    fmt,
    str::{FromStr, SplitWhitespace},
};
use thiserror::Error;

/// Why a line is not a valid [`EngineMessage`].
#[derive(Clone, PartialEq, Eq, Debug, Error)]
pub enum MessageError {
    /// The line has no tokens.
    #[error("empty message")]
    Empty,

    /// The first token names no engine message.
    #[error("unknown message {0:?}")]
    UnknownMessage(String),

    /// A token that does not fit where it appears, including anything after a complete message.
    #[error("unexpected token {0:?}")]
    UnexpectedToken(String),

    /// The line ends where `keyword` needs a value.
    #[error("missing value after {keyword}")]
    MissingValue { keyword: &'static str },

    /// A move after `bestmove` or `ponder` is not UCI move syntax.
    #[error("invalid move {text:?}")]
    InvalidMove { text: String, source: UciMoveError },
}

/// UCI's "no move", printed as `bestmove 0000` when the side to move is mated or stalemated.
const NULL_MOVE: &str = "0000";
/// Stockfish's spelling of the null move. Accepted when parsing, never printed.
const STOCKFISH_NULL_MOVE: &str = "(none)";

/// A message from the engine to the GUI.
///
/// `FromStr` parses one line; `Display` prints the canonical line without its newline.
/// String values must not contain a newline, or the message prints as two lines.
///
/// ```
/// use ferrosian::uci::EngineMessage;
///
/// let msg: EngineMessage = "bestmove  e2e4 ponder e7e5".parse().unwrap();
/// assert_eq!("bestmove e2e4 ponder e7e5", msg.to_string());
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum EngineMessage {
    /// `uciok`: the reply to `uci`, after the `id` lines.
    UciOk,
    /// `readyok`: the reply to `isready`.
    ReadyOk,
    /// `id name <name>`: the engine's name, which may contain spaces.
    IdName(String),
    /// `id author <author>`: the engine's author, which may contain spaces.
    IdAuthor(String),
    /// `info string <text>`: free text for the GUI to show. Takes the rest of the line.
    InfoString(String),
    /// `bestmove <move> [ponder <move>]`: the end of a search.
    ///
    /// `best: None` is the null move (`0000`), sent when there is no legal move. A
    /// `ponder` move is meaningful only after a real best move: parsing rejects
    /// `bestmove 0000 ponder ...`, and printing ignores `ponder` when `best` is `None`.
    BestMove {
        /// The move the engine plays, or `None` for the null move.
        best: Option<UciMove>,
        /// The reply the engine expects, for the GUI to ponder on.
        ponder: Option<UciMove>,
    },
}

impl FromStr for EngineMessage {
    type Err = MessageError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tokens = s.split_whitespace();
        let Some(name) = tokens.next() else {
            return Err(MessageError::Empty);
        };
        let message = match name {
            "uciok" => Self::UciOk,
            "readyok" => Self::ReadyOk,
            "id" => parse_id(&mut tokens)?,
            "info" => parse_info(&mut tokens)?,
            "bestmove" => parse_best_move(&mut tokens)?,
            other => return Err(MessageError::UnknownMessage(other.to_string())),
        };

        expect_end(&mut tokens)?;
        Ok(message)
    }
}

/// Fails on the first token left over after a complete message.
fn expect_end(tokens: &mut SplitWhitespace<'_>) -> Result<(), MessageError> {
    match tokens.next() {
        Some(token) => Err(MessageError::UnexpectedToken(token.to_string())),
        None => Ok(()),
    }
}

/// The remaining tokens joined with single spaces, for the value after `keyword`.
fn rest_of_line(
    keyword: &'static str,
    tokens: &mut SplitWhitespace<'_>,
) -> Result<String, MessageError> {
    let words: Vec<&str> = tokens.collect();
    match words.as_slice() {
        [] => Err(MessageError::MissingValue { keyword }),
        rest => Ok(rest.join(" ")),
    }
}

/// Parses the rest of an `id` line: `name <words>` or `author <words>`.
fn parse_id(tokens: &mut SplitWhitespace<'_>) -> Result<EngineMessage, MessageError> {
    match tokens.next() {
        Some("name") => Ok(EngineMessage::IdName(rest_of_line("name", tokens)?)),
        Some("author") => Ok(EngineMessage::IdAuthor(rest_of_line("author", tokens)?)),
        Some(other) => Err(MessageError::UnexpectedToken(other.to_string())),
        None => Err(MessageError::MissingValue { keyword: "id" }),
    }
}

/// Parses the rest of an `info` line. Only `info string <words>` exists so far.
fn parse_info(tokens: &mut SplitWhitespace<'_>) -> Result<EngineMessage, MessageError> {
    match tokens.next() {
        Some("string") => Ok(EngineMessage::InfoString(rest_of_line("string", tokens)?)),
        Some(other) => Err(MessageError::UnexpectedToken(other.to_string())),
        None => Err(MessageError::MissingValue { keyword: "info" }),
    }
}

/// Parses one token as a move, naming the token in the error.
fn parse_move(mv: &str) -> Result<UciMove, MessageError> {
    mv.parse().map_err(|source| MessageError::InvalidMove {
        text: mv.to_string(),
        source,
    })
}

/// The move after `keyword`.
fn move_after(
    keyword: &'static str,
    tokens: &mut SplitWhitespace<'_>,
) -> Result<UciMove, MessageError> {
    match tokens.next() {
        Some(mv) => parse_move(mv),
        None => Err(MessageError::MissingValue { keyword }),
    }
}

/// Parses the rest of a `bestmove` line: a move or the null move, then optionally
/// `ponder <move>` (only after a real move).
fn parse_best_move(tokens: &mut SplitWhitespace<'_>) -> Result<EngineMessage, MessageError> {
    let best = match tokens.next() {
        None => {
            return Err(MessageError::MissingValue {
                keyword: "bestmove",
            });
        }
        Some(NULL_MOVE | STOCKFISH_NULL_MOVE) => None,
        Some(text) => Some(parse_move(text)?),
    };
    let ponder = match tokens.next() {
        None => None,
        Some("ponder") if best.is_some() => Some(move_after("ponder", tokens)?),
        Some(other) => return Err(MessageError::UnexpectedToken(other.to_string())),
    };
    Ok(EngineMessage::BestMove { best, ponder })
}

impl fmt::Display for EngineMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UciOk => f.write_str("uciok"),
            Self::ReadyOk => f.write_str("readyok"),
            Self::IdName(s) => write!(f, "id name {s}"),
            Self::IdAuthor(s) => write!(f, "id author {s}"),
            Self::InfoString(s) => write!(f, "info string {s}"),
            Self::BestMove { best, ponder } => {
                f.write_str("bestmove")?;
                match (best, ponder) {
                    (Some(b), Some(p)) => write!(f, " {b} ponder {p}"),
                    (Some(b), None) => write!(f, " {b}"),
                    (None, _) => write!(f, " {NULL_MOVE}"),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ParseError;

    fn ucim(mv: &str) -> UciMove {
        mv.parse::<UciMove>().unwrap()
    }

    #[test]
    fn engine_message_round_trip() {
        let cases = [
            ("uciok", EngineMessage::UciOk),
            ("readyok", EngineMessage::ReadyOk),
            (
                "id name Ferrosian 0.1.0",
                EngineMessage::IdName("Ferrosian 0.1.0".to_string()),
            ),
            (
                "id author Jake",
                EngineMessage::IdAuthor("Jake".to_string()),
            ),
            (
                "info string random mover",
                EngineMessage::InfoString("random mover".to_string()),
            ),
            (
                "bestmove e2e4",
                EngineMessage::BestMove {
                    best: Some(ucim("e2e4")),
                    ponder: None,
                },
            ),
            (
                "bestmove e7e8q",
                EngineMessage::BestMove {
                    best: Some(ucim("e7e8q")),
                    ponder: None,
                },
            ),
            (
                "bestmove 0000",
                EngineMessage::BestMove {
                    best: None,
                    ponder: None,
                },
            ),
            (
                "bestmove e2e4 ponder e7e5",
                EngineMessage::BestMove {
                    best: Some(ucim("e2e4")),
                    ponder: Some(ucim("e7e5")),
                },
            ),
        ];
        for (text, command) in cases {
            assert_eq!(text, command.to_string(), "printing {command:?}");
            assert_eq!(
                Ok(command),
                text.parse::<EngineMessage>(),
                "parsing {text:?}"
            );
        }
    }

    #[test]
    fn engine_message_lenient_parse() {
        let cases = [
            ("surrounding spaces", "  readyok  ", EngineMessage::ReadyOk),
            ("windows line ending", "uciok\r", EngineMessage::UciOk),
            (
                "collapsed spaces into value",
                "id name  Ferrosian  0.1.0",
                EngineMessage::IdName("Ferrosian 0.1.0".to_string()),
            ),
            (
                "stockfish null move formatting",
                "bestmove (none)",
                EngineMessage::BestMove {
                    best: None,
                    ponder: None,
                },
            ),
            (
                "extra spaces around ponder",
                "bestmove  e2e4  ponder  e7e5",
                EngineMessage::BestMove {
                    best: Some(ucim("e2e4")),
                    ponder: Some(ucim("e7e5")),
                },
            ),
        ];
        for (why, text, expected) in cases {
            assert_ne!(
                text,
                expected.to_string(),
                "{why}: {text:?} is canonical, move it to the round-trip table"
            );
            assert_eq!(
                Ok(expected),
                text.parse::<EngineMessage>(),
                "{why}: {text:?}"
            );
        }
    }

    #[test]
    fn engine_message_errors() {
        let cases = [
            ("empty message", "", MessageError::Empty),
            ("blank message", "  ", MessageError::Empty),
            (
                "messages are case-sensitive",
                "UCIOK",
                MessageError::UnknownMessage("UCIOK".to_string()),
            ),
            (
                "command is not an engine message",
                "isready",
                MessageError::UnknownMessage("isready".to_string()),
            ),
            (
                "leftover token",
                "uciok now",
                MessageError::UnexpectedToken("now".to_string()),
            ),
            (
                "missing id opts",
                "id",
                MessageError::MissingValue { keyword: "id" },
            ),
            (
                "id missing name keyword",
                "id version 1",
                MessageError::UnexpectedToken("version".to_string()),
            ),
            (
                "id name missing opts",
                "id name",
                MessageError::MissingValue { keyword: "name" },
            ),
            (
                "missing info opts",
                "info",
                MessageError::MissingValue { keyword: "info" },
            ),
            (
                "missing info string opts",
                "info string",
                MessageError::MissingValue { keyword: "string" },
            ),
            (
                "command token is not an arg",
                "info depth 1",
                MessageError::UnexpectedToken("depth".to_string()),
            ),
            (
                "missing bestmove opts",
                "bestmove",
                MessageError::MissingValue {
                    keyword: "bestmove",
                },
            ),
            (
                "invalid bestmove",
                "bestmove e7e9",
                MessageError::InvalidMove {
                    text: "e7e9".to_string(),
                    source: UciMoveError::InvalidSquare {
                        text: "e9".to_string(),
                        source: ParseError::InvalidSquare,
                    },
                },
            ),
            (
                "invalid bestmove keyword",
                "bestmove e2e4 hello",
                MessageError::UnexpectedToken("hello".to_string()),
            ),
            (
                "missing bestmove ponder args",
                "bestmove e2e4 ponder",
                MessageError::MissingValue { keyword: "ponder" },
            ),
            (
                "missing bestmove ponder keyword",
                "bestmove e2e4 e7e5",
                MessageError::UnexpectedToken("e7e5".to_string()),
            ),
            (
                "ponder some while bestmove is none",
                "bestmove 0000 ponder e7e5",
                MessageError::UnexpectedToken("ponder".to_string()),
            ),
            (
                "extra ponder move",
                "bestmove e2e4 ponder e7e5 e2e3",
                MessageError::UnexpectedToken("e2e3".to_string()),
            ),
            (
                "ponder given null move",
                "bestmove e2e4 ponder 0000",
                MessageError::InvalidMove {
                    text: "0000".to_string(),
                    source: UciMoveError::InvalidSquare {
                        text: "00".to_string(),
                        source: ParseError::InvalidSquare,
                    },
                },
            ),
        ];
        for (why, text, expected) in cases {
            assert_eq!(
                Err(expected),
                text.parse::<EngineMessage>(),
                "{why}: {text:?}"
            );
        }
    }
}
