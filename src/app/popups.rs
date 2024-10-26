use crossterm::event::Event;
use ratatui::prelude::*;

use super::{App, AppError};

mod add_transaction;
pub use add_transaction::*;
mod create_user;
pub use create_user::*;

pub enum Popup {
    AddTransaction(AddTransaction),
    CreateUser(CreateUser),
}

impl Popup {
    pub async fn process_event(
        self,
        app: &mut App,
        event: Event,
    ) -> Result<Option<Self>, AppError> {
        match self {
            Popup::AddTransaction(popup) => popup.process_event(app, event).await,
            Popup::CreateUser(popup) => popup.process_event(app, event).await,
        }
    }

    pub fn render_to_frame(&self, area: Rect, frame: &mut Frame)
    where
        Self: Sized,
    {
        match self {
            Popup::AddTransaction(popup) => popup.render_to_frame(area, frame),
            Popup::CreateUser(popup) => popup.render_to_frame(area, frame),
        }
    }
}
