use crate::{
    board::Board,
    movegen::MoveList,
    types::{Bitboard, CastleSide, Move, MoveKind, PieceType, Square},
};
use std::{fmt, str::FromStr};
use thiserror::Error;

#[derive(Error, Clone, PartialEq, Eq, Debug)]
pub enum SanParseError {
    #[error("empty move")]
    Empty,

    #[error("non-ascii move {0:?}")]
    NonAscii(String),

    #[error("invalid target square in {0:?}")]
    InvalidSquare(String),

    #[error("invalid promotion in {0:?}")]
    InvalidPromotion(String),

    #[error("unexpected text in {0:?}")]
    Malformed(String),
}

#[derive(Error, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SanError {
    #[error("no legal move matches")]
    Illegal,

    #[error("more than one legal move matches")]
    Ambiguous,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SanMove {
    Castle(CastleSide),
    Normal {
        piece: PieceType,
        from_file: Option<u8>,
        from_rank: Option<u8>,
        is_capture: bool,
        to: Square,
        promotion: Option<PieceType>,
    },
}

impl FromStr for SanMove {
    type Err = SanParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(SanParseError::Empty);
        }
        if !s.is_ascii() {
            return Err(SanParseError::NonAscii(s.to_string()));
        }
        let rest = s.trim_matches(['+', '#', '?', '!']);
        match rest {
            "O-O" | "0-0" => return Ok(Self::Castle(CastleSide::KingSide)),
            "O-O-O" | "0-0-0" => return Ok(Self::Castle(CastleSide::QueenSide)),
            _ => (),
        }

        let (rest, promotion) =
            strip_promotion(rest).ok_or_else(|| SanParseError::InvalidPromotion(s.to_string()))?;
        let (rest, target) =
            split_target(rest).ok_or_else(|| SanParseError::Malformed(s.to_string()))?;
        let to: Square = target
            .parse()
            .map_err(|_| SanParseError::InvalidSquare(s.to_string()))?;
        let (rest, piece) = split_piece(rest);
        let (from_file, from_rank, is_capture) =
            parse_disambiguation(rest).ok_or_else(|| SanParseError::Malformed(s.to_string()))?;
        Ok(Self::Normal {
            piece,
            from_file,
            from_rank,
            is_capture,
            to,
            promotion,
        })
    }
}

fn piece_from_letter(c: char) -> Option<PieceType> {
    if !c.is_ascii_uppercase() {
        return None;
    }
    if let Some(pt) = PieceType::from_letter(c.to_ascii_lowercase())
        && pt != PieceType::Pawn
    {
        Some(pt)
    } else {
        None
    }
}

fn promotion_from_char(c: char) -> Option<PieceType> {
    if let Some(pt) = PieceType::from_letter(c.to_ascii_lowercase())
        && PieceType::PROMOTIONS.contains(&pt)
    {
        Some(pt)
    } else {
        None
    }
}

fn strip_promotion(s: &str) -> Option<(&str, Option<PieceType>)> {
    let mut chars = s.chars();
    let Some(last) = chars.next_back() else {
        return Some((s, None));
    };
    let head = chars.as_str();

    if last == '=' {
        return None;
    }
    if let Some(head) = head.strip_suffix('=') {
        return Some((head, Some(promotion_from_char(last)?)));
    }
    if head.ends_with(|c: char| c.is_ascii_digit()) && last.is_ascii_uppercase() {
        return Some((head, Some(promotion_from_char(last)?)));
    }
    Some((s, None))
}

fn split_target(s: &str) -> Option<(&str, &str)> {
    if let len = s.len()
        && len >= 2
    {
        s.split_at_checked(len - 2)
    } else {
        None
    }
}

fn split_piece(s: &str) -> (&str, PieceType) {
    let mut chars = s.chars();
    if let Some(c) = chars.next()
        && let Some(piece) = piece_from_letter(c)
    {
        (chars.as_str(), piece)
    } else {
        (s, PieceType::Pawn)
    }
}

fn parse_disambiguation(s: &str) -> Option<(Option<u8>, Option<u8>, bool)> {
    fn take(rest: &mut &str, parse: fn(char) -> Option<u8>) -> Option<u8> {
        let mut chars = rest.chars();
        if let Some(index) = parse(chars.next()?) {
            *rest = chars.as_str();
            Some(index)
        } else {
            None
        }
    }

    let mut rest = s;
    let from_file = take(&mut rest, Square::file_from_char);
    let from_rank = take(&mut rest, Square::rank_from_char);
    let is_capture: bool;
    (rest, is_capture) = if let Some(rest) = rest.strip_prefix("x") {
        (rest, true)
    } else {
        (rest, false)
    };
    rest.is_empty()
        .then_some((from_file, from_rank, is_capture))
}

impl fmt::Display for SanMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Castle(CastleSide::KingSide) => write!(f, "O-O"),
            Self::Castle(CastleSide::QueenSide) => write!(f, "O-O-O"),
            Self::Normal {
                piece,
                from_file,
                from_rank,
                is_capture,
                to,
                promotion,
            } => {
                if piece != PieceType::Pawn {
                    write!(f, "{}", piece_letter(piece))?;
                }
                if let Some(file) = from_file {
                    write!(f, "{}", Square::file_char(file))?;
                }
                if let Some(rank) = from_rank {
                    write!(f, "{}", Square::rank_char(rank))?;
                }
                if is_capture {
                    f.write_str("x")?;
                }
                write!(f, "{to}")?;
                if let Some(piece) = promotion {
                    write!(f, "={}", piece_letter(piece))?;
                }
                Ok(())
            }
        }
    }
}

fn piece_letter(pt: PieceType) -> char {
    pt.letter().to_ascii_uppercase()
}

pub struct San<'a> {
    board: &'a Board,
    mv: Move,
}

fn san_move(board: &Board, mv: Move) -> SanMove {
    if let MoveKind::Castle(side) = mv.kind() {
        return SanMove::Castle(side);
    }
    let piece = board
        .piece_at(mv.from())
        .expect("move is legal so piece exists")
        .piece_type;
    let is_capture = board.piece_at(mv.to()).is_some() || mv.kind() == MoveKind::EnPassant;
    let (from_file, from_rank) = if is_capture && piece == PieceType::Pawn {
        (Some(mv.from().file()), None)
    } else {
        disambiguate(board, mv, piece)
    };
    let promotion = if let MoveKind::Promotion(p) = mv.kind() {
        Some(p)
    } else {
        None
    };
    SanMove::Normal {
        piece,
        from_file,
        from_rank,
        is_capture,
        to: mv.to(),
        promotion,
    }
}

fn disambiguate(board: &Board, mv: Move, piece: PieceType) -> (Option<u8>, Option<u8>) {
    let mut moves = MoveList::new();
    board.legal_moves(&mut moves);

    let file = mv.from().file();
    let rank = mv.from().rank();
    let rivals: Bitboard = moves
        .iter()
        .filter(|other| {
            other.to() == mv.to()
                && other.from() != mv.from()
                && board
                    .piece_at(other.from())
                    .is_some_and(|p| p.piece_type == piece)
        })
        .map(|other| other.from())
        .collect();

    let any = !rivals.is_empty();
    let same_file = rivals.overlaps(Bitboard::FILES[file as usize]);
    let same_rank = rivals.overlaps(Bitboard::RANKS[rank as usize]);
    match (any, same_file, same_rank) {
        (false, _, _) => (None, None),
        (true, false, _) => (Some(file), None),
        (true, true, false) => (None, Some(rank)),
        (true, true, true) => (Some(file), Some(rank)),
    }
}

impl fmt::Display for San<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", san_move(self.board, self.mv))?;
        let mut next = *self.board;
        next.make_move(self.mv);
        if next.is_in_check(next.side_to_move()) {
            let mut moves = MoveList::new();
            next.legal_moves(&mut moves);
            if moves.is_empty() {
                f.write_str("#")?;
            } else {
                f.write_str("+")?;
            }
        }
        Ok(())
    }
}

impl Board {
    pub fn san(&self, mv: Move) -> San<'_> {
        let mut moves = MoveList::new();
        self.legal_moves(&mut moves);
        debug_assert!(
            moves.contains(&mv),
            "{mv} is not legal in current position's legal moves: {moves}"
        );
        San { board: self, mv }
    }

    pub fn resolve_san(&self, san: &SanMove) -> Result<Move, SanError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_helpers::{KIWIPETE, STARTING_FEN, from_fen, sq},
        types::PieceType::{Bishop, King, Knight, Pawn, Queen, Rook},
    };

    const KIWIPETE_BLACK: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R b KQkq -";

    type SanCases = &'static [(&'static str, &'static str, &'static str, &'static str)];

    /// The legal move in `board` whose UCI text is `uci`.
    #[track_caller]
    fn legal(board: &Board, uci: &str) -> Move {
        let mut moves = MoveList::new();
        board.legal_moves(&mut moves);
        moves
            .iter()
            .copied()
            .find(|mv| mv.to_string() == uci)
            .unwrap_or_else(|| panic!("{uci} is not legal in {}", board.fen()))
    }

    /// Checks `board.san(mv)` for every row.
    #[track_caller]
    fn assert_san_rows(cases: SanCases) {
        for &(why, fen, uci, expected) in cases {
            let board = from_fen(fen);
            board
                .validate()
                .unwrap_or_else(|e| panic!("{why}: invalid row position {fen}: {e}"));
            let mv = legal(&board, uci);
            assert_eq!(expected, board.san(mv).to_string(), "{why}: {uci} in {fen}");
        }
    }

    #[track_caller]
    fn normal(
        piece: PieceType,
        from_file: Option<u8>,
        from_rank: Option<u8>,
        is_capture: bool,
        to: &str,
        promotion: Option<PieceType>,
    ) -> SanMove {
        SanMove::Normal {
            piece,
            from_file,
            from_rank,
            is_capture,
            to: sq(to),
            promotion,
        }
    }

    #[test]
    fn sanmove_display() {
        let cases = [
            (normal(Pawn, None, None, false, "e4", None), "e4"),
            (normal(Pawn, Some(4), None, true, "d5", None), "exd5"),
            (normal(Knight, None, None, false, "f3", None), "Nf3"),
            (normal(Knight, Some(1), None, false, "d7", None), "Nbd7"),
            (normal(Rook, None, Some(0), false, "a3", None), "R1a3"),
            (normal(Queen, Some(7), Some(3), false, "e1", None), "Qh4e1"),
            (normal(King, None, None, true, "e2", None), "Kxe2"),
            (normal(Pawn, None, None, false, "e8", Some(Queen)), "e8=Q"),
            (
                normal(Pawn, Some(0), None, true, "b8", Some(Knight)),
                "axb8=N",
            ),
            (SanMove::Castle(CastleSide::KingSide), "O-O"),
            (SanMove::Castle(CastleSide::QueenSide), "O-O-O"),
        ];
        for (mv, expected) in cases {
            assert_eq!(&mv.to_string(), expected);
        }
    }

    #[test]
    fn san_without_hints_or_check() {
        let cases: SanCases = &[
            ("pawn push", STARTING_FEN, "e2e4", "e4"),
            ("double push is just a push", STARTING_FEN, "d2d4", "d4"),
            ("knight", STARTING_FEN, "g1f3", "Nf3"),
            (
                "black knight",
                "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
                "g8f6",
                "Nf6",
            ),
            (
                "white pawn capture",
                "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 2",
                "e4d5",
                "exd5",
            ),
            (
                "black pawn capture",
                "rnbqkbnr/pppp1ppp/8/4p3/3P4/8/PPP1PPPP/RNBQKBNR b KQkq d3 0 2",
                "e5d4",
                "exd4",
            ),
            (
                "white en passant",
                "rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3",
                "e5f6",
                "exf6",
            ),
            (
                "black en passant",
                "rnbqkbnr/pppp1ppp/8/8/3Pp3/5N2/PPP1PPPP/RNBQKB1R b KQkq d3 0 3",
                "e4d3",
                "exd3",
            ),
            ("piece capture", KIWIPETE, "e5f7", "Nxf7"),
            ("bishop capture", KIWIPETE, "e2a6", "Bxa6"),
            ("pawn capture in Kiwipete", KIWIPETE, "d5e6", "dxe6"),
            ("white king side castle", KIWIPETE, "e1g1", "O-O"),
            ("white queen side castle", KIWIPETE, "e1c1", "O-O-O"),
            ("black king side castle", KIWIPETE_BLACK, "e8g8", "O-O"),
            ("black queen side castle", KIWIPETE_BLACK, "e8c8", "O-O-O"),
            ("king step, not a castle", KIWIPETE, "e1f1", "Kf1"),
            ("promotion", "8/P6k/8/8/8/8/8/K7 w - - 0 1", "a7a8q", "a8=Q"),
            (
                "capture-promotion",
                "1n5k/P7/8/8/8/8/8/K7 w - - 0 1",
                "a7b8n",
                "axb8=N",
            ),
            (
                "black underpromotion",
                "k7/8/8/8/8/8/p6K/8 b - - 0 1",
                "a2a1r",
                "a1=R",
            ),
        ];
        assert_san_rows(cases);
    }

    #[test]
    fn san_disambiguation() {
        let cases: SanCases = &[
            (
                "two knights, files differ",
                "4k3/8/8/8/8/2N5/8/4K1N1 w - - 0 1",
                "g1e2",
                "Nge2",
            ),
            (
                "two knights, the other one",
                "4k3/8/8/8/8/2N5/8/4K1N1 w - - 0 1",
                "c3e2",
                "Nce2",
            ),
            (
                "same file, ranks differ",
                "4k3/8/8/1N6/8/1N6/8/4K3 w - - 0 1",
                "b3d4",
                "N3d4",
            ),
            (
                "same file, the other one",
                "4k3/8/8/1N6/8/1N6/8/4K3 w - - 0 1",
                "b5d4",
                "N5d4",
            ),
            (
                "black rooks, files differ",
                "r6r/4k3/8/8/8/8/8/4K3 b - - 0 1",
                "a8d8",
                "Rad8",
            ),
            (
                "three queens, needs the full square",
                "1k6/8/8/8/4Q2Q/8/8/K6Q w - - 0 1",
                "h4e1",
                "Qh4e1",
            ),
            (
                "three queens, file is enough",
                "1k6/8/8/8/4Q2Q/8/8/K6Q w - - 0 1",
                "e4e1",
                "Qee1",
            ),
            (
                "three queens, rank is enough",
                "1k6/8/8/8/4Q2Q/8/8/K6Q w - - 0 1",
                "h1e1",
                "Q1e1",
            ),
            (
                "capture with no rival, no hint",
                "4k3/8/8/3p4/8/2N5/8/4K1N1 w - - 0 1",
                "c3d5",
                "Nxd5",
            ),
            (
                "two knights capture",
                "4k3/8/8/8/8/2N5/4p3/4K1N1 w - - 0 1",
                "g1e2",
                "Ngxe2",
            ),
            (
                "pinned knight forces no hint",
                "4k3/8/8/8/1b6/2N5/8/4K1N1 w - - 0 1",
                "g1e2",
                "Ne2",
            ),
            (
                "rook and knight both reach h3, no hint",
                "4k3/8/8/8/8/8/8/4K1NR w - - 0 1",
                "g1h3",
                "Nh3",
            ),
            (
                "two pawns could capture, still file only",
                "4k3/8/8/3p4/2P1P3/8/8/4K3 w - - 0 1",
                "e4d5",
                "exd5",
            ),
        ];
        assert_san_rows(cases);
    }

    #[test]
    fn san_checks() {
        let cases: SanCases = &[
            (
                "check",
                "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 2",
                "f1b5",
                "Bb5+",
            ),
            (
                "black mate",
                "rnbqkbnr/pppp1ppp/8/4p3/6P1/5P2/PPPPP2P/RNBQKBNR b KQkq g3 0 2",
                "d8h4",
                "Qh4#",
            ),
            (
                "white mate, capture",
                "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4",
                "h5f7",
                "Qxf7#",
            ),
            (
                "castling with check",
                "5k2/8/8/8/8/8/8/4K2R w K - 0 1",
                "e1g1",
                "O-O+",
            ),
            (
                "promotion with check",
                "k7/4P3/8/8/8/8/8/K7 w - - 0 1",
                "e7e8q",
                "e8=Q+",
            ),
            (
                "rook promotion with check",
                "k7/4P3/8/8/8/8/8/K7 w - - 0 1",
                "e7e8r",
                "e8=R+",
            ),
            (
                "knight promotion with no check",
                "k7/4P3/8/8/8/8/8/K7 w - - 0 1",
                "e7e8n",
                "e8=N",
            ),
        ];
        assert_san_rows(cases);
    }

    #[test]
    fn piece_from_letter_rows() {
        type Cases = &'static [(char, Option<PieceType>)];
        let cases: Cases = &[
            ('N', Some(Knight)),
            ('B', Some(Bishop)),
            ('R', Some(Rook)),
            ('Q', Some(Queen)),
            ('K', Some(King)),
            ('P', None),
            ('n', None),
            ('b', None),
            ('x', None),
            ('1', None),
        ];
        for &(c, expected) in cases {
            assert_eq!(expected, piece_from_letter(c), "piece_from_letter({c:?})");
        }
    }

    #[test]
    fn promotion_from_char_rows() {
        type Cases = &'static [(char, Option<PieceType>)];
        let cases: Cases = &[
            ('Q', Some(Queen)),
            ('R', Some(Rook)),
            ('B', Some(Bishop)),
            ('N', Some(Knight)),
            ('q', Some(Queen)),
            ('n', Some(Knight)),
            ('K', None),
            ('P', None),
            ('k', None),
            ('x', None),
            ('=', None),
        ];
        for &(c, expected) in cases {
            assert_eq!(
                expected,
                promotion_from_char(c),
                "promotion_from_char({c:?})"
            );
        }
    }

    #[test]
    fn strip_promotion_rows() {
        type Cases = &'static [(&'static str, Option<(&'static str, Option<PieceType>)>)];
        let cases: Cases = &[
            ("e8=Q", Some(("e8", Some(Queen)))),
            ("e8=R", Some(("e8", Some(Rook)))),
            ("e8=B", Some(("e8", Some(Bishop)))),
            ("exd8=N", Some(("exd8", Some(Knight)))),
            ("e8Q", Some(("e8", Some(Queen)))),
            ("e8=q", Some(("e8", Some(Queen)))),
            ("a1=R", Some(("a1", Some(Rook)))),
            ("Nf3", Some(("Nf3", None))),
            ("e4", Some(("e4", None))),
            ("exd5", Some(("exd5", None))),
            ("Nbd7", Some(("Nbd7", None))),
            ("e88", Some(("e88", None))),
            ("", Some(("", None))),
            ("e8=K", None),
            ("e8=P", None),
            ("e8K", None),
            ("e8=", None),
            ("e8=x", None),
        ];
        for &(s, expected) in cases {
            assert_eq!(expected, strip_promotion(s), "strip_promotion({s:?})");
        }
    }

    #[test]
    fn split_target_rows() {
        type Cases = &'static [(&'static str, Option<(&'static str, &'static str)>)];
        let cases: Cases = &[
            ("Nf3", Some(("N", "f3"))),
            ("e4", Some(("", "e4"))),
            ("Qh4e1", Some(("Qh4", "e1"))),
            ("exd5", Some(("ex", "d5"))),
            ("N", None),
            ("", None),
        ];
        for &(s, expected) in cases {
            assert_eq!(expected, split_target(s), "split_target({s:?})");
        }
    }

    #[test]
    fn split_piece_rows() {
        type Cases = &'static [(&'static str, (&'static str, PieceType))];
        let cases: Cases = &[
            ("N", ("", Knight)),
            ("B", ("", Bishop)),
            ("R", ("", Rook)),
            ("Qh4", ("h4", Queen)),
            ("Kx", ("x", King)),
            ("", ("", Pawn)),
            ("e", ("e", Pawn)),
            ("ex", ("ex", Pawn)),
            ("b", ("b", Pawn)),
            ("P", ("P", Pawn)),
        ];
        for &(s, expected) in cases {
            assert_eq!(expected, split_piece(s), "split_piece({s:?})");
        }
    }

    #[test]
    fn parse_disambiguation_rows() {
        type Cases = &'static [(&'static str, Option<(Option<u8>, Option<u8>, bool)>)];
        let cases: Cases = &[
            ("", Some((None, None, false))),
            ("x", Some((None, None, true))),
            ("e", Some((Some(4), None, false))),
            ("ex", Some((Some(4), None, true))),
            ("1", Some((None, Some(0), false))),
            ("h4", Some((Some(7), Some(3), false))),
            ("bx", Some((Some(1), None, true))),
            ("g1x", Some((Some(6), Some(0), true))),
            ("zz", None),
            ("xx", None),
            ("4h", None),
            ("x1", None),
            ("e4e", None),
        ];
        for &(s, expected) in cases {
            assert_eq!(
                expected,
                parse_disambiguation(s),
                "parse_disambiguation({s:?})"
            );
        }
    }

    #[test]
    fn san_move_parses_canonical_text() {
        let cases: &[(&str, SanMove)] = &[
            ("e4", normal(Pawn, None, None, false, "e4", None)),
            ("exd5", normal(Pawn, Some(4), None, true, "d5", None)),
            ("Nf3", normal(Knight, None, None, false, "f3", None)),
            ("Nbd7", normal(Knight, Some(1), None, false, "d7", None)),
            ("R1a3", normal(Rook, None, Some(0), false, "a3", None)),
            ("Qh4e1", normal(Queen, Some(7), Some(3), false, "e1", None)),
            ("Ngxe2", normal(Knight, Some(6), None, true, "e2", None)),
            ("Kxe2", normal(King, None, None, true, "e2", None)),
            ("e8=Q", normal(Pawn, None, None, false, "e8", Some(Queen))),
            (
                "axb8=N",
                normal(Pawn, Some(0), None, true, "b8", Some(Knight)),
            ),
            ("O-O", SanMove::Castle(CastleSide::KingSide)),
            ("O-O-O", SanMove::Castle(CastleSide::QueenSide)),
        ];
        for &(text, expected) in cases {
            assert_eq!(Ok(expected), text.parse::<SanMove>(), "parse {text:?}");
            assert_eq!(text, expected.to_string(), "Display of {text:?}'s value");
        }
    }

    #[test]
    fn san_move_parse_is_lenient() {
        let cases: &[(&str, &str)] = &[
            ("0-0", "O-O"),
            ("0-0-0", "O-O-O"),
            ("O-O+", "O-O"),
            ("e8Q", "e8=Q"),
            ("e8=q", "e8=Q"),
            ("Nf3+", "Nf3"),
            ("Qh4#", "Qh4"),
            ("Nxf7#", "Nxf7"),
            ("e4!?", "e4"),
            ("e8=Q+", "e8=Q"),
        ];
        for &(text, canonical) in cases {
            let parsed = text
                .parse::<SanMove>()
                .unwrap_or_else(|e| panic!("{text:?} should parse: {e}"));
            assert_eq!(canonical, parsed.to_string(), "canonical form of {text:?}");
        }
    }

    #[test]
    fn san_move_parse_errors() {
        use SanParseError::*;
        let cases: &[(&str, SanParseError)] = &[
            ("", Empty),
            ("N\u{e9}4", NonAscii("N\u{e9}4".to_string())),
            ("Nz9", InvalidSquare("Nz9".to_string())),
            ("e88", InvalidSquare("e88".to_string())),
            ("e8=K", InvalidPromotion("e8=K".to_string())),
            ("e8=", InvalidPromotion("e8=".to_string())),
            ("e8K", InvalidPromotion("e8K".to_string())),
            ("N", Malformed("N".to_string())),
            ("Nbbd7", Malformed("Nbbd7".to_string())),
            ("Nzzd5", Malformed("Nzzd5".to_string())),
            ("Pe4", Malformed("Pe4".to_string())),
            ("xxe4", Malformed("xxe4".to_string())),
        ];
        for (text, expected) in cases {
            assert_eq!(
                Err(expected.clone()),
                text.parse::<SanMove>(),
                "parse {text:?}"
            );
        }
    }
}
