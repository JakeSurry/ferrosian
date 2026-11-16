//! Core chess types: colors, pieces, squares, castling rights and moves.

use derive_more::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, LowerHex, Not};
use std::{fmt, str::FromStr};
use thiserror::Error;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Error)]
pub enum ParseError {
    #[error("invalid square name")]
    InvalidSquare,

    #[error("invalid castling rights")]
    InvalidCastlingRights,

    #[error("invalid color char")]
    InvalidColor,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Default)]
#[repr(u8)]
pub enum Color {
    #[default]
    White = 0,
    Black = 1,
}

impl Color {
    /// Both colors, in index order.
    pub const ALL: [Color; 2] = [Color::White, Color::Black];

    /// Number of colors, for sizing arrays.
    pub const COUNT: usize = Self::ALL.len();

    /// Self from fen char
    pub fn from_fen_char(c: char) -> Option<Self> {
        Self::ALL.into_iter().find(|color| color.fen_char() == c)
    }

    /// Fen character of self.
    pub const fn fen_char(self) -> char {
        match self {
            Color::White => 'w',
            Color::Black => 'b',
        }
    }

    /// Index for array lookups. White = 0, Black = 1.
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Gets opposite color of self.
    pub const fn opposite(self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Color::White => write!(f, "White"),
            Color::Black => write!(f, "Black"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
pub enum PieceType {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
}

impl PieceType {
    /// Every piece type, in index order.
    pub const ALL: [PieceType; 6] = [
        Self::Pawn,
        Self::Knight,
        Self::Bishop,
        Self::Rook,
        Self::Queen,
        Self::King,
    ];

    /// Number of piece types, for sizing arrays.
    pub const COUNT: usize = Self::ALL.len();

    /// Candidate promotion pieces for move generators, queen comes first as it's often best.
    pub const PROMOTIONS: [Self; 4] = [Self::Queen, Self::Rook, Self::Bishop, Self::Knight];

    /// Index for array lookups, 0 through 5.
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The piece type at `index`, or `None` if it is out of range.
    pub fn from_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }

    /// Lowercase letter as used in FEN and UCI.
    pub const fn letter(self) -> char {
        match self {
            Self::Pawn => 'p',
            Self::Knight => 'n',
            Self::Bishop => 'b',
            Self::Rook => 'r',
            Self::Queen => 'q',
            Self::King => 'k',
        }
    }

    /// Parses a lowercase letter. Case and Color are handled by `Piece`.
    pub fn from_letter(c: char) -> Option<Self> {
        Self::ALL.into_iter().find(|pt| pt.letter() == c)
    }
}

impl fmt::Display for PieceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.letter())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Piece {
    pub color: Color,
    pub piece_type: PieceType,
}

impl Piece {
    pub const fn new(color: Color, piece_type: PieceType) -> Self {
        Self { color, piece_type }
    }

    /// Parses a FEN piece character. Uppercase for white, lowercase for black.
    pub fn from_fen_char(c: char) -> Option<Self> {
        let piece_type = PieceType::from_letter(c.to_ascii_lowercase())?;
        let color = if c.is_uppercase() {
            Color::White
        } else {
            Color::Black
        };

        Some(Self { color, piece_type })
    }

    /// Gets the FEN character for this piece.
    pub const fn fen_char(self) -> char {
        let c = self.piece_type.letter();
        match self.color {
            Color::White => c.to_ascii_uppercase(),
            Color::Black => c,
        }
    }

    /// Gets the Unicode character for this piece. Filled for white, outline for black.
    pub const fn unicode_char(self) -> char {
        match (self.color, self.piece_type) {
            (Color::White, PieceType::King) => '\u{265A}',   // ♚
            (Color::White, PieceType::Queen) => '\u{265B}',  // ♛
            (Color::White, PieceType::Rook) => '\u{265C}',   // ♜
            (Color::White, PieceType::Bishop) => '\u{265D}', // ♝
            (Color::White, PieceType::Knight) => '\u{265E}', // ♞
            (Color::White, PieceType::Pawn) => '\u{265F}',   // ♟
            (Color::Black, PieceType::King) => '\u{2654}',   // ♔
            (Color::Black, PieceType::Queen) => '\u{2655}',  // ♕
            (Color::Black, PieceType::Rook) => '\u{2656}',   // ♖
            (Color::Black, PieceType::Bishop) => '\u{2657}', // ♗
            (Color::Black, PieceType::Knight) => '\u{2658}', // ♘
            (Color::Black, PieceType::Pawn) => '\u{2659}',   // ♙
        }
    }

    /// Legible name for a piece, used in error messages.
    pub fn name(self) -> String {
        format!("{:?} {:?}", self.color, self.piece_type)
    }
}

impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.fen_char())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Square(u8);

impl Square {
    /// Number of squares, for sizing arrays.
    pub const COUNT: usize = 64;

    /// The A1 square.
    pub const A1: Self = Self(0);

    /// The square at `index`, or `None` if it is not in range 0..64.
    pub const fn from_index(index: usize) -> Option<Self> {
        if index < Self::COUNT {
            Some(Self(index as u8))
        } else {
            None
        }
    }

    /// The square at a 0-based file and rank (a = 0, rank 1 = 0).
    pub const fn from_file_rank(file: u8, rank: u8) -> Option<Self> {
        if file < 8 && rank < 8 {
            Some(Self(rank * 8 + file))
        } else {
            None
        }
    }

    /// Index for array lookups, 0..63.
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// 0-based file.
    pub const fn file(self) -> u8 {
        self.0 % 8
    }

    /// 0-based rank.
    pub const fn rank(self) -> u8 {
        self.0 / 8
    }

    /// A bitboard with only this square's bit set.
    pub const fn bitboard(self) -> Bitboard {
        Bitboard(1u64 << self.0)
    }

    /// Flip rank to r - 7
    pub const fn flip_rank(self) -> Self {
        Self(self.0 ^ 56)
    }

    /// Gets file char from file index.
    pub const fn file_char(file: u8) -> char {
        debug_assert!(file < 8, "expected file < 8");
        (b'a' + file) as char
    }

    /// Gets rank char from rank index.
    pub const fn rank_char(rank: u8) -> char {
        debug_assert!(rank < 8, "expected rank < 8");
        (b'1' + rank) as char
    }

    /// Gets file index from file char.
    pub const fn file_from_char(c: char) -> Option<u8> {
        match c {
            f @ 'a'..='h' => Some(f as u8 - b'a'),
            _ => None,
        }
    }

    /// Gets rank index from rank char.
    pub const fn rank_from_char(c: char) -> Option<u8> {
        match c {
            r @ '1'..='8' => Some(r as u8 - b'1'),
            _ => None,
        }
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file = Self::file_char(self.file());
        let rank = Self::rank_char(self.rank());
        write!(f, "{file}{rank}")
    }
}

impl FromStr for Square {
    type Err = ParseError;

    /// Parses a square name, e.g. `e4`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();
        match (chars.next(), chars.next(), chars.next()) {
            (Some(f), Some(r), None) => Self::file_from_char(f)
                .zip(Self::rank_from_char(r))
                .and_then(|(file, rank)| Self::from_file_rank(file, rank)),
            _ => None,
        }
        .ok_or(ParseError::InvalidSquare)
    }
}

/// The side of the board a castling move goes to.
///
/// The squares involved are given by [`CastleSide::path`], defined in
/// [`crate::castling`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum CastleSide {
    KingSide = 0,
    QueenSide = 1,
}

impl CastleSide {
    pub const ALL: [Self; 2] = [Self::KingSide, Self::QueenSide];
    pub const COUNT: usize = Self::ALL.len();

    pub fn index(self) -> usize {
        self as usize
    }
}

impl fmt::Display for CastleSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KingSide => write!(f, "kingside"),
            Self::QueenSide => write!(f, "queenside"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Default)]
pub struct CastlingRights(u8);

impl CastlingRights {
    pub const NONE: Self = Self(0);
    pub const ALL: Self = Self(0b1111);

    const FEN_ORDER: [(char, Color, CastleSide); 4] = [
        ('K', Color::White, CastleSide::KingSide),
        ('Q', Color::White, CastleSide::QueenSide),
        ('k', Color::Black, CastleSide::KingSide),
        ('q', Color::Black, CastleSide::QueenSide),
    ];

    /// Matches bit for `color` and `side`.
    const fn bit(color: Color, side: CastleSide) -> u8 {
        match (color, side) {
            (Color::White, CastleSide::KingSide) => 0b0001,
            (Color::White, CastleSide::QueenSide) => 0b0010,
            (Color::Black, CastleSide::KingSide) => 0b0100,
            (Color::Black, CastleSide::QueenSide) => 0b1000,
        }
    }

    /// Whether `color` still has the right to castle on `side`.
    pub const fn has(self, color: Color, side: CastleSide) -> bool {
        self.0 & Self::bit(color, side) != 0
    }

    /// Removes one right.
    pub fn remove_one(&mut self, color: Color, side: CastleSide) {
        self.0 &= !Self::bit(color, side);
    }

    /// Removes both rights for `color`.
    pub fn remove_both(&mut self, color: Color) {
        self.remove_one(color, CastleSide::KingSide);
        self.remove_one(color, CastleSide::QueenSide);
    }
}

impl fmt::Display for CastlingRights {
    /// Writes the FEN castling field, e.g. `KQkq`, `Kq`, `-`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::NONE {
            return write!(f, "-");
        }
        for (c, color, side) in Self::FEN_ORDER {
            if self.has(color, side) {
                write!(f, "{c}")?;
            }
        }
        Ok(())
    }
}

impl FromStr for CastlingRights {
    type Err = ParseError;

    /// Parses the FEN castling field. Rejects invalid FEN.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "-" {
            return Ok(Self::NONE);
        }
        if s.is_empty() {
            return Err(ParseError::InvalidCastlingRights);
        }

        let mut rights = Self::NONE;
        for ch in s.chars() {
            let (_, color, side) = Self::FEN_ORDER
                .into_iter()
                .find(|&(c, _, _)| c == ch)
                .ok_or(ParseError::InvalidCastlingRights)?;
            if rights.has(color, side) {
                return Err(ParseError::InvalidCastlingRights);
            }
            rights.0 |= Self::bit(color, side);
        }
        Ok(rights)
    }
}

/// Describe the function of a move.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum MoveKind {
    /// normal moves and ordinary captures.
    Normal,

    /// double pawn push, adds en passant square.
    DoublePush,

    /// en passant, removes captured pawn.
    EnPassant,

    /// castle to a given `CastleSide`, rook move comes from [`CastleSide::path`].
    Castle(CastleSide),

    /// promotion to a given `PieceType`
    Promotion(PieceType),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Move {
    from: Square,
    to: Square,
    kind: MoveKind,
}

impl Move {
    pub const fn new(from: Square, to: Square, kind: MoveKind) -> Self {
        debug_assert!(
            !matches!(kind, MoveKind::Promotion(PieceType::Pawn | PieceType::King)),
            "promotion piece must be a knight, bishop, rook or queen"
        );
        Move { from, to, kind }
    }

    /// `square` where move originated.
    pub fn from(self) -> Square {
        self.from
    }

    /// `square` where move ended up.
    pub fn to(self) -> Square {
        self.to
    }

    /// `MoveKind` of move.
    pub fn kind(self) -> MoveKind {
        self.kind
    }
}

impl fmt::Display for Move {
    /// Writes the move in UCI notation, e.g. `e2e4`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.from, self.to)?;
        if let MoveKind::Promotion(piece) = self.kind {
            write!(f, "{piece}")?;
        }
        Ok(())
    }
}

/// A u64 newtype where each bit represents a square.
///
/// a1 = 0, h8 = 2.pow(63)
/// impl in [`crate::bitboard`]
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    Hash,
    Debug,
    BitAnd,
    BitOr,
    BitXor,
    Not,
    BitAndAssign,
    BitOrAssign,
    LowerHex,
)]
pub struct Bitboard(pub u64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn piece_type_round_trips() {
        for pt in PieceType::ALL {
            assert_eq!(PieceType::from_index(pt.index()), Some(pt));
            assert_eq!(PieceType::from_letter(pt.letter()), Some(pt));
        }
        assert_eq!(PieceType::from_index(PieceType::COUNT), None);
        assert_eq!(PieceType::from_letter('Q'), None);
    }

    #[test]
    fn piece_round_trips() {
        for c in "PNBRQKpnbrqk".chars() {
            let piece = Piece::from_fen_char(c).expect("valid piece char");
            assert_eq!(piece.fen_char(), c);
        }
        assert_eq!(
            Piece::from_fen_char('N'),
            Some(Piece::new(Color::White, PieceType::Knight))
        );
        assert_eq!(Piece::from_fen_char('x'), None);
        assert_eq!(Piece::from_fen_char('1'), None);
    }

    #[test]
    fn piece_unicode_char() {
        let cases = &[
            ('k', '♔'),
            ('q', '♕'),
            ('r', '♖'),
            ('b', '♗'),
            ('n', '♘'),
            ('p', '♙'),
            ('K', '♚'),
            ('Q', '♛'),
            ('R', '♜'),
            ('B', '♝'),
            ('N', '♞'),
            ('P', '♟'),
        ];
        for &(fen_char, expected) in cases {
            let piece = Piece::from_fen_char(fen_char).expect("valid FEN letter");
            assert_eq!(
                expected,
                piece.unicode_char(),
                "unicode_char for {fen_char:?}"
            );
        }
    }

    #[test]
    fn square_coordinates() {
        for index in 0..Square::COUNT {
            let sq = Square::from_index(index).unwrap();
            let (file, rank) = ((index % 8) as u8, (index / 8) as u8);
            assert_eq!(index, sq.index(), "index of square {index}");
            assert_eq!(file, sq.file(), "file of square {index}");
            assert_eq!(rank, sq.rank(), "rank of square {index}");
            assert_eq!(
                Some(sq),
                Square::from_file_rank(file, rank),
                "from_file_rank({file}, {rank})"
            );
        }
        assert_eq!(None, Square::from_index(Square::COUNT), "index 64");
        assert_eq!(None, Square::from_file_rank(8, 0), "file 8");
        assert_eq!(None, Square::from_file_rank(0, 8), "rank 8");
    }

    #[test]
    fn square_names_match_coordinate_oracle() {
        const FILES: [char; 8] = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
        const RANKS: [char; 8] = ['1', '2', '3', '4', '5', '6', '7', '8'];
        for index in 0..Square::COUNT {
            let sq = Square::from_index(index).unwrap();
            let expected = format!("{}{}", FILES[index % 8], RANKS[index / 8]);
            assert_eq!(expected, sq.to_string(), "Display of square index {index}");
            assert_eq!(Ok(sq), expected.parse::<Square>(), "parse {expected:?}");
        }
    }

    #[test]
    fn square_rejects_bad_names() {
        type Cases = &'static [(&'static str, &'static str)];
        let cases: Cases = &[
            ("", "empty"),
            ("e", "too short"),
            ("e44", "too long"),
            ("4e", "rank before file"),
            ("e9", "rank out of range"),
            ("i1", "file out of range"),
            ("E4", "uppercase file"),
            ("\u{e9}4", "non-ASCII 'é'"),
            ("e4\u{0}", "trailing NUL"),
            ("\u{161}1", "'š' (U+0161): low byte is 'a'"),
            ("e\u{131}", "'ı' (U+0131): low byte is '1'"),
            ("\u{167}\u{138}", "'ŧĸ': low bytes are 'h' and '8'"),
        ];
        for &(bad, why) in cases {
            assert_eq!(
                Err(ParseError::InvalidSquare),
                bad.parse::<Square>(),
                "{bad:?}: {why}"
            );
        }
    }

    #[test]
    fn square_file_chars() {
        type Cases = &'static [(u8, char)];
        let cases: Cases = &[
            (0, 'a'),
            (1, 'b'),
            (2, 'c'),
            (3, 'd'),
            (4, 'e'),
            (5, 'f'),
            (6, 'g'),
            (7, 'h'),
        ];
        for &(file, c) in cases {
            assert_eq!(c, Square::file_char(file), "file_char({file})");
            assert_eq!(
                Some(file),
                Square::file_from_char(c),
                "file_from_char({c:?})"
            );
        }
    }

    #[test]
    fn square_rank_chars() {
        type Cases = &'static [(u8, char)];
        let cases: Cases = &[
            (0, '1'),
            (1, '2'),
            (2, '3'),
            (3, '4'),
            (4, '5'),
            (5, '6'),
            (6, '7'),
            (7, '8'),
        ];
        for &(rank, c) in cases {
            assert_eq!(c, Square::rank_char(rank), "rank_char({rank})");
            assert_eq!(
                Some(rank),
                Square::rank_from_char(c),
                "rank_from_char({c:?})"
            );
        }
    }

    #[test]
    fn square_from_char_rejects() {
        type Cases = &'static [(char, &'static str)];
        let files: Cases = &[
            ('`', "just below 'a'"),
            ('i', "just above 'h'"),
            ('A', "uppercase"),
            ('1', "a rank"),
            (' ', "space"),
            ('\u{161}', "'š' (U+0161): low byte is 0x61 = 'a'"),
            ('\u{167}', "'ŧ' (U+0167): low byte is 0x67 = 'h'"),
        ];
        for &(c, why) in files {
            assert_eq!(
                None,
                Square::file_from_char(c),
                "file_from_char({c:?}): {why}"
            );
        }

        let ranks: Cases = &[
            ('0', "just below '1'"),
            ('9', "just above '8'"),
            ('a', "a file"),
            (' ', "space"),
            ('\u{131}', "'ı' (U+0131): low byte is 0x31 = '1'"),
            ('\u{138}', "'ĸ' (U+0138): low byte is 0x38 = '8'"),
        ];
        for &(c, why) in ranks {
            assert_eq!(
                None,
                Square::rank_from_char(c),
                "rank_from_char({c:?}): {why}"
            );
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "expected file < 8")]
    fn square_file_char_out_of_range() {
        let _ = Square::file_char(8);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "expected rank < 8")]
    fn square_rank_char_out_of_range() {
        let _ = Square::rank_char(8);
    }

    #[test]
    fn square_flip_rank() {
        for index in 0..Square::COUNT {
            let sq = Square::from_index(index).unwrap();
            let expected = Square::from_file_rank(sq.file(), 7 - sq.rank()).unwrap();
            assert_eq!(expected, sq.flip_rank(), "flip_rank of {sq}");
            assert_eq!(sq, sq.flip_rank().flip_rank(), "flip_rank twice of {sq}");
        }
    }

    #[test]
    fn castling_round_trips() {
        for s in ["KQkq", "KQ", "kq", "Kq", "Qk", "K", "q", "-"] {
            let rights: CastlingRights = s.parse().unwrap();
            assert_eq!(rights.to_string(), s);
        }
        // Out-of-order input is accepted and normalised.
        assert_eq!("qK".parse::<CastlingRights>().unwrap().to_string(), "Kq");
    }

    #[test]
    fn castling_rejects_bad_input() {
        for bad in ["", "KK", "-K", "K-", "--", "x", "KQkqK"] {
            assert!(
                bad.parse::<CastlingRights>().is_err(),
                "{bad:?} should not parse"
            );
        }
    }

    #[test]
    fn castling_remove() {
        let mut rights = CastlingRights::ALL;
        rights.remove_one(Color::White, CastleSide::QueenSide);
        assert_eq!(rights.to_string(), "Kkq");
        rights.remove_both(Color::Black);
        assert_eq!(rights.to_string(), "K");
        rights.remove_one(Color::White, CastleSide::KingSide);
        assert_eq!(rights, CastlingRights::NONE);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "promotion piece must be")]
    fn move_invalid_promotion() {
        let sq = |s: &str| s.parse::<Square>().unwrap();
        let _invalid_move = Move::new(sq("e2"), sq("e4"), MoveKind::Promotion(PieceType::Pawn));
    }

    #[test]
    fn move_display() {
        let sq = |s: &str| s.parse::<Square>().unwrap();
        let quiet = Move::new(sq("e2"), sq("e4"), MoveKind::Normal);
        let promo = Move::new(sq("e7"), sq("e8"), MoveKind::Promotion(PieceType::Queen));
        assert_eq!(quiet.to_string(), "e2e4");
        assert_eq!(promo.to_string(), "e7e8q");
    }
}
