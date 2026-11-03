use crate::board::Board;
use crate::types::{Color, Move, MoveKind, Piece, PieceType, Square};

impl Board {
    pub fn make_move(&mut self, mv: Move) {
        let us = self.side_to_move;
        let them = us.opposite();
        let to = mv.to();
        let from = mv.from();

        let piece = self.remove_piece(from);
        let mut captured = self.piece_at(to).map(|_| self.remove_piece(to));

        self.en_passant_square = None;

        match mv.kind() {
            MoveKind::Normal => self.put_piece(to, piece),
            MoveKind::DoublePush => {
                self.put_piece(to, piece);

                let ep_square = Square::from_index((from.index() + to.index()) / 2);
                self.en_passant_square = ep_square;
            }
            MoveKind::EnPassant => {
                self.put_piece(to, piece);

                captured = Some(
                    self.remove_piece(
                        Square::from_file_rank(to.file(), from.rank())
                            .expect("valid move and position will yield valid file and rank"),
                    ),
                )
            }
            MoveKind::Promotion(piece_type) => self.put_piece(to, Piece::new(us, piece_type)),
            MoveKind::Castle(side) => { /* put the king, then move the rook */ }
        }

        if captured.is_some() || piece == Piece::new(us, PieceType::Pawn) {
            self.halfmove_clock = 0;
        } else {
            self.halfmove_clock += 1;
        }

        if us == Color::Black {
            self.fullmove_count += 1;
        }

        self.side_to_move = them;
    }
}

#[cfg(test)]
mod tests {
    use crate::board::Board;
    use crate::test_helpers::{STARTING_FEN, from_fen, sq};
    use crate::types::{Move, MoveKind, MoveKind::*, PieceType::*};

    /// (name, FEN before, from, to, kind, FEN after)
    type Row<'a> = (&'a str, &'a str, &'a str, &'a str, MoveKind, &'a str);

    /// Makes each row's move on its FEN and compares the whole resulting FEN,
    /// so placement, side to move, rights, en passant square and both clocks
    /// are checked together.
    #[track_caller]
    fn check_rows(rows: &[Row]) {
        for &(why, before, from, to, kind, expected) in rows {
            let mut board = from_fen(before);
            board.make_move(Move::new(sq(from), sq(to), kind));

            assert_eq!(
                Ok(()),
                board.is_consistent(),
                "{why}: {from}{to} left the board inconsistent"
            );
            let actual = board.to_fen();
            assert_eq!(
                expected, actual,
                "{why}: {from}{to} ({kind:?})\nbefore:   {before}\nexpected: {expected}\nactual:   {actual}\n{board}"
            );
        }
    }

    #[test]
    fn make_move_quiet_moves_and_captures() {
        let cases: &[Row] = &[
            (
                "White quiet knight move: halfmove + 1, fullmove unchanged",
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                "g1",
                "f3",
                Normal,
                "rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R b KQkq - 1 1",
            ),
            (
                "Black quiet knight move: halfmove + 1, fullmove + 1",
                "rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R b KQkq - 1 1",
                "b8",
                "c6",
                Normal,
                "r1bqkbnr/pppppppp/2n5/8/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 2 2",
            ),
            (
                "White capture resets halfmove",
                "4k3/8/3p4/8/4N3/8/8/4K3 w - - 5 20",
                "e4",
                "d6",
                Normal,
                "4k3/8/3N4/8/8/8/8/4K3 b - - 0 20",
            ),
            (
                "Black capture resets halfmove, fullmove + 1",
                "4k3/8/8/8/4n3/8/3P4/4K3 b - - 7 30",
                "e4",
                "d2",
                Normal,
                "4k3/8/8/8/8/8/3n4/4K3 w - - 0 31",
            ),
            (
                "White pawn single push resets halfmove",
                "4k3/8/8/8/8/8/4P3/4K3 w - - 9 40",
                "e2",
                "e3",
                Normal,
                "4k3/8/8/8/8/4P3/8/4K3 b - - 0 40",
            ),
            (
                "Black pawn single push resets halfmove",
                "4k3/4p3/8/8/8/8/8/4K3 b - - 9 40",
                "e7",
                "e6",
                Normal,
                "4k3/8/4p3/8/8/8/8/4K3 w - - 0 41",
            ),
            (
                "White pawn capture",
                "4k3/8/8/3p4/4P3/8/8/4K3 w - - 3 10",
                "e4",
                "d5",
                Normal,
                "4k3/8/8/3P4/8/8/8/4K3 b - - 0 10",
            ),
            (
                "Black pawn capture",
                "4k3/8/8/3p4/4P3/8/8/4K3 b - - 3 10",
                "d5",
                "e4",
                Normal,
                "4k3/8/8/8/4p3/8/8/4K3 w - - 0 11",
            ),
            (
                "White move clears the en passant square",
                "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2",
                "g1",
                "f3",
                Normal,
                "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2",
            ),
            (
                "Black move clears the en passant square",
                "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
                "g8",
                "f6",
                Normal,
                "rnbqkb1r/pppppppp/5n2/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2",
            ),
            (
                "White king move with no rights",
                "4k3/8/8/8/8/8/8/4K3 w - - 0 1",
                "e1",
                "d2",
                Normal,
                "4k3/8/8/8/8/8/3K4/8 b - - 1 1",
            ),
        ];
        check_rows(cases);
    }

    #[test]
    fn make_move_changes_only_the_copy() {
        let board = Board::starting_position();
        let mut next = board;

        next.make_move(Move::new(sq("g1"), sq("f3"), Normal));

        assert_eq!(STARTING_FEN, board.to_fen(), "the original is untouched");
        assert_ne!(board, next, "the copy moved");
    }

    #[test]
    #[should_panic]
    fn make_move_from_an_empty_square_panics() {
        let mut board = Board::starting_position();
        board.make_move(Move::new(sq("e4"), sq("e5"), Normal));
    }

    #[test]
    fn make_move_double_push_and_en_passant() {
        let cases: &[Row] = &[
            (
                "White double push sets e3",
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                "e2",
                "e4",
                DoublePush,
                "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
            ),
            (
                "Black double push sets e6",
                "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
                "e7",
                "e5",
                DoublePush,
                "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2",
            ),
            (
                "White double push on the a-file sets a3",
                "4k3/7p/8/8/8/8/P7/4K3 w - - 4 9",
                "a2",
                "a4",
                DoublePush,
                "4k3/7p/8/8/P7/8/8/4K3 b - a3 0 9",
            ),
            (
                "Black double push on the h-file sets h6",
                "4k3/7p/8/8/P7/8/8/4K3 b - a3 0 9",
                "h7",
                "h5",
                DoublePush,
                "4k3/8/8/7p/P7/8/8/4K3 w - h6 0 10",
            ),
            (
                "White en passant to the west removes the pawn on d5",
                "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 2",
                "e5",
                "d6",
                EnPassant,
                "4k3/8/3P4/8/8/8/8/4K3 b - - 0 2",
            ),
            (
                "White en passant to the east removes the pawn on f5",
                "4k3/8/8/4Pp2/8/8/8/4K3 w - f6 0 2",
                "e5",
                "f6",
                EnPassant,
                "4k3/8/5P2/8/8/8/8/4K3 b - - 0 2",
            ),
            (
                "Black en passant to the east removes the pawn on e4",
                "4k3/8/8/8/3pP3/8/8/4K3 b - e3 0 2",
                "d4",
                "e3",
                EnPassant,
                "4k3/8/8/8/8/4p3/8/4K3 w - - 0 3",
            ),
            (
                "Black en passant to the west removes the pawn on c4",
                "4k3/8/8/8/2Pp4/8/8/4K3 b - c3 0 2",
                "d4",
                "c3",
                EnPassant,
                "4k3/8/8/8/8/2p5/8/4K3 w - - 0 3",
            ),
        ];
        check_rows(cases);
    }

    #[test]
    fn make_move_promotion() {
        let cases: &[Row] = &[
            (
                "White promotes to a queen, halfmove resets",
                "4k3/P7/8/8/8/8/8/4K3 w - - 3 50",
                "a7",
                "a8",
                Promotion(Queen),
                "Q3k3/8/8/8/8/8/8/4K3 b - - 0 50",
            ),
            (
                "White promotes to a knight",
                "8/1P2k3/8/8/8/8/8/4K3 w - - 0 1",
                "b7",
                "b8",
                Promotion(Knight),
                "1N6/4k3/8/8/8/8/8/4K3 b - - 0 1",
            ),
            (
                "Black promotes to a queen",
                "4k3/8/8/8/8/8/7p/4K3 b - - 2 60",
                "h2",
                "h1",
                Promotion(Queen),
                "4k3/8/8/8/8/8/8/4K2q w - - 0 61",
            ),
            (
                "Black promotes to a rook",
                "4k3/8/8/8/8/8/6p1/4K3 b - - 0 1",
                "g2",
                "g1",
                Promotion(Rook),
                "4k3/8/8/8/8/8/8/4K1r1 w - - 0 2",
            ),
            (
                "White capture-promotion to a queen",
                "1r2k3/P7/8/8/8/8/8/4K3 w - - 6 70",
                "a7",
                "b8",
                Promotion(Queen),
                "1Q2k3/8/8/8/8/8/8/4K3 b - - 0 70",
            ),
            (
                "Black capture-promotion to a bishop",
                "4k3/8/8/8/8/8/1p6/2B1K3 b - - 6 70",
                "b2",
                "c1",
                Promotion(Bishop),
                "4k3/8/8/8/8/8/8/2b1K3 w - - 0 71",
            ),
        ];
        check_rows(cases);
    }
}
