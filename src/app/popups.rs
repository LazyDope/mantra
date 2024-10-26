use std::fmt::Display;

use crossterm::event::Event;
use ratatui::prelude::*;

use super::{App, AppError};

mod add_transaction;
pub use add_transaction::*;
mod user_login;
pub use user_login::*;
mod create_user;
pub use create_user::*;

pub enum Popup {
    AddTransaction(AddTransaction),
    UserLogin(UserLogin),
    CreateUser(CreateUser),
}

#[derive(Default)]
pub struct CursoredString {
    pub text: String,
    pub index: usize,
    pub inserting: bool,
}

impl Popup {
    pub async fn process_event(
        self,
        app: &mut App,
        event: Event,
    ) -> Result<Option<Self>, AppError> {
        match self {
            Popup::AddTransaction(popup) => Ok(popup
                .process_event(app, event)
                .await?
                .map(Self::AddTransaction)),
            Popup::UserLogin(popup) => Ok(popup.process_event(app, event).await?),
            Popup::CreateUser(popup) => Ok(popup.process_event(app, event).await?),
        }
    }

    pub fn render_to_frame(&self, area: Rect, frame: &mut Frame)
    where
        Self: Sized,
    {
        match self {
            Popup::AddTransaction(popup) => popup.render_to_frame(area, frame),
            Popup::UserLogin(popup) => popup.render_to_frame(area, frame),
            Popup::CreateUser(_) => todo!(),
        }
    }
}

impl CursoredString {
    fn next(&mut self) {
        self.index = self.index.saturating_add(1).clamp(0, self.text.len())
    }

    fn prev(&mut self) {
        self.index = self.index.saturating_sub(1).clamp(0, self.text.len())
    }

    fn remove_behind(&mut self) {
        if self.index > 0 {
            let old_len = self.text.len();
            let mut index = 0;
            self.text.retain(|_| {
                index += 1;
                index != self.index
            });
            if self.text.len() < old_len {
                self.index -= 1;
            };
        }
    }

    fn remove_ahead(&mut self) {
        if self.index < self.text.chars().count() {
            let mut index = 0;
            self.text.retain(|_| {
                index += 1;
                if index - 1 == self.index {
                    return false;
                }
                true
            })
        }
    }

    fn insert(&mut self, value: char) {
        if self.inserting {
            self.remove_ahead();
        }
        let byte_index = self
            .text
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.index)
            .unwrap_or(self.text.len());

        self.text.insert(byte_index, value);
        self.index += 1
    }
}

impl Display for CursoredString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.text.fmt(f)
    }
}
