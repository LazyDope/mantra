//! Handler for popups
use crossterm::event::Event;
use enum_dispatch::enum_dispatch;
use ratatui::prelude::*;

use super::{App, AppError};

mod add_transaction;
pub use add_transaction::*;
mod create_user;
pub use create_user::*;
mod filter_results;
pub use filter_results::*;

/// Types of popup that can be displayed
#[enum_dispatch(PopupHandler)]
pub enum Popup {
    AddTransaction,
    CreateUser,
    FilterResults,
    AddFilter,
}

#[enum_dispatch]
pub(crate) trait PopupHandler {
    /// Handles incoming key events and updates log table when submitted
    async fn handle_event(self, app: &mut App, event: &Event) -> Result<Option<Popup>, AppError>;

    /// Handles the rendering of the popup to the given [`Frame`]
    fn render_to_frame(&mut self, area: Rect, frame: &mut Frame)
    where
        Self: Sized;
}
