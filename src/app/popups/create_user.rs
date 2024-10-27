use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    layout::{Constraint, Flex, Layout, Margin},
    prelude::*,
    style::{Color, Style},
    widgets::{Block, Clear, Tabs},
    Frame,
};

use crate::app::{App, AppError, AppMode};

use super::Popup;

pub struct CreateUser {
    new_user: String,
    should_create: bool,
}

impl CreateUser {
    pub fn new(new_user: String) -> Self {
        Self {
            new_user,
            should_create: true,
        }
    }

    pub(crate) async fn process_event(
        mut self,
        app: &mut App,
        event: Event,
    ) -> Result<Option<Popup>, AppError> {
        if let Event::Key(key) = event {
            if key.kind == event::KeyEventKind::Press {
                match key.code {
                    KeyCode::Left | KeyCode::BackTab => {
                        self.should_create = !self.should_create;
                    }
                    KeyCode::Right | KeyCode::Tab => {
                        self.should_create = !self.should_create;
                    }
                    KeyCode::Enter => {
                        if self.should_create {
                            app.data.storage.create_user(&self.new_user).await?;
                            let user = app.data.storage.get_user(&self.new_user).await?;
                            app.data.status_text = format!("Logged in as {}", user.get_name());
                            app.data.current_user = Some(user);
                            app.mode = AppMode::LogTable;
                            app.data.update_table().await?;
                        };
                        return Ok(None);
                    }
                    KeyCode::Esc => return Ok(None),
                    _ => (),
                }
            }
        }
        Ok(Some(Popup::CreateUser(self)))
    }

    pub(crate) fn render_to_frame(&self, area: Rect, frame: &mut Frame) {
        const QUESTION_HEIGHT: u16 = 1;
        const BORDER_SIZE: u16 = 1;

        let [area] = Layout::vertical([Constraint::Length(QUESTION_HEIGHT + 4 * BORDER_SIZE)])
            .flex(Flex::Center)
            .areas(area);
        let [area] = Layout::horizontal([Constraint::Percentage(40)])
            .flex(Flex::Center)
            .areas(area);
        let block = Block::bordered().title("New User");
        frame.render_widget(Clear, area);
        frame.render_widget(block, area);
        let area = area.inner(Margin::new(BORDER_SIZE, BORDER_SIZE));
        let [username_area] =
            Layout::vertical([Constraint::Length(QUESTION_HEIGHT + BORDER_SIZE * 2)]).areas(area);

        let username_field = Block::bordered()
            .title(format!("Create user '{}'?", self.new_user))
            .style(Style::default().bg(Color::LightYellow).fg(Color::Black));

        let username_text = Tabs::new(["No", "Yes"])
            .select(self.should_create as usize)
            .block(username_field);

        frame.render_widget(username_text, username_area);
    }
}
