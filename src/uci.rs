//! The UCI protocol as types: [`GuiCommand`] (GUI to engine), [`EngineMessage`] (engine to
//! GUI), and [`UciMove`], the move syntax both use.
//!
//! Each type parses with `FromStr` and prints with `Display`, so the engine and a UCI client
//! share them in opposite directions. Parsing is lenient about whitespace and printing is
//! canonical. Parsing checks syntax only: [`UciMove::resolve`] turns a move into a legal
//! [`Move`](crate::types::Move) on a given board.

mod command;
mod message;
mod uci_move;

pub use command::{
    CommandError, GoParams, GuiCommand, PositionBase, PositionCommand, PositionCommandError,
};
pub use message::{EngineMessage, MessageError};
pub use uci_move::{UciMove, UciMoveError};
