//! Handler for popups
use crossterm::event::Event;
use enum_dispatch::enum_dispatch;
use ratatui::prelude::*;

use super::{App, AppError};

mod add_transaction;
pub use add_transaction::*;
mod create_user;
pub use create_user::*;

/// Types of popup that can be displayed
#[enum_dispatch(PopupHandler)]
pub enum Popup {
    AddTransaction,
    CreateUser,
}

#[enum_dispatch]
pub(crate) trait PopupHandler {
    /// Handles incoming key events and updates log table when submitted
    async fn process_event(self, app: &mut App, event: &Event) -> Result<Option<Popup>, AppError>;

    /// Handles the rendering of the popup to the given [`Frame`]
    fn render_to_frame<'a>(&self, area: Rect, frame: &mut Frame<'a>)
    where
        Self: Sized;
}
