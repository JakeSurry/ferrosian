//! Core chess types: colors, pieces, squares, castling rights and moves.

use derive_more::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, LowerHex, Not};
use std::{fmt, str::FromStr};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Color
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// PieceType
// ---------------------------------------------------------------------------

impl PieceType {
    /// Every piece type, in index order.
    pub const ALL: [PieceType; 6] = [
        PieceType::Pawn,
        PieceType::Knight,
        PieceType::Bishop,
        PieceType::Rook,
        PieceType::Queen,
        PieceType::King,
    ];

    /// Number of piece types, for sizing arrays.
    pub const COUNT: usize = Self::ALL.len();

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
            PieceType::Pawn => 'p',
            PieceType::Knight => 'n',
            PieceType::Bishop => 'b',
            PieceType::Rook => 'r',
            PieceType::Queen => 'q',
            PieceType::King => 'k',
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

// ---------------------------------------------------------------------------
// Piece
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Square
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Square(u8);

impl Square {
    /// Number of squares, for sizing arrays.
    pub const COUNT: usize = 64;

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
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file = (b'a' + self.file()) as char;
        let rank = self.rank() + 1;
        write!(f, "{file}{rank}")
    }
}

impl FromStr for Square {
    type Err = ParseError;

    /// Parses a square name, e.g. `e4`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match *s.as_bytes() {
            [file @ b'a'..=b'h', rank @ b'1'..=b'8'] => Ok(Self((rank - b'1') * 8 + (file - b'a'))),
            _ => Err(ParseError::InvalidSquare),
        }
    }
}

// ---------------------------------------------------------------------------
// Castling
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Move
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Move {
    from: Square,
    to: Square,
    promotion: Option<PieceType>,
}

impl Move {
    pub fn new(from: Square, to: Square, promotion: Option<PieceType>) -> Self {
        Move {
            from,
            to,
            promotion,
        }
    }

    /// `square` where move originated.
    pub fn from(&self) -> Square {
        self.from
    }

    /// `square` were move ended up.
    pub fn to(&self) -> Square {
        self.to
    }

    /// `PieceType` of promotion piece, `None` if move was not a promotion.
    pub fn promotion(&self) -> Option<PieceType> {
        self.promotion
    }
}

impl fmt::Display for Move {
    /// Writes the move in UCI notation, e.g. `e2e4`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.from, self.to)?;
        if let Some(promotion) = self.promotion {
            write!(f, "{promotion}")?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Bitboard (impl in src/bitbord.rs)
// ---------------------------------------------------------------------------

/// A u64 newtype where each bit represents a square.
///
/// a1 = 0, h8 = 2.pow(63)
/// impl in [`crate::bitboards`]
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

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug, Error)]
pub enum ParseError {
    #[error("invalid square name")]
    InvalidSquare,

    #[error("invalid castling rights")]
    InvalidCastlingRights,

    #[error("invalid color char")]
    InvalidColor,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
    fn square_round_trips() {
        for index in 0..Square::COUNT {
            let sq = Square::from_index(index).unwrap();
            assert_eq!(sq.to_string().parse::<Square>(), Ok(sq));
            assert_eq!(Square::from_file_rank(sq.file(), sq.rank()), Some(sq));
        }
        assert_eq!(Square::from_index(64), None);
        assert_eq!(Square::from_file_rank(8, 0), None);
    }

    #[test]
    fn square_names() {
        assert_eq!(Square::from_index(0).unwrap().to_string(), "a1");
        assert_eq!(Square::from_index(28).unwrap().to_string(), "e4");
        assert_eq!(Square::from_index(63).unwrap().to_string(), "h8");
        for bad in ["", "e", "e9", "i1", "E4", "e44", "4e"] {
            assert!(bad.parse::<Square>().is_err(), "{bad:?} should not parse");
        }
    }

    #[test]
    fn square_from_file_rank_invalid_is_none() {
        assert_eq!(Square::from_file_rank(8, 0), None);
    }

    #[test]
    fn square_flip_rank() {
        let expected = Square::from_file_rank(0, 0).expect("0, 0 valid");
        let actual = Square::from_file_rank(0, 7)
            .expect("0, 7 valid")
            .flip_rank();
        assert_eq!(expected, actual);
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

    #[test]
    fn move_display() {
        let sq = |s: &str| s.parse::<Square>().unwrap();
        let quiet = Move {
            from: sq("e2"),
            to: sq("e4"),
            promotion: None,
        };
        let promo = Move {
            from: sq("e7"),
            to: sq("e8"),
            promotion: Some(PieceType::Queen),
        };
        assert_eq!(quiet.to_string(), "e2e4");
        assert_eq!(promo.to_string(), "e7e8q");
    }
}
