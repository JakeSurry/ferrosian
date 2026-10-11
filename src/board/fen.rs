use crate::board::Board;
use crate::types::{Color, Piece, Square};
use std::{error::Error, fmt, num::ParseIntError, str::FromStr};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FenError {
    /// The FEN did not have 4 or 6 whitespace-separated fields.
    WrongFieldCount(usize),
    /// The placement field did not contain 8 ranks.
    WrongRankCount(usize),
    /// A rank's pieces and empty-square counts did not add up to 8.
    /// `rank` is 1-based, as in chess notation.
    WrongRankLength {
        rank: u8,
        text: String,
    },
    /// A character in a rank that is neither a piece letter nor a digit 1-8,
    /// or a digit directly following another digit.
    InvalidRankChar {
        rank: u8,
        ch: char,
    },
    InvalidSideToMove(String),
    InvalidCastling(String),
    InvalidEnPassant(String),
    InvalidHalfmoveClock {
        text: String,
        source: ParseIntError,
    },
    InvalidFullmoveCount {
        text: String,
        source: ParseIntError,
    },
}

impl fmt::Display for FenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongFieldCount(n) => write!(f, "expected 4 or 6 fields, found {n}"),
            Self::WrongRankCount(n) => write!(f, "expected 8 ranks, found {n}"),
            Self::WrongRankLength { rank, text } => {
                write!(
                    f,
                    "rank {rank} ({text:?}) does not describe exactly 8 squares"
                )
            }
            Self::InvalidRankChar { rank, ch } => {
                write!(f, "invalid character {ch:?} in rank {rank}")
            }
            Self::InvalidSideToMove(s) => {
                write!(f, "invalid side to move {s:?}, expected \"w\" or \"b\"")
            }
            Self::InvalidCastling(s) => write!(f, "invalid castling rights {s:?}"),
            Self::InvalidEnPassant(s) => write!(f, "invalid en passant square {s:?}"),
            Self::InvalidHalfmoveClock { text, .. } => {
                write!(f, "invalid halfmove clock {text:?}")
            }
            Self::InvalidFullmoveCount { text, .. } => {
                write!(f, "invalid fullmove count {text:?}")
            }
        }
    }
}

impl Error for FenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidHalfmoveClock { source, .. }
            | Self::InvalidFullmoveCount { source, .. } => Some(source),
            _ => None,
        }
    }
}
impl FromStr for Board {
    type Err = FenError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let fields: Vec<&str> = s.split_whitespace().collect();
        let (placement, side_field, castling_field, ep_field, halfmove_field, fullmove_field) =
            match fields[..] {
                [p, s, c, e, h, f] => (p, s, c, e, Some(h), Some(f)),
                [p, s, c, e] => (p, s, c, e, None, None),
                _ => return Err(FenError::WrongFieldCount(fields.len())),
            };
        let mut board = Board::empty();

        parse_placement(&mut board, placement)?;

        board.side_to_move = match side_field {
            "w" => Color::White,
            "b" => Color::Black,
            other => return Err(FenError::InvalidSideToMove(other.to_string())),
        };

        board.castling_rights = castling_field
            .parse()
            .map_err(|_| FenError::InvalidCastling(castling_field.to_string()))?;

        board.en_passant_square = match ep_field {
            "-" => None,
            text => Some(
                text.parse()
                    .map_err(|_| FenError::InvalidEnPassant(text.to_string()))?,
            ),
        };

        board.halfmove_clock = match halfmove_field {
            None => 0,
            Some(text) => text
                .parse()
                .map_err(|source| FenError::InvalidHalfmoveClock {
                    text: text.to_string(),
                    source,
                })?,
        };

        board.fullmove_count = match fullmove_field {
            None => 1,
            Some(text) => text
                .parse()
                .map_err(|source| FenError::InvalidFullmoveCount {
                    text: text.to_string(),
                    source,
                })?,
        };

        debug_assert_eq!(board.is_consistent(), Ok(()));
        Ok(board)
    }
}

fn parse_placement(board: &mut Board, placement: &str) -> Result<(), FenError> {
    let ranks: Vec<&str> = placement.split('/').collect();
    if ranks.len() != 8 {
        return Err(FenError::WrongRankCount(ranks.len()));
    }

    for (i, rank_text) in ranks.iter().enumerate() {
        // FEN lists rank 8 first: i = 0 is rank index 7.
        let rank = 7 - i as u8;
        let wrong_length = || FenError::WrongRankLength {
            rank: rank + 1,
            text: rank_text.to_string(),
        };

        let mut file: u8 = 0;
        let mut previous_was_digit = false;

        for ch in rank_text.chars() {
            if let Some(n) = ch.to_digit(10) {
                // A digit: skip that many empty squares.
                if !(1..=8).contains(&n) || previous_was_digit {
                    return Err(FenError::InvalidRankChar { rank: rank + 1, ch });
                }
                file += n as u8;
                previous_was_digit = true;
            } else {
                // Anything else must be a piece letter.
                let piece = Piece::from_fen_char(ch)
                    .ok_or(FenError::InvalidRankChar { rank: rank + 1, ch })?;
                let square = Square::from_file_rank(file, rank).ok_or_else(wrong_length)?;
                board.put_piece(square, piece);
                file += 1;
                previous_was_digit = false;
            }
        }

        if file != 8 {
            return Err(wrong_length());
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
pub struct Fen<'a>(&'a Board);

impl Board {
    /// A value that displays this position as FEN.
    pub fn fen(&self) -> Fen<'_> {
        Fen(self)
    }

    /// The position in Forsyth–Edwards Notation.
    pub fn to_fen(&self) -> String {
        self.fen().to_string()
    }
}

impl fmt::Display for Fen<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let board = self.0;

        for rank in (0..8).rev() {
            let mut empty = 0;
            for file in 0..8 {
                let square = Square::from_file_rank(file, rank).expect("file and rank are below 8");
                match board.piece_at(square) {
                    Some(piece) => {
                        if empty > 0 {
                            write!(f, "{empty}")?;
                            empty = 0;
                        }
                        write!(f, "{piece}")?;
                    }
                    None => empty += 1,
                }
            }
            if empty > 0 {
                write!(f, "{empty}")?;
            }
            if rank > 0 {
                f.write_str("/")?;
            }
        }

        let side = match board.side_to_move() {
            Color::White => 'w',
            Color::Black => 'b',
        };
        write!(f, " {side} {} ", board.castling_rights())?;
        match board.en_passant_square() {
            Some(square) => write!(f, "{square}")?,
            None => f.write_str("-")?,
        }
        write!(f, " {} {}", board.halfmove_clock(), board.fullmove_count())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::*;
    use super::*;
    use crate::board::STARTING_FEN;
    use crate::types::{CastleSide, CastlingRights};

    const ROUND_TRIP_FENS: [&str; 7] = [
        STARTING_FEN,
        // en passant, black to move
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
        // Kiwipete
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        // no castling, sparse
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        // partial castling
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        // odd clocks
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/3P1N1P/PPP1NPP1/R4RK1 w - - 0 10",
    ];

    fn parse(fen: &str) -> Board {
        fen.parse()
            .unwrap_or_else(|e| panic!("failed to parse {fen:?}: {e}"))
    }

    fn parse_err(fen: &str) -> FenError {
        match fen.parse::<Board>() {
            Ok(_) => panic!("{fen:?} should have been rejected"),
            Err(e) => e,
        }
    }

    fn placement_err(placement: &str) -> FenError {
        parse_err(&format!("{placement} w - - 0 1"))
    }

    // Round trips

    #[test]
    fn fen_string_round_trip() {
        for fen in ROUND_TRIP_FENS {
            let board = parse(fen);
            assert_eq!(board.to_fen(), fen);
            assert_eq!(board.is_consistent(), Ok(()), "{fen}");
        }
    }

    #[test]
    fn fen_board_round_trip() {
        let mut board = board_with(&[
            ("e1", 'K'),
            ("e8", 'k'),
            ("a1", 'R'),
            ("h8", 'r'),
            ("d5", 'p'),
            ("e5", 'P'),
        ]);
        board.castling_rights = "Qk".parse().unwrap();
        board.en_passant_square = Some(sq("d6"));
        board.halfmove_clock = 0;
        board.fullmove_count = 42;

        assert_eq!(parse(&board.to_fen()), board);

        board.side_to_move = Color::Black;
        board.en_passant_square = None;
        board.halfmove_clock = 17;
        assert_eq!(parse(&board.to_fen()), board);
    }

    // to_fen output

    #[test]
    fn empty_board_to_fen() {
        assert_eq!(Board::empty().to_fen(), "8/8/8/8/8/8/8/8 w - - 0 1");
    }

    #[test]
    fn to_fen_counts_empty_squares() {
        // Pieces at the start and end of a rank, and one in the middle.
        let board = board_with(&[("a1", 'K'), ("h8", 'k'), ("d4", 'P')]);
        assert_eq!(board.to_fen(), "7k/8/8/8/3P4/8/8/K7 w - - 0 1");
    }

    #[test]
    fn starting_position_matches_fen() {
        assert_eq!(Board::starting_position().to_fen(), STARTING_FEN);
        assert_eq!(Board::starting_position(), parse(STARTING_FEN));
    }

    // Spot checks

    #[test]
    fn parse_starting_position() {
        let board = parse(STARTING_FEN);
        assert_eq!(board.piece_at(sq("e1")), Some(p('K')));
        assert_eq!(board.piece_at(sq("d8")), Some(p('q')));
        assert_eq!(board.piece_at(sq("a2")), Some(p('P')));
        assert_eq!(board.piece_at(sq("h7")), Some(p('p')));
        assert_eq!(board.piece_at(sq("e4")), None);
        assert_eq!(board.all_occupancy().count_ones(), 32);
        assert_eq!(board.side_to_move(), Color::White);
        assert_eq!(board.castling_rights(), CastlingRights::ALL);
        assert_eq!(board.en_passant_square(), None);
        assert_eq!(board.halfmove_clock(), 0);
        assert_eq!(board.fullmove_count(), 1);
    }

    #[test]
    fn parse_en_passant() {
        let board = parse("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1");
        assert_eq!(board.side_to_move(), Color::Black);
        assert_eq!(board.en_passant_square(), Some(sq("e3")));
        assert_eq!(board.piece_at(sq("e4")), Some(p('P')));
        assert_eq!(board.piece_at(sq("e2")), None);
    }

    #[test]
    fn parse_kiwipete() {
        let board = parse("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
        assert_eq!(board.piece_at(sq("e5")), Some(p('N')));
        assert_eq!(board.piece_at(sq("f3")), Some(p('Q')));
        assert_eq!(board.piece_at(sq("a6")), Some(p('b')));
        assert_eq!(board.piece_at(sq("h3")), Some(p('p')));
        assert_eq!(board.king_square(Color::White), Some(sq("e1")));
        assert_eq!(board.king_square(Color::Black), Some(sq("e8")));
        assert_eq!(board.all_occupancy().count_ones(), 32);
    }

    #[test]
    fn parse_partial_castling() {
        let board = parse("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1");
        let rights = board.castling_rights();
        assert!(!rights.has(Color::White, CastleSide::KingSide));
        assert!(!rights.has(Color::White, CastleSide::QueenSide));
        assert!(rights.has(Color::Black, CastleSide::KingSide));
        assert!(rights.has(Color::Black, CastleSide::QueenSide));
        assert_eq!(board.king_square(Color::White), Some(sq("g1")));
        assert_eq!(board.piece_at(sq("a7")), Some(p('P')));
    }

    #[test]
    fn parse_clocks() {
        let board = parse("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8");
        assert_eq!(board.halfmove_clock(), 1);
        assert_eq!(board.fullmove_count(), 8);
        assert_eq!(board.piece_at(sq("d7")), Some(p('P')));
        assert_eq!(board.piece_at(sq("f2")), Some(p('n')));
    }

    #[test]
    fn missing_clocks_default_to_0_and_1() {
        let board = parse("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -");
        assert_eq!(board.halfmove_clock(), 0);
        assert_eq!(board.fullmove_count(), 1);
        assert_eq!(board.to_fen(), STARTING_FEN);
    }

    #[test]
    fn extra_whitespace_is_ignored() {
        let fen = "  rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR   w KQkq -  0 1\n";
        assert_eq!(parse(fen), parse(STARTING_FEN));
    }

    #[test]
    fn accepts_fullmove_zero() {
        // Semantic check, deliberately left for a later `validate`.
        assert_eq!(parse("8/8/8/8/8/8/8/8 w - - 0 0").fullmove_count(), 0);
    }

    // Rejections

    #[test]
    fn rejects_wrong_field_count() {
        for (fen, count) in [
            ("", 0),
            ("8/8/8/8/8/8/8/8 w -", 3),
            ("8/8/8/8/8/8/8/8 w - - 0", 5),
            ("8/8/8/8/8/8/8/8 w - - 0 1 extra", 7),
        ] {
            assert_eq!(parse_err(fen), FenError::WrongFieldCount(count), "{fen:?}");
        }
    }

    #[test]
    fn rejects_wrong_rank_count() {
        assert_eq!(placement_err("8/8/8/8/8/8/8"), FenError::WrongRankCount(7));
        assert_eq!(
            placement_err("8/8/8/8/8/8/8/8/8"),
            FenError::WrongRankCount(9)
        );
    }

    #[test]
    fn rejects_wrong_rank_length() {
        for (placement, rank, text) in [
            ("7/8/8/8/8/8/8/8", 8, "7"),             // too short, digit
            ("8/ppppppp/8/8/8/8/8/8", 7, "ppppppp"), // too short, pieces
            ("8/8/4p4/8/8/8/8/8", 6, "4p4"),         // 9 squares
            ("8/8/8/8/8/8/8/8p", 1, "8p"),           // piece after a full rank
            ("8/8/8/8/8/8/8/p8", 1, "p8"),           // digit overflows the rank
            ("8/8/8/8/8/8/8/", 1, ""),               // trailing slash, empty rank
        ] {
            assert_eq!(
                placement_err(placement),
                FenError::WrongRankLength {
                    rank,
                    text: text.to_string()
                },
                "{placement}"
            );
        }
    }

    #[test]
    fn rejects_invalid_rank_chars() {
        for (placement, rank, ch) in [
            ("rnbqkbnr/ppppxppp/8/8/8/8/PPPPPPPP/RNBQKBNR", 7, 'x'),
            ("9/8/8/8/8/8/8/8", 8, '9'),
            ("8/8/8/8/8/8/8/08", 1, '0'),
            ("8/8/8/8/44/8/8/8", 4, '4'), // consecutive digits
        ] {
            assert_eq!(
                placement_err(placement),
                FenError::InvalidRankChar { rank, ch },
                "{placement}"
            );
        }
    }

    #[test]
    fn rejects_invalid_side_to_move() {
        for side in ["W", "x", "white", "-"] {
            assert_eq!(
                parse_err(&format!("8/8/8/8/8/8/8/8 {side} - - 0 1")),
                FenError::InvalidSideToMove(side.to_string())
            );
        }
    }

    #[test]
    fn rejects_invalid_castling() {
        for castling in ["KQkqK", "KK", "X"] {
            assert_eq!(
                parse_err(&format!("8/8/8/8/8/8/8/8 w {castling} - 0 1")),
                FenError::InvalidCastling(castling.to_string())
            );
        }
    }

    #[test]
    fn rejects_invalid_en_passant() {
        for ep in ["e9", "z3", "e"] {
            assert_eq!(
                parse_err(&format!("8/8/8/8/8/8/8/8 w - {ep} 0 1")),
                FenError::InvalidEnPassant(ep.to_string())
            );
        }
    }

    #[test]
    fn rejects_invalid_clocks() {
        for clock in ["-1", "abc", "70000"] {
            let err = parse_err(&format!("8/8/8/8/8/8/8/8 w - - {clock} 1"));
            assert!(
                matches!(&err, FenError::InvalidHalfmoveClock { text, .. } if text == clock),
                "{err:?}"
            );
            assert!(std::error::Error::source(&err).is_some());
        }

        let err = parse_err("8/8/8/8/8/8/8/8 w - - 0 x");
        assert!(
            matches!(&err, FenError::InvalidFullmoveCount { text, .. } if text == "x"),
            "{err:?}"
        );
    }
}
