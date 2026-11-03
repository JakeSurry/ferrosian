//! Sliding attacks by walking each ray one square at a time.
//!
//! Slow but correct. It is not used for lookups: [`super::magic`] uses it to build its
//! tables, and the magic tests compare every lookup against it.

use crate::types::{Bitboard, Square};

/// A one-square shift in one direction. Off-board bits are dropped, so a ray ends when the
/// result is empty.
pub(super) type Step = fn(Bitboard) -> Bitboard;

/// The four orthogonal directions: north, south, east, west.
pub(super) const ROOK_STEPS: [Step; 4] = [
    Bitboard::north,
    Bitboard::south,
    Bitboard::east,
    Bitboard::west,
];

/// The four diagonal directions: north-east, north-west, south-east, south-west.
pub(super) const BISHOP_STEPS: [Step; 4] = [
    Bitboard::north_east,
    Bitboard::north_west,
    Bitboard::south_east,
    Bitboard::south_west,
];

/// Attacks in one direction from `start`: every square up to and including the first blocker,
/// or up to the edge if there is none.
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

/// Attacks from `square` for a slider moving along `steps`: the union of [`ray`] over each
/// direction. `blockers` is the full occupancy.
pub(super) fn slider_rays(square: Square, blockers: Bitboard, steps: &[Step]) -> Bitboard {
    let start = square.bitboard();
    steps.iter().fold(Bitboard::EMPTY, |acc, &step| {
        acc | ray(start, blockers, step)
    })
}
