//! Bitboard logic, defined in types.rs

use crate::types::{Bitboard, Square};
use std::{fmt, iter::FusedIterator};

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

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn union_all(bbs: &[Self]) -> Self {
        let mut index = 0;
        let mut bb_acc: u64 = 0;

        while index < bbs.len() {
            bb_acc |= bbs[index].0;
            index += 1;
        }

        Self(bb_acc)
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, square: Square) -> bool {
        self.0 & square.bitboard().0 != 0
    }

    pub const fn overlaps(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    pub const fn count(self) -> u8 {
        self.0.count_ones() as u8
    }

    pub const fn flip_vertical(self) -> Self {
        Self(self.0.swap_bytes())
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

    pub const fn lsb(self) -> Option<Square> {
        Square::from_index(self.0.trailing_zeros() as usize)
    }

    pub fn pop_lsb(&mut self) -> Option<Square> {
        let square: Square = self.lsb()?;
        self.0 &= self.0 - 1;
        Some(square)
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

impl FromIterator<Square> for Bitboard {
    fn from_iter<T: IntoIterator<Item = Square>>(iter: T) -> Self {
        iter.into_iter()
            .fold(Bitboard::EMPTY, |bb, sq| bb | sq.bitboard())
    }
}

pub struct Squares(Bitboard);

impl Iterator for Squares {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop_lsb()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.0.count() as usize;
        (n, Some(n))
    }
}

impl ExactSizeIterator for Squares {}
impl FusedIterator for Squares {}

impl IntoIterator for Bitboard {
    type Item = Square;
    type IntoIter = Squares;

    fn into_iter(self) -> Self::IntoIter {
        Squares(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_unites_two_bitboards() {
        let expected = Bitboard(3);
        let actual = Bitboard(1).union(Bitboard(2));
        assert_eq!(expected, actual);
    }

    #[test]
    fn union_all_unites_all_bitboards() {
        let expected = Bitboard(7);
        let actual = Bitboard::union_all(&[Bitboard(1), Bitboard(1), Bitboard(2), Bitboard(4)]);
        assert_eq!(expected, actual);
    }

    #[test]
    fn files_and_ranks_partition_the_board() {
        for index in 0..64 {
            let square = Square::from_index(index).expect("square valid in 0..64");
            let files_in = Bitboard::FILES
                .iter()
                .filter(|f| f.contains(square))
                .count();
            assert_eq!(files_in, 1, "square {square}");
            let ranks_in = Bitboard::RANKS
                .iter()
                .filter(|f| f.contains(square))
                .count();
            assert_eq!(ranks_in, 1, "square {square}");
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
                        .bitboard(),
                    "file {file}, rank {rank}"
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
    fn flip_vertical() {
        let expected = Bitboard(1);
        let actual = Bitboard(2_u64.pow(56)).flip_vertical();
        assert_eq!(expected, actual);
    }

    // ---- Square iteration: lsb, pop_lsb, IntoIterator, FromIterator ----

    /// Bitboards for the iteration tests.    
    const ITERATION_CASES: &[(&str, Bitboard)] = &[
        ("empty", Bitboard::EMPTY),
        ("full", Bitboard::FULL),
        ("a1 alone", Bitboard(1)),
        ("h8 alone", Bitboard(1 << 63)),
        ("a1 and h8", Bitboard(1 | 1 << 63)),
        ("e-file", Bitboard::FILE_E),
        ("rank 7", Bitboard::RANK_7),
        ("checkerboard", Bitboard(0x55AA_55AA_55AA_55AA)),
        ("scattered", Bitboard(0x8040_2010_0C04_0281)),
        ("sparse high", Bitboard(0xF000_0000_0000_0010)),
    ];

    /// Reference: the set squares found by testing each bit index in turn.
    /// Independent of `lsb`/`pop_lsb`, and ascending by construction.
    fn squares_by_bit_test(bb: Bitboard) -> Vec<Square> {
        (0..64)
            .filter(|&i| bb.0 & (1 << i) != 0)
            .map(|i| Square::from_index(i).expect("index below 64"))
            .collect()
    }

    #[test]
    fn lsb_of_empty_is_none() {
        assert_eq!(Bitboard::EMPTY.lsb(), None);
    }

    #[test]
    fn lsb_of_single_bit_is_that_square() {
        for i in 0..64 {
            let bb = Bitboard(1 << i);
            assert_eq!(bb.lsb().map(Square::index), Some(i), "bit {i}");
        }
    }

    #[test]
    fn lsb_picks_lowest_of_several() {
        let cases: &[(Bitboard, usize, &str)] = &[
            (Bitboard::FULL, 0, "full board starts at a1"),
            (Bitboard(1 << 63 | 1 << 5), 5, "f1 below h8"),
            (Bitboard::FILE_H, 7, "h-file starts at h1"),
            (Bitboard::RANK_8, 56, "rank 8 starts at a8"),
        ];
        for &(bb, expected, why) in cases {
            assert_eq!(bb.lsb().map(Square::index), Some(expected), "{why}\n{bb}");
        }
    }

    #[test]
    fn pop_lsb_removes_one_square_at_a_time() {
        let mut bb = Bitboard(1 << 63 | 1 << 5);

        assert_eq!(bb.pop_lsb().map(Square::index), Some(5), "first pop is f1");
        assert_eq!(bb, Bitboard(1 << 63), "only h8 left\n{bb}");

        assert_eq!(
            bb.pop_lsb().map(Square::index),
            Some(63),
            "second pop is h8"
        );
        assert_eq!(bb, Bitboard::EMPTY, "nothing left\n{bb}");

        assert_eq!(bb.pop_lsb(), None);
        assert_eq!(bb, Bitboard::EMPTY);
    }

    #[test]
    fn iteration_matches_bit_test_in_ascending_order() {
        for &(name, bb) in ITERATION_CASES {
            let actual: Vec<Square> = bb.into_iter().collect();
            assert_eq!(actual, squares_by_bit_test(bb), "{name}\n{bb}");
        }
    }

    #[test]
    fn collect_round_trips() {
        for &(name, bb) in ITERATION_CASES {
            let rebuilt: Bitboard = bb.into_iter().collect();
            assert_eq!(rebuilt, bb, "{name}\nexpected:\n{bb}\nactual:\n{rebuilt}");
        }
    }

    #[test]
    fn collect_from_square_list() {
        let a1 = Square::from_index(0).unwrap();
        let h8 = Square::from_index(63).unwrap();

        let none: Bitboard = std::iter::empty().collect();
        assert_eq!(none, Bitboard::EMPTY, "no squares");

        let corners: Bitboard = [a1, h8].into_iter().collect();
        assert_eq!(corners, Bitboard(1 | 1 << 63), "a1 and h8\n{corners}");

        let repeated: Bitboard = [a1, a1, h8].into_iter().collect();
        assert_eq!(repeated, corners, "duplicates\n{repeated}");
    }

    #[test]
    fn len_counts_down_and_iterator_stays_empty() {
        for &(name, bb) in ITERATION_CASES {
            let mut squares = bb.into_iter();
            let mut remaining = usize::from(bb.count());
            assert_eq!(squares.len(), remaining, "{name}: initial len");

            while squares.next().is_some() {
                remaining = remaining
                    .checked_sub(1)
                    .unwrap_or_else(|| panic!("{name}: yielded more than count()"));
                assert_eq!(squares.len(), remaining, "{name}: len after next");
            }

            assert_eq!(remaining, 0, "{name}: yielded fewer than count()");
            assert_eq!(squares.next(), None, "{name}: after exhaustion");
            assert_eq!(squares.next(), None, "{name}: still exhausted");
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
