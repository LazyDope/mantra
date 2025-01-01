use core::iter::Iterator;
use std::borrow::Cow;

use crossterm::event::{self, Event, KeyCode};
use itertools::Itertools;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use ratatui::{
    layout::{Constraint, Flex, Layout, Margin},
    prelude::*,
    style::{Color, Style},
    widgets::{Block, Cell, Clear, ListState, Paragraph, Row, Table, Tabs},
    Frame,
};
use strum::{EnumCount, VariantNames};
use text::ToText;

use crate::{
    app::{App, AppError},
    storage::TransactionFilter,
};

use super::{Popup, PopupHandler};

/// Popup for viewing and editing filters
pub struct FilterResults {
    filters: Vec<TransactionFilter>,
    list_state: ListState,
}

/// Popup that goes over the filter results for adding new filters
pub struct AddFilter {
    pop_under: FilterResults,
    filter: TransactionFilter,
    selected_field: AddFilterField,
    selected_type: AddFilterType,
}

#[derive(Default, PartialEq, Eq, FromPrimitive, EnumCount, Clone, Copy)]
enum AddFilterField {
    #[default]
    Type = 0,
    Value,
    Submit,
}

#[derive(Clone, Copy, VariantNames)]
enum AddFilterType {
    Type,
    DateRange,
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

impl AddFilter {
    pub fn new(pop_under: FilterResults) -> Self {
        Self {
            pop_under,
            filter: TransactionFilter::Type(Default::default()),
            selected_field: AddFilterField::Type,
            selected_type: AddFilterType::Type,
        }
    }

    pub fn new_with_entry(pop_under: FilterResults, filter: TransactionFilter) -> Self {
        Self {
            pop_under,
            filter,
            selected_field: AddFilterField::Type,
            selected_type: AddFilterType::Type,
        }
    }
}

impl AddFilterField {
    /// Switch the selected field to the next one
    fn next(&mut self) {
        *self =
            FromPrimitive::from_i8((*self as i8 + 1).rem_euclid(<Self as EnumCount>::COUNT as i8))
                .expect("Will always be a valid isize unless AddFilterField became an empty enum")
    }

    /// Switch the selected field to the previous one
    fn prev(&mut self) {
        *self =
            FromPrimitive::from_i8((*self as i8 - 1).rem_euclid(<Self as EnumCount>::COUNT as i8))
                .expect("Will always be a valid isize unless AddFilterField became an empty enum")
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
                            let index = index.clamp(0, self.filters.len() - 1);
                            self.filters.remove(index);
                        }
                    }
                    KeyCode::Char('a') => return Ok(Some(Popup::AddFilter(AddFilter::new(self)))),
                    KeyCode::Char('e') => {
                        if let Some(index) = self.list_state.selected() {
                            let index = index.clamp(0, self.filters.len() - 1);
                            let entry = self.filters.swap_remove(index);

                            return Ok(Some(Popup::AddFilter(AddFilter::new_with_entry(
                                self, entry,
                            ))));
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

impl PopupHandler for AddFilter {
    async fn handle_event(
        mut self,
        app: &mut App,
        event: &Event,
    ) -> Result<Option<Popup>, AppError> {
        if let Event::Key(key) = event {
            if key.kind == event::KeyEventKind::Press {
                match key.code {
                    KeyCode::Up => {
                        self.selected_field.prev();
                    }
                    KeyCode::Down => {
                        self.selected_field.next();
                    }
                    KeyCode::Esc => {
                        return Ok(Some(Popup::FilterResults(self.pop_under)));
                    }
                    _ => (),
                }
            }
        }
        Ok(Some(Popup::AddFilter(self)))
    }

    fn render_to_frame(&mut self, area: Rect, frame: &mut Frame)
    where
        Self: Sized,
    {
        let Self {
            pop_under,
            selected_field,
            selected_type,
            ..
        } = self;

        pop_under.render_to_frame(area, frame);

        const BOX_HEIGHT: u16 = 1;
        const BORDER_SIZE: u16 = 1;
        const SUBMIT_TEXT: &str = "Submit";

        let [area] = Layout::vertical([Constraint::Length(3 * BOX_HEIGHT + 8 * BORDER_SIZE)])
            .flex(Flex::Center)
            .areas(area);
        let [area] = Layout::horizontal([Constraint::Percentage(30)])
            .flex(Flex::Center)
            .areas(area);
        let block = Block::bordered().title("Add Filter");
        frame.render_widget(Clear, area);
        frame.render_widget(block, area);

        let area = area.inner(Margin::new(BORDER_SIZE, BORDER_SIZE));
        let [type_area, values_area, submit_area] = Layout::vertical([
            Constraint::Length(BOX_HEIGHT + BORDER_SIZE * 2),
            Constraint::Length(BOX_HEIGHT + BORDER_SIZE * 2),
            Constraint::Length(BOX_HEIGHT + BORDER_SIZE * 2),
        ])
        .areas(area);

        let mut type_field = Block::bordered().title("Type");
        let mut values_field = Block::bordered().title("Values");
        let mut submit_field = Block::bordered();

        let active_style = Style::default().bg(Color::LightYellow).fg(Color::Black);

        {
            use AddFilterField::*;
            match selected_field {
                Submit => submit_field = submit_field.style(active_style),
                Type => type_field = type_field.style(active_style),
                Value => values_field = values_field.style(active_style),
            };
        }

        let type_text = Tabs::new(<AddFilterType as VariantNames>::VARIANTS.iter().copied())
            .select(*selected_type as usize)
            .block(type_field);
        let values_text = Paragraph::new("").block(values_field);
        let submit_text = Paragraph::new(SUBMIT_TEXT)
            .block(submit_field)
            .alignment(Alignment::Center);

        frame.render_widget(type_text, type_area);
        frame.render_widget(values_text, values_area);
        frame.render_widget(
            submit_text,
            Layout::horizontal([Constraint::Length(
                SUBMIT_TEXT.len() as u16 + BORDER_SIZE * 2,
            )])
            .flex(Flex::Center)
            .areas::<1>(submit_area)[0],
        )
    }
}

fn filters_as_rows(filters: &[TransactionFilter]) -> impl Iterator<Item = Row> {
    filters.iter().map(|filter| match filter {
        TransactionFilter::UserId(ids) => Row::new([
            Cell::new("user id must be"),
            Cell::new(Line::from_iter(Itertools::intersperse(
                ids.iter().map(|v| Cow::from(v.to_string())),
                Cow::from(" or "),
            ))),
        ]),
        TransactionFilter::Type(transaction_types) => Row::new([
            Cell::new("transaction type must be"),
            Cell::new(Line::from_iter(Itertools::intersperse(
                transaction_types
                    .kv_pairs()
                    .filter(|&(_, selected)| *selected)
                    .map(|(t_type, _)| Cow::from(t_type.to_string())),
                Cow::from(" or "),
            ))),
        ]),
        TransactionFilter::DateRange(date_range) => {
            Row::new([Cell::new("must be within"), Cell::new(date_range.to_text())])
        }
        TransactionFilter::Id(ids) => Row::new([
            Cell::new("transaction id must be"),
            Cell::new(Line::from_iter(Itertools::intersperse(
                ids.iter().map(|v| Cow::from(v.to_string())),
                Cow::from(" or "),
            ))),
        ]),
        TransactionFilter::Not(_) => todo!(),
    })
}
