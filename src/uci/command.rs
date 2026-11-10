//! GUI-to-engine UCI commands: what the engine parses and what a UCI client prints.
//!
//! The other direction is [`EngineMessage`](super::EngineMessage).

use super::{UciMove, UciMoveError};
use crate::board::{Board, FenError, PositionErrors};
use std::{
    fmt,
    num::ParseIntError,
    str::{FromStr, SplitWhitespace},
    time::Duration,
};
use thiserror::Error;

/// Why a line is not a valid [`GuiCommand`].
#[derive(Clone, PartialEq, Eq, Debug, Error)]
pub enum CommandError {
    /// The line has no tokens.
    #[error("empty command")]
    Empty,

    /// The first token names no command.
    #[error("unknown command {0:?}")]
    UnknownCommand(String),

    /// A token that does not fit where it appears, including anything after a complete command.
    #[error("unexpected token {0:?}")]
    UnexpectedToken(String),

    /// The line ends where `keyword` needs a value.
    #[error("missing value after {keyword}")]
    MissingValue { keyword: &'static str },

    /// A move after `moves` or `searchmoves` is not UCI move syntax.
    #[error("invalid move {text:?}")]
    InvalidMove { text: String, source: UciMoveError },

    /// The text after `position fen` is not FEN syntax.
    #[error("invalid FEN")]
    InvalidFen { source: FenError },

    /// The value after a `go` keyword is not a number of the right type.
    #[error("invalid number {text:?} after {keyword}")]
    InvalidNumber {
        keyword: &'static str,
        text: String,
        source: ParseIntError,
    },
}

/// Why a [`PositionCommand`] does not describe a playable position.
#[derive(Clone, PartialEq, Eq, Debug, Error)]
pub enum PositionCommandError {
    /// The FEN base fails [`Board::validate`].
    #[error("invalid FEN position")]
    InvalidPosition { source: PositionErrors },

    /// Move `index` (counted from 0) is not legal on the board it is played on.
    #[error("illegal move {mv} at index {index}")]
    IllegalMove { index: usize, mv: UciMove },
}

/// A command from the GUI to the engine.
///
/// `FromStr` parses one line, strictly: unknown commands and stray tokens are errors, and the
/// engine loop decides to ignore them. `Display` prints the canonical line without its
/// newline.
///
/// ```
/// use chess::uci::GuiCommand;
///
/// let cmd: GuiCommand = "go  depth 5 wtime 1000".parse().unwrap();
/// assert_eq!("go wtime 1000 depth 5", cmd.to_string());
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum GuiCommand {
    /// `uci`: switch to UCI mode; the engine answers with `id` lines and `uciok`.
    Uci,

    /// `isready`: the engine answers `readyok` once it can take commands.
    IsReady,

    /// `ucinewgame`: the next position is from a new game.
    UciNewGame,

    /// `stop`: end the current search and send `bestmove`.
    Stop,

    /// `ponderhit`: the opponent played the expected move; keep searching as a normal search.
    PonderHit,

    /// `quit`: exit as soon as possible.
    Quit,

    /// `debug on` or `debug off`.
    Debug(bool),

    /// `position (startpos | fen <fen>) [moves <move>...]`.
    Position(PositionCommand),

    /// `go [<param>...]`: start searching the current position.
    Go(GoParams),

    /// `setoption name <name> [value <value>]`. Both may contain spaces; they stay strings
    /// until engine options exist.
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
            "go" => parse_go(&mut tokens)?,
            "setoption" => parse_setoption(&mut tokens)?,
            other => return Err(CommandError::UnknownCommand(other.to_string())),
        };

        expect_end(tokens)?;
        Ok(command)
    }
}

/// Fails on the first token left over after a complete command.
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

/// The next token parsed as a number, for the value after `keyword`.
fn number<T: FromStr<Err = ParseIntError>>(
    keyword: &'static str,
    tokens: &mut SplitWhitespace<'_>,
) -> Result<T, CommandError> {
    match tokens.next() {
        Some(n) => n
            .parse::<T>()
            .map_err(|source| CommandError::InvalidNumber {
                keyword,
                text: n.to_string(),
                source,
            }),
        None => Err(CommandError::MissingValue { keyword }),
    }
}

/// The next token as milliseconds. Negative values (a GUI's clock under lag) clamp to zero.
fn millis(
    keyword: &'static str,
    tokens: &mut SplitWhitespace<'_>,
) -> Result<Duration, CommandError> {
    let ms = number::<i64>(keyword, tokens)?;
    Ok(Duration::from_millis(u64::try_from(ms).unwrap_or(0)))
}

/// Parses the rest of a `debug` line: `on` or `off`.
fn parse_debug(tokens: &mut SplitWhitespace<'_>) -> Result<GuiCommand, CommandError> {
    match tokens.next() {
        None => Err(CommandError::MissingValue { keyword: "debug" }),
        Some("on") => Ok(GuiCommand::Debug(true)),
        Some("off") => Ok(GuiCommand::Debug(false)),
        Some(w) => Err(CommandError::UnexpectedToken(w.to_string())),
    }
}

/// Parses the rest of a `position` line: the base, then any moves after `moves`.
fn parse_position(tokens: &mut SplitWhitespace<'_>) -> Result<GuiCommand, CommandError> {
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
        ["fen"] => return Err(CommandError::MissingValue { keyword: "fen" }),
        [] => {
            return Err(CommandError::MissingValue {
                keyword: "position",
            });
        }
        ["startpos", extra, ..] | [extra, ..] => {
            return Err(CommandError::UnexpectedToken(extra.to_string()));
        }
    };
    let moves = parse_moves(move_words.unwrap_or_default())?;
    Ok(GuiCommand::Position(PositionCommand { base, moves }))
}

/// Parses the rest of a `go` line. Keywords may come in any order; `searchmoves` takes the
/// rest of the line.
fn parse_go(tokens: &mut SplitWhitespace<'_>) -> Result<GuiCommand, CommandError> {
    let mut params = GoParams::default();
    while let Some(keyword) = tokens.next() {
        match keyword {
            "wtime" => params.wtime = Some(millis("wtime", tokens)?),
            "btime" => params.btime = Some(millis("btime", tokens)?),
            "winc" => params.winc = Some(millis("winc", tokens)?),
            "binc" => params.binc = Some(millis("binc", tokens)?),
            "movestogo" => params.moves_to_go = Some(number("movestogo", tokens)?),
            "depth" => params.depth = Some(number("depth", tokens)?),
            "nodes" => params.nodes = Some(number("nodes", tokens)?),
            "mate" => params.mate = Some(number("mate", tokens)?),
            "movetime" => params.move_time = Some(millis("movetime", tokens)?),
            "infinite" => params.infinite = true,
            "ponder" => params.ponder = true,
            "searchmoves" => params.search_moves = parse_moves(&tokens.collect::<Vec<_>>())?,
            other => return Err(CommandError::UnexpectedToken(other.to_string())),
        }
    }
    Ok(GuiCommand::Go(params))
}

/// Parses the rest of a `setoption` line: `name <words> [value <words>]`.
fn parse_setoption(tokens: &mut SplitWhitespace<'_>) -> Result<GuiCommand, CommandError> {
    let words: Vec<&str> = tokens.collect();
    let ["name", rest @ ..] = words.as_slice() else {
        return Err(match words.first() {
            Some(w) => CommandError::UnexpectedToken(w.to_string()),
            None => CommandError::MissingValue {
                keyword: "setoption",
            },
        });
    };

    match split_at_keyword(rest, "value") {
        (&[], _) => Err(CommandError::MissingValue { keyword: "name" }),
        (_, Some(&[])) => Err(CommandError::MissingValue { keyword: "value" }),
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
            Self::Go(gc) => write!(f, "{gc}"),
            Self::SetOption { name, value } => {
                write!(f, "setoption name {name}")?;
                match value {
                    Some(v) => write!(f, " value {v}"),
                    None => Ok(()),
                }
            }
        }
    }
}

/// The payload of a `position` command: a starting position and the moves played from it.
///
/// Parsing checks syntax only: the FEN base is not validated and the moves are not checked
/// for legality. [`PositionCommand::to_board`] does both and returns the resulting position.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PositionCommand {
    /// The position before any moves.
    pub base: PositionBase,

    /// The moves played from `base`, in order.
    pub moves: Vec<UciMove>,
}

impl PositionCommand {
    /// The position this command describes: the base with every move played in order.
    ///
    /// Each move is resolved with [`UciMove::resolve`] on the board it is played on.
    ///
    /// # Errors
    ///
    /// [`PositionCommandError::InvalidPosition`] if the FEN base fails [`Board::validate`], and
    /// [`PositionCommandError::IllegalMove`] for the first move that is not legal where it is
    /// played. Later moves are not looked at.
    pub fn to_board(&self) -> Result<Board, PositionCommandError> {
        let mut board = match &self.base {
            PositionBase::StartPos => Board::starting_position(),
            PositionBase::Fen(board) => {
                board
                    .validate()
                    .map_err(|source| PositionCommandError::InvalidPosition { source })?;
                **board
            }
        };
        for (index, &uci) in self.moves.iter().enumerate() {
            let mv = uci
                .resolve(&board)
                .ok_or(PositionCommandError::IllegalMove { index, mv: uci })?;
            board.make_move(mv);
        }
        Ok(board)
    }
}

impl fmt::Display for PositionCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "position {}", self.base)?;
        write_moves(f, "moves", self.moves.as_slice())
    }
}

/// Writes ` <prefix> <move>...`, or nothing when `moves` is empty.
fn write_moves(f: &mut fmt::Formatter<'_>, prefix: &'static str, moves: &[UciMove]) -> fmt::Result {
    if moves.is_empty() {
        return Ok(());
    }
    write!(f, " {prefix}")?;
    for mv in moves {
        write!(f, " {mv}")?;
    }
    Ok(())
}

/// Where a `position` command starts.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PositionBase {
    /// `startpos`: the standard starting position.
    StartPos,

    /// `fen <fen>`: a position parsed from FEN (syntax only, not yet validated). Boxed so the
    /// enum stays small.
    Fen(Box<Board>),
}

impl fmt::Display for PositionBase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StartPos => f.write_str("startpos"),
            Self::Fen(board) => write!(f, "fen {}", board.fen()),
        }
    }
}

/// The search limits of a `go` command. `Default` is a bare `go`.
///
/// Times are [`Duration`]s; milliseconds exist only in the text. A negative time (a GUI's
/// clock under lag) parses as zero. `Display` prints the fields in declaration order and skips
/// the ones that are unset.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct GoParams {
    /// `wtime`: White's remaining clock time.
    pub wtime: Option<Duration>,

    /// `btime`: Black's remaining clock time.
    pub btime: Option<Duration>,

    /// `winc`: White's increment per move.
    pub winc: Option<Duration>,

    /// `binc`: Black's increment per move.
    pub binc: Option<Duration>,

    /// `movestogo`: moves until the next time control.
    pub moves_to_go: Option<u32>,

    /// `depth`: search this many plies only.
    pub depth: Option<u32>,

    /// `nodes`: search this many nodes only.
    pub nodes: Option<u64>,

    /// `mate`: look for a mate in this many moves.
    pub mate: Option<u32>,

    /// `movetime`: search exactly this long.
    pub move_time: Option<Duration>,

    /// `infinite`: search until `stop`.
    pub infinite: bool,

    /// `ponder`: search in pondering mode until `ponderhit` or `stop`.
    pub ponder: bool,

    /// `searchmoves`: consider only these root moves. Empty means all.
    pub search_moves: Vec<UciMove>,
}

impl fmt::Display for GoParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("go")?;
        write_opt(f, "wtime", self.wtime.map(|d| d.as_millis()))?;
        write_opt(f, "btime", self.btime.map(|d| d.as_millis()))?;
        write_opt(f, "winc", self.winc.map(|d| d.as_millis()))?;
        write_opt(f, "binc", self.binc.map(|d| d.as_millis()))?;
        write_opt(f, "movestogo", self.moves_to_go)?;
        write_opt(f, "depth", self.depth)?;
        write_opt(f, "nodes", self.nodes)?;
        write_opt(f, "mate", self.mate)?;
        write_opt(f, "movetime", self.move_time.map(|d| d.as_millis()))?;
        if self.infinite {
            f.write_str(" infinite")?;
        }
        if self.ponder {
            f.write_str(" ponder")?;
        }
        write_moves(f, "searchmoves", &self.search_moves)
    }
}

/// Writes ` <keyword> <value>`, or nothing when `value` is `None`.
fn write_opt(
    f: &mut fmt::Formatter<'_>,
    keyword: &str,
    value: Option<impl fmt::Display>,
) -> fmt::Result {
    match value {
        Some(v) => write!(f, " {keyword} {v}"),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::PositionError;
    use crate::movegen::MoveList;
    use crate::test_helpers::{KIWIPETE, PERFT3, PERFT4, PERFT5, PERFT6, STARTING_FEN, from_fen};
    use crate::types::{Color, ParseError};
    use PositionBase::*;
    use std::num::IntErrorKind;

    fn ucims(moves: &[&str]) -> Vec<UciMove> {
        moves
            .iter()
            .map(|mv| mv.parse::<UciMove>().unwrap())
            .collect()
    }

    fn position(base: PositionBase, moves: &[&str]) -> GuiCommand {
        GuiCommand::Position(position_command(base, moves))
    }

    fn custom_board() -> Board {
        from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1")
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn go(params: GoParams) -> GuiCommand {
        GuiCommand::Go(params)
    }

    fn position_command(base: PositionBase, moves: &[&str]) -> PositionCommand {
        PositionCommand {
            base,
            moves: ucims(moves),
        }
    }

    fn fen(text: &str) -> PositionBase {
        Fen(Box::new(from_fen(text)))
    }

    /// The `ParseIntError` that parsing `text` as a `T` gives. `ParseIntError` has no public
    /// constructor, and its `PartialEq` compares only the `IntErrorKind`, so parse with the same
    /// type the field uses.
    fn int_error<T: FromStr<Err = ParseIntError> + fmt::Debug>(text: &str) -> ParseIntError {
        text.parse::<T>().unwrap_err()
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
            ("go", go(GoParams::default())),
            (
                "go infinite",
                go(GoParams {
                    infinite: true,
                    ..GoParams::default()
                }),
            ),
            (
                "go depth 5",
                go(GoParams {
                    depth: Some(5),
                    ..GoParams::default()
                }),
            ),
            (
                "go movetime 1000",
                go(GoParams {
                    move_time: Some(ms(1000)),
                    ..GoParams::default()
                }),
            ),
            (
                "go wtime 300000 btime 300000 winc 2000 binc 2000 movestogo 40",
                go(GoParams {
                    wtime: Some(ms(300000)),
                    btime: Some(ms(300000)),
                    winc: Some(ms(2000)),
                    binc: Some(ms(2000)),
                    moves_to_go: Some(40),
                    ..GoParams::default()
                }),
            ),
            (
                "go wtime 0",
                go(GoParams {
                    wtime: Some(ms(0)),
                    ..GoParams::default()
                }),
            ),
            (
                "go wtime 1 btime 2 winc 3 binc 4 movestogo 5 depth 6 nodes 7 mate 8 movetime 9 infinite ponder searchmoves e2e4 d2d4",
                go(GoParams {
                    wtime: Some(ms(1)),
                    btime: Some(ms(2)),
                    winc: Some(ms(3)),
                    binc: Some(ms(4)),
                    moves_to_go: Some(5),
                    depth: Some(6),
                    nodes: Some(7),
                    mate: Some(8),
                    move_time: Some(ms(9)),
                    infinite: true,
                    ponder: true,
                    search_moves: ucims(&["e2e4", "d2d4"]),
                }),
            ),
            (
                "go depth 5 movetime 100",
                go(GoParams {
                    depth: Some(5),
                    move_time: Some(ms(100)),
                    ..GoParams::default()
                }),
            ),
            (
                "go nodes 1000000",
                go(GoParams {
                    nodes: Some(1_000_000),
                    ..GoParams::default()
                }),
            ),
            (
                "go mate 3",
                go(GoParams {
                    mate: Some(3),
                    ..GoParams::default()
                }),
            ),
            (
                "go ponder",
                go(GoParams {
                    ponder: true,
                    ..GoParams::default()
                }),
            ),
            (
                "go searchmoves e2e4 d2d4 g1f3",
                go(GoParams {
                    search_moves: ucims(&["e2e4", "d2d4", "g1f3"]),
                    ..GoParams::default()
                }),
            ),
            (
                "setoption name Hash value 64",
                GuiCommand::SetOption {
                    name: "Hash".to_string(),
                    value: Some("64".to_string()),
                },
            ),
            (
                "setoption name Clear Hash",
                GuiCommand::SetOption {
                    name: "Clear Hash".to_string(),
                    value: None,
                },
            ),
            (
                "setoption name Book File value /home/jake/my book.bin",
                GuiCommand::SetOption {
                    name: "Book File".to_string(),
                    value: Some("/home/jake/my book.bin".to_string()),
                },
            ),
        ];
        for (text, command) in cases {
            assert_eq!(text, command.to_string(), "printing {command:?}");
            assert_eq!(Ok(command), text.parse::<GuiCommand>(), "parsing {text:?}");
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
            (
                "go args unordered",
                "go ponder wtime 1000",
                go(GoParams {
                    wtime: Some(ms(1000)),
                    ponder: true,
                    ..GoParams::default()
                }),
            ),
            (
                "negative time clamps to 0",
                "go wtime -1",
                go(GoParams {
                    wtime: Some(ms(0)),
                    ..GoParams::default()
                }),
            ),
            (
                "last duplicate arg wins",
                "go depth 3 depth 5",
                go(GoParams {
                    depth: Some(5),
                    ..GoParams::default()
                }),
            ),
            (
                "empty searchmoves list defaults",
                "go searchmoves",
                go(GoParams::default()),
            ),
            (
                "keywords print in declaration order",
                "go movetime 100 depth 5",
                go(GoParams {
                    depth: Some(5),
                    move_time: Some(ms(100)),
                    ..GoParams::default()
                }),
            ),
            (
                "only the negative time clamps",
                "go wtime -300000 btime 300000",
                go(GoParams {
                    wtime: Some(ms(0)),
                    btime: Some(ms(300_000)),
                    ..GoParams::default()
                }),
            ),
            (
                "inner whitespace collapses to one space",
                "setoption name  Clear   Hash",
                GuiCommand::SetOption {
                    name: "Clear Hash".to_string(),
                    value: None,
                },
            ),
        ];
        for (why, text, expected) in cases {
            assert_ne!(
                text,
                expected.to_string(),
                "{why}: {text:?} is canonical, move it to the round-trip table"
            );
            assert_eq!(Ok(expected), text.parse::<GuiCommand>(), "{why}: {text:?}");
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
                CommandError::MissingValue { keyword: "debug" },
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
                    keyword: "position",
                },
            ),
            (
                "missing fen opts",
                "position fen",
                CommandError::MissingValue { keyword: "fen" },
            ),
            (
                "no fields before moves",
                "position fen moves e2e4",
                CommandError::MissingValue { keyword: "fen" },
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
            (
                "missing arg with no defaults",
                "go depth",
                CommandError::MissingValue { keyword: "depth" },
            ),
            (
                "time with no value",
                "go wtime",
                CommandError::MissingValue { keyword: "wtime" },
            ),
            (
                "missing value is noticed after other keywords",
                "go depth 5 movetime",
                CommandError::MissingValue {
                    keyword: "movetime",
                },
            ),
            (
                "not a number",
                "go depth x",
                CommandError::InvalidNumber {
                    keyword: "depth",
                    text: "x".to_string(),
                    source: int_error::<u32>("x"),
                },
            ),
            (
                "only times clamp: depth has no sign",
                "go depth -1",
                CommandError::InvalidNumber {
                    keyword: "depth",
                    text: "-1".to_string(),
                    source: int_error::<u32>("-1"),
                },
            ),
            (
                "depth overflows u32",
                "go depth 99999999999",
                CommandError::InvalidNumber {
                    keyword: "depth",
                    text: "99999999999".to_string(),
                    source: int_error::<u32>("99999999999"),
                },
            ),
            (
                "times are whole milliseconds",
                "go wtime 1.5",
                CommandError::InvalidNumber {
                    keyword: "wtime",
                    text: "1.5".to_string(),
                    source: int_error::<i64>("1.5"),
                },
            ),
            (
                "no scientific notation",
                "go nodes 1e6",
                CommandError::InvalidNumber {
                    keyword: "nodes",
                    text: "1e6".to_string(),
                    source: int_error::<u64>("1e6"),
                },
            ),
            (
                "unknown go keyword",
                "go fast",
                CommandError::UnexpectedToken("fast".to_string()),
            ),
            (
                "value is one token",
                "go depth 5 6",
                CommandError::UnexpectedToken("6".to_string()),
            ),
            (
                "searchmoves takes the rest of the line",
                "go searchmoves e2e4 depth 5",
                CommandError::InvalidMove {
                    text: "depth".to_string(),
                    source: UciMoveError::InvalidSquare {
                        text: "de".to_string(),
                        source: ParseError::InvalidSquare,
                    },
                },
            ),
            (
                "set option with no args",
                "setoption",
                CommandError::MissingValue {
                    keyword: "setoption",
                },
            ),
            (
                "forgot name",
                "setoption Hash value 64",
                CommandError::UnexpectedToken("Hash".to_string()),
            ),
            (
                "name with no args",
                "setoption name",
                CommandError::MissingValue { keyword: "name" },
            ),
            (
                "name with no args with trailing keyword",
                "setoption name value 64",
                CommandError::MissingValue { keyword: "name" },
            ),
            (
                "value with no args",
                "setoption name Hash value",
                CommandError::MissingValue { keyword: "value" },
            ),
        ];
        for (why, text, expected) in cases {
            assert_eq!(Err(expected), text.parse::<GuiCommand>(), "{why}: {text:?}");
        }
    }

    /// The error rows compare `ParseIntError`s by kind against `int_error`, so a row passes
    /// even if both fail for the same wrong reason. This pins the kinds independently.
    #[test]
    fn go_number_error_kinds() {
        let cases = [
            ("go depth x", IntErrorKind::InvalidDigit),
            ("go depth -1", IntErrorKind::InvalidDigit),
            ("go depth 99999999999", IntErrorKind::PosOverflow),
            ("go wtime 1.5", IntErrorKind::InvalidDigit),
            ("go wtime -99999999999999999999", IntErrorKind::NegOverflow),
        ];
        for (text, expected) in cases {
            let error = text.parse::<GuiCommand>().unwrap_err();
            let CommandError::InvalidNumber { source, .. } = &error else {
                panic!("{text:?}: expected InvalidNumber, got {error:?}");
            };
            assert_eq!(&expected, source.kind(), "{text:?}");
        }
    }

    #[test]
    fn position_round_trips_cpw() {
        let mut moves = MoveList::new();
        let mut child_moves = MoveList::new();
        for fen in [STARTING_FEN, KIWIPETE, PERFT3, PERFT4, PERFT5, PERFT6] {
            let board = from_fen(fen);
            board.legal_moves(&mut moves);
            let children = moves.iter().map(|&mv| {
                let mut child = board;
                child.make_move(mv);
                child
            });
            for position in std::iter::once(board).chain(children) {
                position.legal_moves(&mut child_moves);
                let command = GuiCommand::Position(PositionCommand {
                    base: Fen(Box::new(position)),
                    moves: child_moves.iter().map(|&mv| UciMove::from(mv)).collect(),
                });
                let text = command.to_string();
                assert_eq!(Ok(command), text.parse::<GuiCommand>(), "parsing {text:?}");
            }
        }
    }

    #[test]
    fn legal_sequences_give_exact_fen() {
        let cases: [(&str, PositionBase, &[&str], &str); 7] = [
            ("no moves", StartPos, &[], STARTING_FEN),
            (
                "double push sets en passant",
                StartPos,
                &["e2e4"],
                "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
            ),
            (
                "both colors, clocks advance",
                StartPos,
                &["e2e4", "e7e5", "g1f3"],
                "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2",
            ),
            (
                "en passant only resolves on the board after d7d5",
                StartPos,
                &["e2e4", "a7a6", "e4e5", "d7d5", "e5d6"],
                "rnbqkbnr/1pp1pppp/p2P4/8/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 3",
            ),
            (
                "FEN base, castling",
                fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"),
                &["e1g1"],
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R4RK1 b kq - 1 1",
            ),
            (
                "promotion",
                fen("4k3/P7/8/8/8/8/8/4K3 w - - 0 1"),
                &["a7a8q"],
                "Q3k3/8/8/8/8/8/8/4K3 b - - 0 1",
            ),
            (
                "underpromotion",
                fen("4k3/P7/8/8/8/8/8/4K3 w - - 0 1"),
                &["a7a8n"],
                "N3k3/8/8/8/8/8/8/4K3 b - - 0 1",
            ),
        ];
        for (why, base, moves, expected) in cases {
            let cmd = position_command(base, moves);
            assert_eq!(expected, cmd.to_board().unwrap().to_fen(), "{cmd} ({why})");
        }
    }

    #[test]
    fn valid_fen_with_no_moves_is_unchanged() {
        let cases = [STARTING_FEN, KIWIPETE, PERFT3, PERFT4, PERFT5, PERFT6];
        for case in cases {
            let actual = position_command(fen(case), &[]).to_board().unwrap();
            assert_eq!(from_fen(case), actual, "{case}");
        }
    }

    #[test]
    fn illegal_moves_give_exact_index_and_move() {
        let cases: [(&str, PositionBase, &[&str], usize, &str); 6] = [
            (
                "black's move with white to move",
                StartPos,
                &["e7e5"],
                0,
                "e7e5",
            ),
            ("empty from-square", StartPos, &["a3a4"], 0, "a3a4"),
            (
                "stops at the first illegal move",
                StartPos,
                &["e2e5", "e2e4"],
                0,
                "e2e5",
            ),
            (
                "second illegal move is caught after making first move",
                StartPos,
                &["d2d4", "e2e4"],
                1,
                "e2e4",
            ),
            (
                "castling through own pieces",
                StartPos,
                &["e2e4", "e7e5", "e1g1"],
                2,
                "e1g1",
            ),
            (
                "promotion with no promotion piece letter",
                fen("4k3/P7/8/8/8/8/8/4K3 w - - 0 1"),
                &["a7a8"],
                0,
                "a7a8",
            ),
        ];
        for (why, base, moves, index, mv) in cases {
            let cmd = position_command(base, moves);
            let expected = PositionCommandError::IllegalMove {
                index,
                mv: mv.parse().unwrap(),
            };
            assert_eq!(Some(expected), cmd.to_board().err(), "{cmd} ({why})");
        }
    }

    #[test]
    fn invalid_position() {
        let cmd = position_command(fen("4k3/8/8/8/8/8/8/4KK2 w - - 0 1"), &["e1d1"]);
        let Some(PositionCommandError::InvalidPosition { source }) = cmd.to_board().err() else {
            panic!("{cmd}: expected InvalidPosition, got {:?}", cmd.to_board());
        };
        assert_eq!(
            &[PositionError::InvalidKingCount {
                color: Color::White,
                count: 2
            }],
            source.errors(),
            "{cmd}",
        );
    }
}
