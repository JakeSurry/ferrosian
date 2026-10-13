use crate::types::{Bitboard, Square};
use std::fmt;

impl Bitboard {
    pub const EMPTY: Self = Self(0);
    pub const FULL: Self = Self(u64::MAX);

    pub const FILE_A: Self = Self(0x0101_0101_0101_0101);
    pub const RANK_1: Self = Self(0xFF);

    pub const FILES: [Self; 8] = Self::build_files();
    pub const RANKS: [Self; 8] = Self::build_ranks();

    pub const FILE_B: Self = Self::FILES[1];
    pub const FILE_C: Self = Self::FILES[2];
    pub const FILE_D: Self = Self::FILES[3];
    pub const FILE_E: Self = Self::FILES[4];
    pub const FILE_F: Self = Self::FILES[5];
    pub const FILE_G: Self = Self::FILES[6];
    pub const FILE_H: Self = Self::FILES[7];

    pub const RANK_2: Self = Self::RANKS[1];
    pub const RANK_3: Self = Self::RANKS[2];
    pub const RANK_4: Self = Self::RANKS[3];
    pub const RANK_5: Self = Self::RANKS[4];
    pub const RANK_6: Self = Self::RANKS[5];
    pub const RANK_7: Self = Self::RANKS[6];
    pub const RANK_8: Self = Self::RANKS[7];

    const NOT_A_FILE: Self = Self(!Self::FILE_A.0);
    const NOT_H_FILE: Self = Self(!Self::FILE_H.0);

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, square: Square) -> bool {
        self.0 & square.bitboard().0 != 0
    }

    pub const fn count(self) -> u32 {
        self.0.count_ones()
    }

    pub const fn north(self) -> Self {
        Self(self.0 << 8)
    }

    pub const fn south(self) -> Self {
        Self(self.0 >> 8)
    }

    pub const fn east(self) -> Self {
        Self((self.0 << 1) & Self::NOT_A_FILE.0)
    }

    pub const fn west(self) -> Self {
        Self((self.0 >> 1) & Self::NOT_H_FILE.0)
    }

    pub const fn north_east(self) -> Self {
        Self((self.0 << 9) & Self::NOT_A_FILE.0)
    }

    pub const fn north_west(self) -> Self {
        Self((self.0 << 7) & Self::NOT_H_FILE.0)
    }

    pub const fn south_east(self) -> Self {
        Self((self.0 >> 7) & Self::NOT_A_FILE.0)
    }

    pub const fn south_west(self) -> Self {
        Self((self.0 >> 9) & Self::NOT_H_FILE.0)
    }

    const fn build_files() -> [Self; 8] {
        let mut files = [Self::EMPTY; 8];
        let mut i = 0;

        while i < 8 {
            files[i] = Self(Self::FILE_A.0 << i);
            i += 1;
        }

        files
    }

    const fn build_ranks() -> [Self; 8] {
        let mut ranks = [Self::EMPTY; 8];
        let mut i = 0;

        while i < 8 {
            ranks[i] = Self(Self::RANK_1.0 << (8 * i));
            i += 1;
        }

        ranks
    }
}

impl fmt::Display for Bitboard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for rank in (0..8).rev() {
            write!(f, "{}", rank + 1)?;
            for file in 0..8 {
                let square = Square::from_file_rank(file, rank).expect("file and rank are in 0..8");
                let c = if self.contains(square) { "x" } else { "." };
                write!(f, " {c}")?;
            }
            writeln!(f)?;
        }
        writeln!(f, "  a b c d e f g h")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn files_and_ranks_partition_the_board() {
        for index in 0..64 {
            let square = Square::from_index(index).expect("square valid in 0..64");
            let files_in = Bitboard::FILES.iter().fold(0, |acc, &f| {
                if f & square.bitboard() != Bitboard::EMPTY {
                    acc + 1
                } else {
                    acc
                }
            });
            assert_eq!(files_in, 1);
            let square = Square::from_index(index).expect("square valid in 0..64");
            let ranks_in = Bitboard::RANKS.iter().fold(0, |acc, &r| {
                if r & square.bitboard() != Bitboard::EMPTY {
                    acc + 1
                } else {
                    acc
                }
            });
            assert_eq!(ranks_in, 1)
        }
    }

    #[test]
    fn file_rank_intersect_at_only_one_square() {
        for file in 0..8 {
            for rank in 0..8 {
                assert_eq!(
                    Bitboard::FILES[file] & Bitboard::RANKS[rank],
                    Square::from_file_rank(file as u8, rank as u8)
                        .expect("square valid in file 0..8 rank 0..8")
                        .bitboard()
                );
            }
        }
    }

    #[test]
    fn shift_shifts_squares() {
        type Shift = fn(Bitboard) -> Bitboard;

        let directions: [(&str, Shift, i8, i8); 8] = [
            ("north", Bitboard::north, 0, 1),
            ("east", Bitboard::east, 1, 0),
            ("south", Bitboard::south, 0, -1),
            ("west", Bitboard::west, -1, 0),
            ("north_east", Bitboard::north_east, 1, 1),
            ("north_west", Bitboard::north_west, -1, 1),
            ("south_east", Bitboard::south_east, 1, -1),
            ("south_west", Bitboard::south_west, -1, -1),
        ];
        for file in 0..8 {
            for rank in 0..8 {
                let square = Square::from_file_rank(file, rank).expect("square valid in file 0..8");
                for (name, shift, file_delta, rank_delta) in directions {
                    let expected = square
                        .file()
                        .checked_add_signed(file_delta)
                        .zip(square.rank().checked_add_signed(rank_delta))
                        .and_then(|(file, rank)| Square::from_file_rank(file, rank))
                        .map_or(Bitboard::EMPTY, Square::bitboard);
                    let actual = shift(square.bitboard());
                    assert_eq!(
                        expected, actual,
                        "{name} from {square}\nexpected:\n{expected}\nactual:\n{actual}"
                    );
                }
            }
        }
    }

    #[test]
    fn empty_bitboard_displays_empty_grid() {
        let bitboard_grid = Bitboard(0);
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
        let bitboard_grid = Bitboard(u64::MAX);
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
        let bitboard_grid = Bitboard(1_u64 << 28);
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
