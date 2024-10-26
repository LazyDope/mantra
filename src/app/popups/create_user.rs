use crossterm::event::Event;

use crate::app::{App, AppError};

use super::Popup;

pub struct CreateUser {
    new_user: String,
}

impl CreateUser {
    pub fn new(new_user: String) -> Self {
        Self { new_user }
    }

    pub(crate) async fn process_event(
        mut self,
        app: &mut App,
        event: Event,
    ) -> Result<Option<Popup>, AppError> {
        todo!()
    }
}
