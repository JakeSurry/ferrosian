//! UCI move text (`e2e4`, `e7e8q`): [`UciMove`] holds only the information needed to generate
//! or parse a UCI move.
//!
//! [`UciMove::resolve`] matches it against the legal moves to get a full [`Move`], since only
//! the board knows the kind (`e1g1` may be a castle or a king step).

use crate::{
    board::Board,
    movegen::MoveList,
    types::{Move, MoveKind, ParseError, PieceType, Square},
};
use std::{fmt, str::FromStr};
use thiserror::Error;

/// A move as UCI text: `from` and `to` squares plus an optional promotion piece (`e7e8q`).
///
/// Syntax only: every value is well formed, but it may not be legal, or even possible, on a
/// given board. The null move `0000` is not a `UciMove`; messages that allow it use
/// `Option<UciMove>`.
///
/// ```
/// use ferrosian::uci::UciMove;
///
/// let mv: UciMove = "e7e8q".parse().unwrap();
/// assert_eq!("e7e8q", mv.to_string());
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct UciMove {
    /// The square the piece leaves (the king's square for castling).
    pub from: Square,
    /// The square the piece lands on (the king's destination for castling).
    pub to: Square,
    /// The piece a pawn promotes to: knight, bishop, rook or queen.
    pub promotion: Option<PieceType>,
}

/// Parses lowercase UCI move text: two squares and an optional promotion letter (`nbrq`).
impl FromStr for UciMove {
    type Err = UciMoveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.is_ascii() {
            return Err(UciMoveError::NotAscii(s.to_string()));
        }

        if !(4..=5).contains(&s.len()) {
            return Err(UciMoveError::WrongLength(s.to_string()));
        }

        let square = |text: String| {
            text.parse()
                .map_err(|source| UciMoveError::InvalidSquare { text, source })
        };
        let from = square(s[..2].to_string())?;
        let to = square(s[2..4].to_string())?;

        let promotion = s
            .chars()
            .nth(4)
            .map(|c| PieceType::from_letter(c).ok_or(UciMoveError::InvalidPromotion(c)))
            .transpose()?
            .map(|p| {
                if PieceType::PROMOTIONS.contains(&p) {
                    Ok(p)
                } else {
                    Err(UciMoveError::InvalidPromotion(p.letter()))
                }
            })
            .transpose()?;

        Ok(Self {
            from,
            to,
            promotion,
        })
    }
}

impl fmt::Display for UciMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.from, self.to)?;
        if let Some(promotion) = self.promotion {
            write!(f, "{promotion}")?;
        }
        Ok(())
    }
}

/// Forgets the move kind. A promotion keeps its piece; every other kind has none.
impl From<Move> for UciMove {
    fn from(mv: Move) -> Self {
        let promotion = match mv.kind() {
            MoveKind::Promotion(p) => Some(p),
            _ => None,
        };
        Self {
            from: mv.from(),
            to: mv.to(),
            promotion,
        }
    }
}

impl UciMove {
    /// The legal move this text names, or `None` if there is none (including a promotion
    /// with no letter).
    ///
    /// Compares through [`From<Move>`](UciMove::from), so the two can never disagree. Assumes
    /// a validated position, like [`Board::legal_moves`].
    pub fn resolve(self, board: &Board) -> Option<Move> {
        let mut moves = MoveList::new();
        board.legal_moves(&mut moves);
        moves.iter().copied().find(|&mv| UciMove::from(mv) == self)
    }
}

/// Why a string is not UCI move syntax.
#[derive(Clone, PartialEq, Eq, Debug, Error)]
pub enum UciMoveError {
    /// Not 4 or 5 characters long.
    #[error("expected a string of length 4 or 5, got {0}")]
    WrongLength(String),

    /// The first or second pair of characters is not a square.
    #[error("expected valid square string, got {text:?}")]
    InvalidSquare { text: String, source: ParseError },

    /// The fifth character is not `n`, `b`, `r` or `q`.
    #[error("invalid promotion char {0:?}")]
    InvalidPromotion(char),

    /// Contains a non-ASCII character (checked first, so slicing by byte is safe).
    #[error("non-ascii string {0:?}")]
    NotAscii(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{
        KIWIPETE, PERFT3, PERFT4, PERFT5, PERFT6, STARTING_FEN, from_fen, sq,
    };
    use crate::types::CastleSide;
    use PieceType::{Bishop, Knight, Queen, Rook};

    /// `UciMove` from square names, without going through `FromStr` (which is under test).
    fn uci(from: &str, to: &str, promotion: Option<PieceType>) -> UciMove {
        UciMove {
            from: sq(from),
            to: sq(to),
            promotion,
        }
    }

    /// The CPW positions plus every position one legal move later. The CPW FENs are all
    /// White to move, so the children give the Black-to-move coverage.
    fn positions_and_children() -> Vec<Board> {
        let mut boards = Vec::new();
        let mut moves = MoveList::new();
        for fen in [STARTING_FEN, KIWIPETE, PERFT3, PERFT4, PERFT5, PERFT6] {
            let board = from_fen(fen);
            boards.push(board);
            board.legal_moves(&mut moves);
            for &mv in &moves {
                let mut child = board;
                child.make_move(mv);
                boards.push(child);
            }
        }
        boards
    }

    #[test]
    fn round_trip() {
        let cases = [
            ("e2e4", uci("e2", "e4", None)),
            ("a1h8", uci("a1", "h8", None)),
            ("h8a1", uci("h8", "a1", None)),
            ("e1g1", uci("e1", "g1", None)),
            ("e7e8q", uci("e7", "e8", Some(Queen))),
            ("e7e8r", uci("e7", "e8", Some(Rook))),
            ("e7e8b", uci("e7", "e8", Some(Bishop))),
            ("b2a1n", uci("b2", "a1", Some(Knight))),
        ];

        for (text, expected) in cases {
            assert_eq!(Ok(expected), text.parse::<UciMove>(), "parsing {text:?}");
            assert_eq!(text, expected.to_string(), "printing {expected:?}");
        }
    }

    #[test]
    fn parse_errors() {
        let invalid_square = |text: &str| UciMoveError::InvalidSquare {
            text: text.to_string(),
            source: ParseError::InvalidSquare,
        };
        let cases = [
            ("empty", "", UciMoveError::WrongLength(String::new())),
            (
                "too short",
                "e2e",
                UciMoveError::WrongLength("e2e".to_string()),
            ),
            (
                "too long",
                "e7e8qq",
                UciMoveError::WrongLength("e7e8qq".to_string()),
            ),
            // 4 bytes; rejected before slicing, whichever check comes first.
            (
                "non-ASCII, 4 bytes",
                "é2e",
                UciMoveError::NotAscii("é2e".to_string()),
            ),
            // 5 bytes; `s[2..4]` would split the 'é' and panic without the ASCII check.
            (
                "non-ASCII, 5 bytes",
                "e2eé",
                UciMoveError::NotAscii("e2eé".to_string()),
            ),
            ("bad from file", "i2e4", invalid_square("i2")),
            ("bad from rank", "e0e4", invalid_square("e0")),
            (
                "bad to square names the to half",
                "e2e9",
                invalid_square("e9"),
            ),
            ("uppercase square", "E2e4", invalid_square("E2")),
            (
                "no trimming: callers split on whitespace",
                " e2e4",
                invalid_square(" e"),
            ),
            (
                "king is not a promotion",
                "e7e8k",
                UciMoveError::InvalidPromotion('k'),
            ),
            (
                "pawn is not a promotion",
                "e7e8p",
                UciMoveError::InvalidPromotion('p'),
            ),
            (
                "UCI is lowercase",
                "e7e8Q",
                UciMoveError::InvalidPromotion('Q'),
            ),
            (
                "not a piece letter",
                "e7e8x",
                UciMoveError::InvalidPromotion('x'),
            ),
        ];

        for (why, text, expected) in cases {
            assert_eq!(Err(expected), text.parse::<UciMove>(), "{why}: {text:?}");
        }
    }

    #[test]
    fn from_move_prints_like_move() {
        let mut moves = MoveList::new();
        for board in positions_and_children() {
            board.legal_moves(&mut moves);
            for &mv in &moves {
                assert_eq!(
                    mv.to_string(),
                    UciMove::from(mv).to_string(),
                    "{mv:?} in {}",
                    board.fen()
                );
            }
        }
    }

    #[test]
    fn from_move_keeps_only_the_promotion_piece() {
        let cases = [
            (MoveKind::Normal, None),
            (MoveKind::DoublePush, None),
            (MoveKind::EnPassant, None),
            (MoveKind::Castle(CastleSide::KingSide), None),
            (MoveKind::Promotion(Knight), Some(Knight)),
            (MoveKind::Promotion(Queen), Some(Queen)),
        ];

        for (kind, expected) in cases {
            let mv = Move::new(sq("e7"), sq("e8"), kind);
            assert_eq!(uci("e7", "e8", expected), UciMove::from(mv), "{kind:?}");
        }
    }

    // ---- resolve ----

    #[test]
    fn resolve_rows() {
        use CastleSide::{KingSide, QueenSide};
        use MoveKind::{Castle, DoublePush, EnPassant, Normal, Promotion};

        const CASTLING_W: &str = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
        const CASTLING_B: &str = "r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1";
        const KINGS: &str = "4k3/8/8/8/8/8/8/4K3 w - - 0 1";

        // (why, fen, move text, expected kind; None = illegal)
        let cases: &[(&str, &str, &str, Option<MoveKind>)] = &[
            // pawn pushes, both colors
            ("white single push", STARTING_FEN, "e2e3", Some(Normal)),
            ("white double push", STARTING_FEN, "e2e4", Some(DoublePush)),
            (
                "black double push",
                "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
                "e7e5",
                Some(DoublePush),
            ),
            (
                "black push to the second rank (u8 underflow before the fix)",
                "4k3/8/8/8/8/4p3/8/K7 b - - 0 1",
                "e3e2",
                Some(Normal),
            ),
            (
                "black capture onto the second rank",
                "4k3/8/8/8/8/1p6/P7/4K3 b - - 0 1",
                "b3a2",
                Some(Normal),
            ),
            // en passant, both colors
            (
                "white en passant",
                "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1",
                "e5d6",
                Some(EnPassant),
            ),
            (
                "black en passant",
                "4k3/8/8/8/3Pp3/8/8/4K3 b - d3 0 1",
                "e4d3",
                Some(EnPassant),
            ),
            (
                "diagonal pawn move to an empty square without an en passant square",
                "4k3/8/8/3pP3/8/8/8/4K3 w - - 0 1",
                "e5d6",
                None,
            ),
            // castling, both colors and sides
            ("white kingside", CASTLING_W, "e1g1", Some(Castle(KingSide))),
            (
                "white queenside",
                CASTLING_W,
                "e1c1",
                Some(Castle(QueenSide)),
            ),
            ("black kingside", CASTLING_B, "e8g8", Some(Castle(KingSide))),
            (
                "black queenside",
                CASTLING_B,
                "e8c8",
                Some(Castle(QueenSide)),
            ),
            (
                "castling without the right",
                "r3k2r/8/8/8/8/8/8/R3K2R w Qkq - 0 1",
                "e1g1",
                None,
            ),
            // king steps
            (
                "straight king step (rejected before the fix)",
                KINGS,
                "e1e2",
                Some(Normal),
            ),
            ("diagonal king step", KINGS, "e1f2", Some(Normal)),
            (
                "king two squares with no castling setup",
                KINGS,
                "e1g1",
                None,
            ),
            // promotions, both colors
            (
                "white promotion to queen",
                "k7/4P3/8/8/8/8/8/4K3 w - - 0 1",
                "e7e8q",
                Some(Promotion(Queen)),
            ),
            (
                "white underpromotion",
                "k7/4P3/8/8/8/8/8/4K3 w - - 0 1",
                "e7e8n",
                Some(Promotion(Knight)),
            ),
            (
                "white capture-promotion",
                "k2r4/4P3/8/8/8/8/8/4K3 w - - 0 1",
                "e7d8r",
                Some(Promotion(Rook)),
            ),
            (
                "black promotion",
                "4k3/8/8/8/8/8/4p3/K7 b - - 0 1",
                "e2e1b",
                Some(Promotion(Bishop)),
            ),
            (
                "white promotion without a letter",
                "k7/4P3/8/8/8/8/8/4K3 w - - 0 1",
                "e7e8",
                None,
            ),
            (
                "black promotion without a letter",
                "4k3/8/8/8/8/8/4p3/K7 b - - 0 1",
                "e2e1",
                None,
            ),
            (
                "promotion letter on a non-promotion",
                STARTING_FEN,
                "e2e4q",
                None,
            ),
            // illegal for other reasons
            (
                "pinned knight",
                "4k3/4r3/8/8/8/8/4N3/4K3 w - - 0 1",
                "e2c3",
                None,
            ),
            (
                "king into check",
                "4k3/8/8/8/8/8/3r4/4K3 w - - 0 1",
                "e1e2",
                None,
            ),
            ("from an empty square", KINGS, "d4d5", None),
            ("opponent's piece", STARTING_FEN, "e7e5", None),
            ("rook through its own pawn", STARTING_FEN, "a1a3", None),
        ];

        for &(why, fen, text, expected) in cases {
            let parsed: UciMove = text.parse().expect("valid UCI text in test");
            let expected = expected.map(|kind| Move::new(parsed.from, parsed.to, kind));
            assert_eq!(
                expected,
                parsed.resolve(&from_fen(fen)),
                "{why}: {text} in {fen}"
            );
        }
    }

    #[test]
    fn every_legal_move_resolves_to_itself() {
        let mut moves = MoveList::new();
        for board in positions_and_children() {
            board.legal_moves(&mut moves);
            for &mv in &moves {
                let text = mv.to_string();
                let parsed: UciMove = text.parse().expect("legal move prints as valid UCI");
                assert_eq!(
                    Some(mv),
                    parsed.resolve(&board),
                    "{text} in {}",
                    board.fen()
                );
            }
        }
    }
}
