//! Engine versions: every [`Searcher`] that has been released, kept runnable forever.
//!
//! Each version lives in its own private module (`v0.rs`, then `v1.rs`, …) and is
//! reached only through its entry in [`VERSIONS`], selected by id at runtime
//! (`engine --engine <id>`). That lets any two versions play each other, so every
//! improvement can be measured against the version before it and logged in `ENGINES.md`.
//!
//! Rules for version modules:
//!
//! - A version depends on core (board, movegen, uci types, rng) and on this module's
//!   [`Searcher`] and [`SearchRequest`], **never on another version**. A new version starts
//!   as a copy of the previous one.
//! - A released version is frozen: it changes only when a core API change forces it.
//! - Strength code (evaluation, search, move ordering, time allocation) lives in versions;
//!   rules and protocol plumbing live in core.
//!
//! [`VERSIONS`] is the single source of truth for ids, `--list-engines`, the default
//! ([`newest`]) and the tests. The contract tests in this module run every registered
//! version, so a new version is tested as soon as it is added.

use crate::{
    board::Board,
    rng::Rng,
    types::Move,
    uci::{EngineMessage, GoParams},
};
use thiserror::Error;

mod v0;

/// Error from [`find`]: no registered version has this id.
#[derive(Error, Clone, Eq, PartialEq, Debug)]
pub enum UnknownVersion {
    /// The id that was looked up.
    #[error("unknown engine version {0:?}")]
    Id(String),
}

/// Everything a [`Searcher`] gets for one `go` command.
///
/// New inputs (position history, a stop flag, time limits) become new fields here, so adding
/// one never changes the signature of [`Searcher::search`] in every version.
#[derive(Clone, Copy, Debug)]
pub struct SearchRequest<'a> {
    /// The position to search. It always passes [`Board::validate`].
    pub board: &'a Board,
    /// The `go` command's parameters (clocks, depth, …). Versions may ignore them.
    pub params: &'a GoParams,
}

/// The move-choosing part of an engine version. [`Engine`](crate::engine::Engine) holds one
/// as a `Box<dyn Searcher>` and handles the UCI protocol around it.
pub trait Searcher {
    /// Chooses a move in `req.board`.
    ///
    /// Returns `None` only when there are no legal moves (checkmate if the side to move is
    /// [in check](Board::is_in_check), stalemate otherwise); the engine then sends
    /// `bestmove 0000`.
    ///
    /// Progress goes through `report` (for example `info string` lines). A searcher never
    /// sends [`EngineMessage::BestMove`] itself: the return value is the move, and the engine
    /// turns it into `bestmove`.
    fn search(
        &mut self,
        req: SearchRequest<'_>,
        report: &mut dyn FnMut(EngineMessage),
    ) -> Option<Move>;

    /// Called on `ucinewgame`. Override it to clear state that belongs to one game, such as
    /// a transposition table. The default does nothing.
    fn new_game(&mut self) {}
}

/// One entry in [`VERSIONS`]: a version's id, its description and how to build it.
pub struct Version {
    /// Short, unique id: `v<N>` with an optional `-<word>` (`v0`, `v1-material`), where `N`
    /// is the entry's index in [`VERSIONS`]. Sent as `id name Ferrosian <id>`.
    pub id: &'static str,
    /// Sent as `id author`.
    pub author: &'static str,
    /// One line on what this version does or changed, shown by `--list-engines`.
    pub summary: &'static str,
    /// Builds a fresh searcher that takes its randomness from the given [`Rng`].
    pub build: fn(Rng) -> Box<dyn Searcher>,
}

/// Every released version, oldest first. The last entry is the default ([`newest`]).
pub static VERSIONS: &[Version] = &[Version {
    id: "v0",
    author: "Jake",
    summary: "makes a uniform random legal move",
    build: v0::build,
}];

/// The newest version, used when `--engine` is not given.
pub fn newest() -> &'static Version {
    VERSIONS.last().expect("always at least one version")
}

/// Looks up a version by its exact, case-sensitive id.
///
/// # Errors
///
/// [`UnknownVersion`] if no entry has that id.
pub fn find(id: &str) -> Result<&'static Version, UnknownVersion> {
    if let Some(version) = VERSIONS.iter().find(|v| v.id == id) {
        Ok(version)
    } else {
        Err(UnknownVersion::Id(id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        board::Board,
        movegen::MoveList,
        rng::Rng,
        test_helpers::{
            KIWIPETE, PERFT3, PERFT4, PERFT5, PERFT6, STARTING_FEN, TEST_SEED, from_fen,
        },
        uci::{EngineMessage, GoParams},
    };
    use std::collections::HashSet;

    // Positions where every version must return a legal move.
    const HAS_MOVES: &[&str] = &[STARTING_FEN, KIWIPETE, PERFT3, PERFT4, PERFT5, PERFT6];

    /// Positions with no legal moves, where every version must return `None`.
    const NO_MOVES: &[(&str, &str)] = &[
        (
            "checkmate with white to move",
            "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3",
        ),
        (
            "stalemate with black to move",
            "7k/5Q2/6K1/8/8/8/8/8 b - - 0 1",
        ),
    ];

    /// Searches per position, so a version with randomness gets several draws.
    const SEARCHES_PER_POSITION: usize = 10;

    /// Runs one search with default `go` params, returning the move and every reported message.
    fn search_once(
        searcher: &mut dyn Searcher,
        board: &Board,
    ) -> (Option<Move>, Vec<EngineMessage>) {
        let params = GoParams::default();
        let mut reported = Vec::new();
        let best = searcher.search(
            SearchRequest {
                board,
                params: &params,
            },
            &mut |msg| reported.push(msg),
        );
        (best, reported)
    }

    #[track_caller]
    fn assert_no_bestmove(id: &str, fen: &str, reported: &[EngineMessage]) {
        let sent: Vec<&EngineMessage> = reported
            .iter()
            .filter(|msg| matches!(msg, EngineMessage::BestMove { .. }))
            .collect();
        assert_eq!(
            Vec::<&EngineMessage>::new(),
            sent,
            "{id} on {fen}: searcher sent bestmove itself"
        );
    }

    #[test]
    fn registry_not_empty() {
        assert!(!VERSIONS.is_empty(), "VERSIONS is empty")
    }

    #[test]
    fn ids_unique() {
        let mut seen = HashSet::new();
        let duplicates: Vec<&str> = VERSIONS
            .iter()
            .filter_map(|v| (!seen.insert(v.id)).then_some(v.id))
            .collect();
        assert_eq!(Vec::<&str>::new(), duplicates, "duplicate ids in VERSIONS");
    }

    #[test]
    fn ids_follow_format_in_order() {
        for (i, version) in VERSIONS.iter().enumerate() {
            let id = version.id;
            let number = format!("v{i}");
            let rest = id
                .strip_prefix(&number)
                .unwrap_or_else(|| panic!("entry {i}: id {id:?} should start with {number:?}"));
            assert!(
                rest.is_empty() || (rest.len() > 1 && rest.starts_with('-')),
                "entry {i}: id {id:?} should be {number:?} or {number:?}-<word>"
            );
            assert!(
                id.len() <= 16,
                "entry {i}: id {id:?} is longer than 16 characters"
            );
            assert!(
                id.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
                "entry {i}: id {id:?} may only use a-z, 0-9 and '-'"
            );
        }
    }

    #[test]
    fn find_case_sensitive() {
        let cases = [
            ("v0", Ok("v0")),
            ("V0", Err(UnknownVersion::Id("V0".to_string()))),
            ("", Err(UnknownVersion::Id("".to_string()))),
            ("v99", Err(UnknownVersion::Id("v99".to_string()))),
            ("v0-", Err(UnknownVersion::Id("v0-".to_string()))),
        ];
        for (id, expected) in cases {
            assert_eq!(expected, find(id).map(|v| v.id), "{id:?}");
        }
    }

    #[test]
    fn every_version_plays_legal_moves() {
        for version in VERSIONS {
            let id = version.id;
            let mut searcher = (version.build)(Rng::new(TEST_SEED));
            for fen in HAS_MOVES {
                let board = from_fen(fen);
                let mut legal = MoveList::new();
                board.legal_moves(&mut legal);
                for search in 0..SEARCHES_PER_POSITION {
                    let (best, reported) = search_once(searcher.as_mut(), &board);
                    let mv =
                        best.unwrap_or_else(|| panic!("{id} on {fen}: no move (search {search})"));
                    assert!(
                        legal.contains(&mv),
                        "{id} on {fen}: illegal move {mv} (search {search})"
                    );
                    assert_no_bestmove(id, fen, &reported);
                }
                searcher.new_game();
            }
        }
    }

    #[test]
    fn every_version_returns_none_without_moves() {
        for version in VERSIONS {
            let id = version.id;
            let mut searcher = (version.build)(Rng::new(TEST_SEED));
            for (why, fen) in NO_MOVES {
                let (best, reported) = search_once(searcher.as_mut(), &from_fen(fen));
                assert_eq!(None, best, "{id}: {why}");
                assert_no_bestmove(id, fen, &reported);
            }
        }
    }
}
