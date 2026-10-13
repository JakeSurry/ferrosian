use crate::types::Square;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BitboardGrid(pub u64);

impl fmt::Display for BitboardGrid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for rank in (0..8).rev() {
            write!(f, "{}", rank + 1)?;
            for file in 0..8 {
                let square = Square::from_file_rank(file, rank).expect("file and rank are in 0..8");
                let c = if self.0 & square.bitboard() != 0 {
                    "x"
                } else {
                    "."
                };
                write!(f, " {c}")?;
            }
            writeln!(f)?;
        }
        writeln!(f, "  a b c d e f g h")
    }
}

pub const FILE_A: u64 = 0x0101_0101_0101_0101;
pub const RANK_1: u64 = 0xFF;

pub const FILES: [u64; 8] = build_files();
pub const RANKS: [u64; 8] = build_ranks();

pub const FILE_B: u64 = FILES[1];
pub const FILE_C: u64 = FILES[2];
pub const FILE_D: u64 = FILES[3];
pub const FILE_E: u64 = FILES[4];
pub const FILE_F: u64 = FILES[5];
pub const FILE_G: u64 = FILES[6];
pub const FILE_H: u64 = FILES[7];

pub const RANK_2: u64 = RANKS[1];
pub const RANK_3: u64 = RANKS[2];
pub const RANK_4: u64 = RANKS[3];
pub const RANK_5: u64 = RANKS[4];
pub const RANK_6: u64 = RANKS[5];
pub const RANK_7: u64 = RANKS[6];
pub const RANK_8: u64 = RANKS[7];

const NOT_A_FILE: u64 = !FILE_A;
const NOT_H_FILE: u64 = !FILE_H;

const fn build_files() -> [u64; 8] {
    let mut files = [0; 8];
    let mut i = 0;

    while i < 8 {
        files[i] = FILE_A << i;
        i += 1;
    }

    files
}

const fn build_ranks() -> [u64; 8] {
    let mut ranks = [0; 8];
    let mut i = 0;

    while i < 8 {
        ranks[i] = RANK_1 << (8 * i);
        i += 1;
    }

    ranks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_bitboard_displays_empty_grid() {
        let bitboard_grid = BitboardGrid(0);
        let expected = "\
8 . . . . . . . .
7 . . . . . . . .
6 . . . . . . . .
5 . . . . . . . .
4 . . . . . . . .
3 . . . . . . . .
2 . . . . . . . .
1 . . . . . . . .
  a b c d e f g h
";
        assert_eq!(bitboard_grid.to_string(), expected);
    }

    #[test]
    fn full_bitboard_displays_full_grid() {
        let bitboard_grid = BitboardGrid(u64::MAX);
        let expected = "\
8 x x x x x x x x
7 x x x x x x x x
6 x x x x x x x x
5 x x x x x x x x
4 x x x x x x x x
3 x x x x x x x x
2 x x x x x x x x
1 x x x x x x x x
  a b c d e f g h
";
        assert_eq!(bitboard_grid.to_string(), expected);
    }

    #[test]
    fn e4_bitboard_displays_only_e4() {
        let bitboard_grid = BitboardGrid(1_u64 << 28);
        let expected = "\
8 . . . . . . . .
7 . . . . . . . .
6 . . . . . . . .
5 . . . . . . . .
4 . . . . x . . .
3 . . . . . . . .
2 . . . . . . . .
1 . . . . . . . .
  a b c d e f g h
";
        assert_eq!(bitboard_grid.to_string(), expected);
    }
}
