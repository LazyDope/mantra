use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    layout::{Constraint, Flex, Layout, Margin},
    prelude::*,
    style::{Color, Style},
    widgets::{Block, Clear, ListState, Paragraph, Tabs},
    Frame,
};

use crate::app::{App, AppError};

use super::{Popup, PopupHandler};

/// Popup for confirming new user creation
pub struct FilterResults {
    list_state: ListState,
}

impl FilterResults {
    /// Create a popup that lists the current filters applied to the transaction table.
    /// Also provides controls for adding new filters and .
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
        }
    }
}

impl PopupHandler for FilterResults {
    async fn process_event(
        mut self,
        app: &mut App,
        event: &Event,
    ) -> Result<Option<Popup>, AppError> {
        if let Event::Key(key) = event {
            if key.kind == event::KeyEventKind::Press {
                match key.code {
                    KeyCode::Up => {
                        self.list_state.select_previous();
                    }
                    KeyCode::Down => {
                        self.list_state.select_next();
                    }
                    KeyCode::Esc => return Ok(None),
                    KeyCode::Char('d') => {
                        if let Some(index) = self.list_state.selected() {
                            let index = index.clamp(0, app.data.transaction_filters.len() - 1);
                            app.data.transaction_filters.remove(index);
                        }
                    }
                    _ => (),
                }
            }
        }
        Ok(Some(Popup::FilterResults(self)))
    }

    fn render_to_frame(&mut self, area: Rect, frame: &mut Frame) {
        const TEXT_HEIGHT: u16 = 1;
        const BORDER_SIZE: u16 = 1;

        let [area] = Layout::vertical([Constraint::Length(TEXT_HEIGHT + 4 * BORDER_SIZE)])
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
            Layout::vertical([Constraint::Length(TEXT_HEIGHT + BORDER_SIZE * 2)]).areas(area);

        let text_field =
            Block::bordered().style(Style::default().bg(Color::LightYellow).fg(Color::Black));

        let text_text = Paragraph::new("Not Implemented").block(text_field);

        frame.render_widget(text_text, username_area);
    }
}
