use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum Color {
    White,
    Black,
}

impl Color {
    fn opposite(&self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
    fn num(&self) -> u8 {
        match self {
            Color::White => 1,
            Color::Black => 0,
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
    pub const ALL: [PieceType; 6] = [
        PieceType::Pawn,
        PieceType::Knight,
        PieceType::Bishop,
        PieceType::Rook,
        PieceType::Queen,
        PieceType::King,
    ];

    pub const COUNT: usize = Self::ALL.len();

    pub const fn index(self) -> usize {
        self as usize
    }

    pub fn from_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }

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
struct Piece {
    color: Color,
    piece_type: PieceType,
}

impl Piece {
    fn from_fen(fen_char: char) -> Option<Self> {
        let color = match fen_char.is_uppercase() {
            true => Color::White,
            false => Color::Black,
        };
        let piece_type = PieceType::from_letter(fen_char)?;
        Some(Self { color, piece_type })
    }
    fn get_fen(&self) -> char {
        let fen_char = self.piece_type.letter();
        match self.color {
            Color::White => fen_char.to_ascii_uppercase(),
            Color::Black => fen_char,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
struct Square(u8);

impl Square {
    const FILES: [char; 8] = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];

    fn from_idx(idx: u8) -> Option<Self> {
        if idx > 63 {
            return None;
        };
        Some(Self(idx))
    }
    fn from_rank_and_file(rank: u8, file: u8) -> Result<Self, String> {
        if rank > 7 || rank == 0 {
            return Err("Rank out of domain {0, 7}".to_string());
        };
        if file > 7 || file == 0 {
            return Err("File out of domain {a-h}".to_string());
        };
        Ok(Self(rank * 8 + file))
    }
    fn idx(&self) -> u8 {
        self.0
    }
    fn rank(&self) -> u8 {
        self.0 / 8
    }
    fn file(&self) -> char {
        let file_idx = (self.0 % 8) as usize;
        Self::FILES[file_idx]
    }
    fn bitboard(&self) -> u64 {
        1u64 << self.0
    }
    fn fmt_square(square: Self) -> String {
        format!("{}{}", square.file(), square.rank())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum CastleSide {
    KingSide,
    QueenSide,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
struct CastlingRights(u8);

impl CastlingRights {
    const WK_BIT: u8 = 0b0000_0001;
    const WQ_BIT: u8 = 0b0000_0010;
    const BK_BIT: u8 = 0b0000_0100;
    const BQ_BIT: u8 = 0b0000_1000;

    fn can_castle(&self, color: Color, castle_side: CastleSide) -> bool {
        let shift = match (color, castle_side) {
            (Color::White, CastleSide::KingSide) => 0,
            (Color::White, CastleSide::QueenSide) => 1,
            (Color::Black, CastleSide::KingSide) => 2,
            (Color::Black, CastleSide::QueenSide) => 3,
        };

        (self.0 & (1u8 << shift)) != 0
    }
    fn remove_one_right(&mut self, color: Color, castle_side: CastleSide) {
        match (color, castle_side) {
            (Color::White, CastleSide::KingSide) => self.0 &= !(Self::WK_BIT),
            (Color::White, CastleSide::QueenSide) => self.0 &= !(Self::WQ_BIT),
            (Color::Black, CastleSide::KingSide) => self.0 &= !(Self::BK_BIT),
            (Color::Black, CastleSide::QueenSide) => self.0 &= !(Self::BQ_BIT),
        }
    }
    fn remove_both_right(&mut self, color: Color) {
        match color {
            Color::White => self.0 &= !(Self::WK_BIT + Self::WQ_BIT),
            Color::Black => self.0 &= !(Self::BK_BIT + Self::BQ_BIT),
        }
    }
    fn rights_from_fen(fen: String) -> Result<Self, String> {
        if fen.len() > 4
            || fen.is_empty()
            || fen
                .chars()
                .any(|c| !matches!(c, 'K' | 'Q' | 'k' | 'q' | '-'))
        {
            return Err("Invalid FEN".to_string());
        }

        let mut rights: u8 = 0;
        if fen.contains("K") {
            rights |= Self::WK_BIT;
        }
        if fen.contains("Q") {
            rights |= Self::WQ_BIT;
        }
        if fen.contains("k") {
            rights |= Self::BK_BIT;
        }
        if fen.contains("q") {
            rights |= Self::BQ_BIT;
        }
        Ok(Self(rights))
    }
    fn fen(&self) -> String {
        let mut fen: String = String::new();
        if (self.0 & Self::WK_BIT) != 0 {
            fen.push('K');
        }
        if (self.0 & Self::WQ_BIT) != 0 {
            fen.push('Q');
        }
        if (self.0 & Self::BK_BIT) != 0 {
            fen.push('k');
        }
        if (self.0 & Self::BQ_BIT) != 0 {
            fen.push('q');
        }
        if fen.is_empty() {
            fen.push('-');
        }
        fen
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
struct Move {
    from: Square,
    to: Square,
    promotion: Option<PieceType>,
}

impl Move {
    fn display_uci(&self) -> String {
        let promo_str = match self.promotion {
            Some(c) => c.to_string(), // LIKELY BROKEN AS c IS TYPE 'Square'
            None => String::new(),
        };

        format!(
            "{}{}{}",
            Square::fmt_square(self.from),
            Square::fmt_square(self.to),
            promo_str
        )
    }
}
