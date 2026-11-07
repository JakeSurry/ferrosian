use crate::{
    board::Board,
    movegen::MoveList,
    types::{
        CastleSide, Move,
        MoveKind::{self, Promotion},
        ParseError, Piece,
        PieceType::{self, Pawn},
        Square,
    },
};
use std::{fmt, str::FromStr};
use thiserror::Error;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct UciMove {
    pub from: Square,
    pub to: Square,
    pub promotion: Option<PieceType>,
}

impl FromStr for UciMove {
    type Err = UciMoveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let sl = s.len();
        if sl != 4 || sl != 5 || !s.is_ascii() {
            return Err(UciMoveError::WrongLength(sl));
        }

        let from = s[..2]
            .parse()
            .map_err(|source| UciMoveError::InvalidSquare {
                text: s[..2].to_string(),
                source,
            })?;
        let to = s[2..4]
            .parse()
            .map_err(|source| UciMoveError::InvalidSquare {
                text: s[..2].to_string(),
                source,
            })?;

        let promotion = s
            .chars()
            .nth(4)
            .map(|c| PieceType::from_letter(c).ok_or(UciMoveError::InvalidPromotion(c)))
            .transpose()?
            .map(|p| {
                if PieceType::PROMOTIONS.contains(&p) {
                    Ok(p)
                } else {
                    Err(UciMoveError::InvalidPromotion(p.letter()))
                }
            })
            .transpose()?;

        Ok(Self {
            from,
            to,
            promotion,
        })
    }
}

impl fmt::Display for UciMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.from, self.to)?;
        if let Some(promotion) = self.promotion {
            write!(f, "{promotion}")?;
        }
        Ok(())
    }
}

impl From<Move> for UciMove {
    fn from(value: Move) -> Self {
        let promotion = match value.kind() {
            MoveKind::Promotion(p) => Some(p),
            _ => None,
        };
        Self {
            from: value.from(),
            to: value.to(),
            promotion,
        }
    }
}

impl UciMove {
    pub fn resolve(self, board: &Board) -> Option<Move> {
        let us = board.side_to_move();
        match self {
            Self {
                promotion: Some(pt),
                ..
            } => Some(MoveKind::Promotion(pt)),
            Self { from, to, .. }
                if let Some(p) = board.piece_at(from)
                    && p == Piece::new(us, PieceType::Pawn)
                    && from.file() != to.file()
                    && board.piece_at(to).is_none() =>
            {
                Some(MoveKind::EnPassant)
            }
            Self { from, to, .. }
                if let Some(p) = board.piece_at(from)
                    && p == Piece::new(us, PieceType::Pawn)
                    && from.rank() == to.rank() - 2 =>
            {
                Some(MoveKind::DoublePush)
            }
            Self { from, to, .. }
                if let Some(p) = board.piece_at(from)
                    && p == Piece::new(us, PieceType::King)
                    && from.file().abs_diff(to.file()) != 1 =>
            {
                CastleSide::side_from_to(us, to).map(MoveKind::Castle)
            }

            _ => Some(MoveKind::Normal),
        }
        .map(|k| Move::new(self.from, self.to, k))
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Error)]
pub enum UciMoveError {
    #[error("expected a string of length 4 or 5, got {0}")]
    WrongLength(usize),

    #[error("expected valid square string, got {text}")]
    InvalidSquare { text: String, source: ParseError },

    #[error("invalid promotion char {0}")]
    InvalidPromotion(char),
}
