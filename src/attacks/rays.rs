use crate::types::{Bitboard, Square};

pub(super) type Step = fn(Bitboard) -> Bitboard;

pub(super) const ROOK_STEPS: [Step; 4] = [
    Bitboard::north,
    Bitboard::south,
    Bitboard::east,
    Bitboard::west,
];
pub(super) const BISHOP_STEPS: [Step; 4] = [
    Bitboard::north_east,
    Bitboard::north_west,
    Bitboard::south_east,
    Bitboard::south_west,
];

fn ray(start: Bitboard, blockers: Bitboard, step: Step) -> Bitboard {
    let mut current = step(start);
    let mut attacks = Bitboard::EMPTY;

    while !current.is_empty() {
        attacks |= current;
        if current.overlaps(blockers) {
            break;
        }
        current = step(current);
    }
    attacks
}

pub(super) fn slider_rays(square: Square, blockers: Bitboard, steps: &[Step]) -> Bitboard {
    let start = square.bitboard();
    steps.iter().fold(Bitboard::EMPTY, |acc, &step| {
        acc | ray(start, blockers, step)
    })
}
