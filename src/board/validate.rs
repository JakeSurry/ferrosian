//! Validate a given position

use crate::board::Board;
use crate::types::{Bitboard, CastleSide, Color, Piece, PieceType, Square};
use core::fmt;
use thiserror::Error;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Error)]
pub enum PositionError {
    #[error("expected one king per color, found {count} kings for {color}")]
    InvalidKingCount { color: Color, count: u8 },

    #[error("pawn on back rank: {0}")]
    PawnOnBackRank(Square),

    #[error("{0} in check on opponent's turn")]
    SideNotToMoveInCheck(Color),

    #[error("{color} cannot castle {side}")]
    InvalidCastlingRights { color: Color, side: CastleSide },

    #[error("invalid en passant square {square}: {reason}")]
    InvalidEnPassantSquare {
        square: Square,
        reason: EnPassantError,
    },

    #[error("too many {color} pawns, expected <=8, found {count}")]
    TooManyPawns { color: Color, count: u8 },

    #[error("expected a fullmove count >0, got 0")]
    ZeroFullmoveCount,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Error)]
pub enum EnPassantError {
    #[error("wrong rank for side to move")]
    WrongRank,

    #[error("no pushed pawn in front of it")]
    NoPushedPawn,

    #[error("square and/or the one behind is occupied")]
    Occupied,

    #[error("expected halfmove clock of 0, got {0}")]
    NonZeroHalfmoveClock(u16),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PositionErrors(Vec<PositionError>);

impl PositionErrors {
    pub fn errors(&self) -> &[PositionError] {
        self.0.as_slice()
    }
}

impl std::error::Error for PositionErrors {}

impl fmt::Display for PositionErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid position:")?;
        for err in &self.0 {
            write!(f, "\n  - {err}")?;
        }
        Ok(())
    }
}

impl Board {
    pub fn validate(&self) -> Result<(), PositionErrors> {
        let mut errors: Vec<PositionError> = Vec::new();

        self.validate_king_count(&mut errors);
        self.validate_pawn_count(&mut errors);
        self.validate_pawn_positions(&mut errors);
        self.validate_castle_rights(&mut errors);
        self.validate_opponent_not_in_check(&mut errors);
        self.validate_en_passant_square(&mut errors);
        self.validate_non_zero_fullmove_count(&mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(PositionErrors(errors))
        }
    }

    fn validate_king_count(&self, errors: &mut Vec<PositionError>) {
        for color in Color::ALL {
            let king_bb = self.pieces(color, PieceType::King);
            let king_count = king_bb.count();

            if king_count != 1 {
                errors.push(PositionError::InvalidKingCount {
                    color,
                    count: king_count,
                });
            }
        }
    }

    fn validate_pawn_count(&self, errors: &mut Vec<PositionError>) {
        for color in Color::ALL {
            let pawn_bb = self.pieces(color, PieceType::Pawn);
            let pawn_count = pawn_bb.count();

            if pawn_count > 8 {
                errors.push(PositionError::TooManyPawns {
                    color,
                    count: pawn_count,
                });
            }
        }
    }

    fn validate_pawn_positions(&self, errors: &mut Vec<PositionError>) {
        let pawn_bb =
            self.pieces(Color::White, PieceType::Pawn) | self.pieces(Color::Black, PieceType::Pawn);
        let terminal_pawns = (Bitboard::RANK_1 | Bitboard::RANK_8) & pawn_bb;

        for square in terminal_pawns.to_squares() {
            errors.push(PositionError::PawnOnBackRank(square));
        }
    }

    fn validate_castle_rights(&self, errors: &mut Vec<PositionError>) {
        for color in Color::ALL {
            let king = Piece::new(color, PieceType::King);
            let rook = Piece::new(color, PieceType::Rook);
            for side in CastleSide::ALL {
                let path = side.path(color);
                let pieces_home = self.piece_at(path.king_from) == Some(king)
                    && self.piece_at(path.rook_from) == Some(rook);
                if self.castling_rights().has(color, side) && !pieces_home {
                    errors.push(PositionError::InvalidCastlingRights { color, side });
                }
            }
        }
    }

    fn validate_opponent_not_in_check(&self, errors: &mut Vec<PositionError>) {
        let side_not_to_move = self.side_to_move().opposite();

        if self.is_in_check(side_not_to_move) {
            errors.push(PositionError::SideNotToMoveInCheck(side_not_to_move));
        }
    }

    fn validate_en_passant_square(&self, errors: &mut Vec<PositionError>) {
        if let Some(square) = self.en_passant_square() {
            let just_moved = self.side_to_move().opposite();

            let (rank, pushed_pawn, origin) = match just_moved {
                Color::White => (
                    Bitboard::RANK_3,
                    square.bitboard().north(),
                    square.bitboard().south(),
                ),
                Color::Black => (
                    Bitboard::RANK_6,
                    square.bitboard().south(),
                    square.bitboard().north(),
                ),
            };

            if !self
                .pieces(just_moved, PieceType::Pawn)
                .overlaps(pushed_pawn)
            {
                errors.push(PositionError::InvalidEnPassantSquare {
                    square,
                    reason: EnPassantError::NoPushedPawn,
                });
            } else if self.all_occupancy().overlaps(square.bitboard() | origin) {
                errors.push(PositionError::InvalidEnPassantSquare {
                    square,
                    reason: EnPassantError::Occupied,
                });
            } else if !rank.contains(square) {
                errors.push(PositionError::InvalidEnPassantSquare {
                    square,
                    reason: EnPassantError::WrongRank,
                });
            } else if self.halfmove_clock() != 0 {
                errors.push(PositionError::InvalidEnPassantSquare {
                    square,
                    reason: EnPassantError::NonZeroHalfmoveClock(self.halfmove_clock()),
                });
            }
        }
    }

    fn validate_non_zero_fullmove_count(&self, errors: &mut Vec<PositionError>) {
        let count = self.fullmove_count();
        if count == 0 {
            errors.push(PositionError::ZeroFullmoveCount)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::test_helpers::sq;
    use Color::*;
    use PositionError::*;

    type Cases<'a> = &'a [(&'a str, &'a str, &'a [PositionError])];
    type ValidateFunc = fn(&Board, &mut Vec<PositionError>);

    fn check_cases(cases: Cases, validate: ValidateFunc) {
        for &(name, fen, expected) in cases {
            let mut errors: Vec<PositionError> = Vec::new();
            let board: Board = fen.parse().expect("case fen is valid");

            validate(&board, &mut errors);
            let actual = errors;
            assert_eq!(actual, expected, "\n{board}\n\n{name}:",);
        }
    }

    #[test]
    fn validate_king_count() {
        let cases: Cases = &[
            (
                "0 kings",
                "8/8/8/8/8/8/8/8 w - - 0 1",
                &[
                    InvalidKingCount {
                        color: White,
                        count: 0,
                    },
                    InvalidKingCount {
                        color: Black,
                        count: 0,
                    },
                ],
            ),
            (
                "1 king",
                "8/8/8/8/8/8/8/K7 w - - 0 1",
                &[InvalidKingCount {
                    color: Black,
                    count: 0,
                }],
            ),
            (
                "2 kings on same side",
                "k7/8/8/8/8/8/8/KK6 w - - 0 1",
                &[InvalidKingCount {
                    color: White,
                    count: 2,
                }],
            ),
            ("2 kings valid", "k7/8/8/8/8/8/8/K7 w - - 0 1", &[]),
        ];

        check_cases(cases, Board::validate_king_count);
    }

    #[test]
    fn validate_pawn_count() {
        let cases: Cases = &[
            ("0 pawns", "8/8/8/8/8/8/8/8 w - - 0 1", &[]),
            ("1 pawn", "8/8/8/8/8/8/P7/8 w - - 0 1", &[]),
            ("16 pawns", "8/pppppppp/8/8/8/8/PPPPPPPP/8 w - - 0 1", &[]),
            (
                "9 pawns same side",
                "8/8/8/8/8/P7/PPPPPPPP/8 w - - 0 1",
                &[TooManyPawns {
                    color: White,
                    count: 9,
                }],
            ),
        ];

        check_cases(cases, Board::validate_pawn_count);
    }

    #[test]
    fn validate_pawn_positions() {
        let cases: Cases = &[
            (
                "pawn on 8th",
                "P7/8/8/8/8/8/8/8 w - - 0 1",
                &[PawnOnBackRank("a8".parse().unwrap())],
            ),
            (
                "pawn on 1st",
                "8/8/8/8/8/8/8/P7 w - - 0 1",
                &[PawnOnBackRank("a1".parse().unwrap())],
            ),
            ("pawn on 2nd", "8/8/8/8/8/8/P7/8 w - - 0 1", &[]),
        ];

        check_cases(cases, Board::validate_pawn_positions);
    }

    #[test]
    fn validate_castle_rights() {
        use CastleSide::*;

        let cases: Cases = &[
            (
                "missing kingside rook kingside castle",
                "r3k3/8/8/8/8/8/8/R3K3 w Kk - 0 1",
                &[
                    InvalidCastlingRights {
                        color: White,
                        side: KingSide,
                    },
                    InvalidCastlingRights {
                        color: Black,
                        side: KingSide,
                    },
                ],
            ),
            (
                "missing queenside rook queenside castle",
                "4k2r/8/8/8/8/8/8/4K2R w Qq - 0 1",
                &[
                    InvalidCastlingRights {
                        color: White,
                        side: QueenSide,
                    },
                    InvalidCastlingRights {
                        color: Black,
                        side: QueenSide,
                    },
                ],
            ),
            (
                "missing kingside rook queenside castle",
                "r3k3/8/8/8/8/8/8/R3K3 w Qq - 0 1",
                &[],
            ),
            (
                "missing king both castle",
                "r6r/8/8/8/8/8/8/R6R w KQkq - 0 1",
                &[
                    InvalidCastlingRights {
                        color: White,
                        side: KingSide,
                    },
                    InvalidCastlingRights {
                        color: White,
                        side: QueenSide,
                    },
                    InvalidCastlingRights {
                        color: Black,
                        side: KingSide,
                    },
                    InvalidCastlingRights {
                        color: Black,
                        side: QueenSide,
                    },
                ],
            ),
            (
                "king and both rooks",
                "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
                &[],
            ),
        ];

        check_cases(cases, Board::validate_castle_rights);
    }

    #[test]
    fn validate_opponent_not_in_check() {
        let cases: Cases = &[
            (
                "opponent in check",
                "8/8/8/8/8/8/8/r6K b - - 0 1",
                &[SideNotToMoveInCheck(White)],
            ),
            ("self in check", "8/8/8/8/8/8/8/r6K w - - 0 1", &[]),
        ];

        check_cases(cases, Board::validate_opponent_not_in_check);
    }

    #[test]
    fn validate_en_passant_square() {
        let cases: Cases = &[
            (
                "missing pawn",
                "8/8/8/8/8/8/8/8 b - a3 0 1",
                &[InvalidEnPassantSquare {
                    square: sq("a3"),
                    reason: EnPassantError::NoPushedPawn,
                }],
            ),
            (
                "wrong rank",
                "8/8/8/8/P7/8/8/8 w - a3 0 1",
                &[InvalidEnPassantSquare {
                    square: sq("a3"),
                    reason: EnPassantError::NoPushedPawn,
                }],
            ),
            (
                "occupied origin square",
                "8/8/8/8/P7/8/P7/8 b - a3 0 1",
                &[InvalidEnPassantSquare {
                    square: sq("a3"),
                    reason: EnPassantError::Occupied,
                }],
            ),
            (
                "occupied skip square",
                "8/8/8/8/P7/P7/8/8 b - a3 0 1",
                &[InvalidEnPassantSquare {
                    square: sq("a3"),
                    reason: EnPassantError::Occupied,
                }],
            ),
            (
                ">0 halfmove count",
                "8/8/8/8/P7/8/8/8 b - a3 1 1",
                &[InvalidEnPassantSquare {
                    square: sq("a3"),
                    reason: EnPassantError::NonZeroHalfmoveClock(1),
                }],
            ),
            (
                "wrong pawn color",
                "8/8/8/8/p7/8/8/8 b - a3 0 1",
                &[InvalidEnPassantSquare {
                    square: sq("a3"),
                    reason: EnPassantError::NoPushedPawn,
                }],
            ),
            ("valid", "8/8/8/8/P7/8/8/8 b - a3 0 1", &[]),
        ];

        check_cases(cases, Board::validate_en_passant_square);
    }

    #[test]
    fn validate_non_zero_fullmove_count() {
        let cases: Cases = &[
            (
                "zero fullmove count",
                "8/8/8/8/8/8/8/8 w - - 0 0",
                &[ZeroFullmoveCount],
            ),
            ("positive fullmove count", "8/8/8/8/8/8/8/8 w - - 0 1", &[]),
        ];

        check_cases(cases, Board::validate_non_zero_fullmove_count);
    }

    #[test]
    fn validate() {
        let cases = [
            (
                "kiwipete",
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq -",
                Ok(()),
            ),
            (
                "perft 3",
                "r2qk2r/ppp1b1pp/2n1p3/3pP1n1/3P2b1/2PB1NN1/PP4PP/R1BQK2R w KQkq -",
                Ok(()),
            ),
            (
                "perft 4",
                "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq -",
                Ok(()),
            ),
            (
                "perft 5",
                "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
                Ok(()),
            ),
            (
                "perft 6",
                "rnbqk2r/1p3ppp/p7/1NpPp3/QPP1P1n1/P4N2/4KbPP/R1B2B1R b kq -",
                Ok(()),
            ),
            (
                "en passant",
                "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2",
                Ok(()),
            ),
            (
                "several errors",
                "1P2r3/8/8/8/8/7P/PPPP1PPP/4K1p1 b Kq a3 0 0",
                Err(PositionErrors(vec![
                    InvalidKingCount {
                        color: Black,
                        count: 0,
                    },
                    TooManyPawns {
                        color: White,
                        count: 9,
                    },
                    PawnOnBackRank(sq("g1")),
                    PawnOnBackRank(sq("b8")),
                    InvalidCastlingRights {
                        color: White,
                        side: CastleSide::KingSide,
                    },
                    InvalidCastlingRights {
                        color: Black,
                        side: CastleSide::QueenSide,
                    },
                    SideNotToMoveInCheck(White),
                    InvalidEnPassantSquare {
                        square: sq("a3"),
                        reason: EnPassantError::NoPushedPawn,
                    },
                    ZeroFullmoveCount,
                ])),
            ),
        ];

        for (name, fen, expected) in cases {
            let board: Board = fen.parse().expect("case fen is valid");
            let actual = board.validate();
            assert_eq!(expected, actual, "\n{board}\n\n{name}:",)
        }
    }
}
