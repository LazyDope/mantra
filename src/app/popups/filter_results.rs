use core::iter::Iterator;

use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    layout::{Constraint, Flex, Layout, Margin},
    prelude::*,
    style::{Color, Style},
    widgets::{Block, Cell, Clear, ListState, Paragraph, Row, Table},
    Frame,
};
use text::ToText;

use crate::{
    app::{App, AppError},
    storage::TransactionFilter,
};

use super::{Popup, PopupHandler};

/// Popup for confirming new user creation
pub struct FilterResults {
    filters: Vec<TransactionFilter>,
    list_state: ListState,
}

impl FilterResults {
    /// Create a popup that lists the current filters applied to the transaction table.
    /// Also provides controls for adding new filters and .
    pub fn new(filters: Vec<TransactionFilter>) -> Self {
        Self {
            filters,
            list_state: Default::default(),
        }
    }
}

impl PopupHandler for FilterResults {
    async fn handle_event(
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
                    KeyCode::Esc => {
                        app.data.transaction_filters = self.filters;
                        return Ok(None);
                    }
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
        const LIST_HEIGHT: u16 = 7;
        const BORDER_SIZE: u16 = 1;

        let [area] = Layout::vertical([Constraint::Length(LIST_HEIGHT + 4 * BORDER_SIZE)])
            .flex(Flex::Center)
            .areas(area);
        let [area] = Layout::horizontal([Constraint::Percentage(40)])
            .flex(Flex::Center)
            .areas(area);
        let block = Block::bordered().title("Filter Transactions");
        frame.render_widget(Clear, area);
        frame.render_widget(block, area);
        let area = area.inner(Margin::new(BORDER_SIZE, BORDER_SIZE));
        let [table_area] =
            Layout::vertical([Constraint::Length(LIST_HEIGHT + BORDER_SIZE * 2)]).areas(area);

        let table_block =
            Block::bordered().style(Style::default().bg(Color::LightYellow).fg(Color::Black));

        let filter_table = Table::new(
            filters_as_rows(&self.filters),
            [Constraint::Percentage(70), Constraint::Fill(1)],
        )
        .block(table_block);

        frame.render_widget(filter_table, table_area);
    }
}

fn filters_as_rows(filters: &[TransactionFilter]) -> impl Iterator<Item = Row> {
    filters.iter().map(|filter| match filter {
        TransactionFilter::UserId(id) => {
            Row::new([Cell::new("user id must be"), Cell::new(id.to_text())])
        }
        TransactionFilter::Type(transaction_type) => Row::new([
            Cell::new("transaction type must be"),
            Cell::new(transaction_type.to_text()),
        ]),
        TransactionFilter::DateRange(date_range) => {
            Row::new([Cell::new("must be within"), Cell::new(date_range.to_text())])
        }
        TransactionFilter::Id(id) => {
            Row::new([Cell::new("transaction id must be"), Cell::new(id.to_text())])
        }
        TransactionFilter::And(filters) => todo!(),
        TransactionFilter::Or(_) => todo!(),
        TransactionFilter::Not(_) => todo!(),
    })
}
