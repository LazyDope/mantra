use crossterm::event::{self, Event, KeyCode};
use ratatui::{prelude::*, Frame};

use crate::{
    app::{App, AppError},
    storage::StorageRunError,
};

use super::{CreateUser, CursoredString, Popup};

#[derive(Default)]
pub struct UserLogin {
    username: CursoredString,
}

impl UserLogin {
    pub(crate) async fn process_event(
        mut self,
        app: &mut App,
        event: Event,
    ) -> Result<Option<Popup>, AppError> {
        if let Event::Key(key) = event {
            if key.kind == event::KeyEventKind::Press {
                match key.code {
                    KeyCode::Left => {
                        self.username.prev();
                    }
                    KeyCode::Right => {
                        self.username.next();
                    }
                    KeyCode::Enter => {
                        let UserLogin { username } = self;
                        match app.storage.get_user(username.text.to_lowercase()).await {
                            Ok(user) => {
                                app.status_text = format!("Logged in as {}", user.name);
                                app.current_user = Some(user);
                            }
                            Err(StorageRunError::RecordMissing) => {
                                return Ok(Some(Popup::CreateUser(CreateUser::new(username.text))))
                            }
                            Err(e) => return Err(e.into()),
                        }
                        return Ok(None);
                    }
                    KeyCode::Backspace => self.username.remove_behind(),
                    KeyCode::Delete => self.username.remove_ahead(),
                    KeyCode::Insert => self.username.inserting = !self.username.inserting,
                    KeyCode::Esc => return Ok(None),
                    KeyCode::Char(c) => self.username.insert(c),
                    _ => (),
                }
            }
        }
        Ok(Some(Popup::UserLogin(self)))
    }

    pub(crate) fn render_to_frame(&self, area: Rect, frame: &mut Frame) {
        todo!()
    }
}
