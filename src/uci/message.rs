use super::uci_move::{UciMove, UciMoveError};
use crate::uci::MessageError::UnexpectedToken;
use std::{
    fmt,
    str::{FromStr, SplitWhitespace},
};
use thiserror::Error;

#[derive(Clone, PartialEq, Eq, Debug, Error)]
pub enum MessageError {
    #[error("empty message")]
    Empty,

    #[error("unknown message {0:?}")]
    UnknownMessage(String),

    #[error("unexpected token {0:?}")]
    UnexpectedToken(String),

    #[error("missing value after {keyword}")]
    MissingValue { keyword: &'static str },

    #[error("invalid move {text:?}")]
    InvalidMove { text: String, source: UciMoveError },
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum EngineMessage {
    UciOk,
    ReadyOk,
    IdName(String),
    IdAuthor(String),
}

impl FromStr for EngineMessage {
    type Err = MessageError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tokens = s.split_whitespace();
        let Some(name) = tokens.next() else {
            return Err(MessageError::Empty);
        };
        let command = match name {
            "uciok" => Self::UciOk,
            "readyok" => Self::ReadyOk,
            "id" => parse_id(&mut tokens)?,
            other => Err(MessageError::UnexpectedToken(other.to_string()))?,
        };

        expect_end(&mut tokens)?;
        Ok(command)
    }
}

fn expect_end(tokens: &mut SplitWhitespace<'_>) -> Result<(), MessageError> {
    match tokens.next() {
        Some(token) => Err(MessageError::UnexpectedToken(token.to_string())),
        None => Ok(()),
    }
}

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

fn parse_id(tokens: &mut SplitWhitespace<'_>) -> Result<EngineMessage, MessageError> {
    match tokens.next() {
        Some("name") => Ok(EngineMessage::IdName(rest_of_line("name", tokens)?)),
        Some("author") => Ok(EngineMessage::IdAuthor(rest_of_line("author", tokens)?)),
        Some(other) => Err(MessageError::UnexpectedToken(other.to_string())),
        None => Err(MessageError::MissingValue { keyword: "id" }),
    }
}

impl fmt::Display for EngineMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UciOk => f.write_str("uciok"),
            Self::ReadyOk => f.write_str("readyok"),
            Self::IdName(s) => write!(f, "id name {s}"),
            Self::IdAuthor(s) => write!(f, "id author {s}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            (
                "collapsed spaces into value",
                "id name  Ferrosian  0.1.0",
                EngineMessage::IdName("Ferrosian 0.1.0".to_string()),
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
                MessageError::UnexpectedToken("UCIOK".to_string()),
            ),
            (
                "command is not an engine message",
                "isready",
                MessageError::UnexpectedToken("isready".to_string()),
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
