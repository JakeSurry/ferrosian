use crate::{
    board::Board,
    rng::Rng,
    types::Move,
    uci::{EngineMessage, GoParams},
};
use thiserror::Error;

mod v0;

#[derive(Error, Clone, Eq, PartialEq, Debug)]
pub enum UnknownVersion {
    #[error("unknown engine version {0:?}")]
    Id(String),
}

pub struct SearchRequest<'a> {
    pub board: &'a Board,
    pub params: &'a GoParams,
}

/// Assumes a validated position.
/// Returns `None` only when there are no legal moves.
/// Never sends [`BestMove`](`EngineMessage`).
pub trait Searcher {
    fn search(
        &mut self,
        req: SearchRequest<'_>,
        report: &mut dyn FnMut(EngineMessage),
    ) -> Option<Move>;

    fn new_game(&mut self) {}
}

pub struct Version {
    pub id: &'static str,
    pub author: &'static str,
    pub summary: &'static str,
    pub build: fn(Rng) -> Box<dyn Searcher>,
}

pub static VERSIONS: &[Version] = &[Version {
    id: "v0",
    author: "Jake",
    summary: "makes a uniform random legal move",
    build: v0::build,
}];

pub fn newest() -> &'static Version {
    VERSIONS.last().expect("always at least one version")
}

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
        assert_eq!(0, duplicates.len(), "duplicates: {duplicates:?}");
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
        }
    }

    #[test]
    fn find_case_sensitive() {
        let cases = [
            ("v0", Ok("v0")),
            ("V0", Err(UnknownVersion::Id("V0".to_string()))),
            ("", Err(UnknownVersion::Id("".to_string()))),
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
