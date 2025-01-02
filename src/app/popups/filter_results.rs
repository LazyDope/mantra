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
use text::{ToSpan, ToText};

use crate::{
    app::{App, AppError},
    storage::{TransactionFilter, TransactionType},
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
    index: usize,
}

#[derive(Default, PartialEq, Eq, FromPrimitive, EnumCount, Clone, Copy)]
#[repr(u8)]
enum AddFilterField {
    #[default]
    Type = 0,
    Value,
    Submit,
}

#[derive(Clone, Copy, VariantNames, FromPrimitive, EnumCount)]
#[repr(u8)]
enum AddFilterType {
    TransactionType = 0,
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
        Self::new_with_entry(pop_under, TransactionFilter::Type(Default::default()))
    }

    pub fn new_with_entry(pop_under: FilterResults, filter: TransactionFilter) -> Self {
        Self {
            pop_under,
            filter,
            selected_field: AddFilterField::Type,
            selected_type: AddFilterType::TransactionType,
            index: 0,
        }
    }
}

impl AddFilterField {
    /// Switch the selected field to the next one
    fn next(&mut self) {
        *self =
            FromPrimitive::from_i8((*self as i8 + 1).rem_euclid(<Self as EnumCount>::COUNT as i8))
                .expect("Will always be a valid i8 unless AddFilterField became an empty enum")
    }

    /// Switch the selected field to the previous one
    fn prev(&mut self) {
        *self =
            FromPrimitive::from_i8((*self as i8 - 1).rem_euclid(<Self as EnumCount>::COUNT as i8))
                .expect("Will always be a valid i8 unless AddFilterField became an empty enum")
    }
}

impl AddFilterType {
    /// Switch the selected field to the next one
    fn next(&mut self) {
        *self =
            FromPrimitive::from_i8((*self as i8 + 1).rem_euclid(<Self as EnumCount>::COUNT as i8))
                .expect("Will always be a valid i8 unless AddFilterType became an empty enum")
    }

    /// Switch the selected field to the previous one
    fn prev(&mut self) {
        *self =
            FromPrimitive::from_i8((*self as i8 - 1).rem_euclid(<Self as EnumCount>::COUNT as i8))
                .expect("Will always be a valid i8 unless AddFilterType became an empty enum")
    }

    /// How many possibilities available for the value selector
    fn value_count(&self) -> usize {
        match self {
            AddFilterType::TransactionType => TransactionType::COUNT,
            AddFilterType::DateRange => 2,
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
                    KeyCode::Left => match self.selected_field {
                        AddFilterField::Type => {
                            self.selected_type.prev();
                            self.filter = self.selected_type.into()
                        }
                        AddFilterField::Value => {
                            self.index = (self.index as isize - 1)
                                .rem_euclid(self.selected_type.value_count() as isize)
                                as usize
                        }
                        AddFilterField::Submit => (),
                    },
                    KeyCode::Right => match self.selected_field {
                        AddFilterField::Type => {
                            self.selected_type.next();
                            self.filter = self.selected_type.into()
                        }
                        AddFilterField::Value => {
                            self.index =
                                (self.index + 1).rem_euclid(self.selected_type.value_count())
                        }
                        AddFilterField::Submit => (),
                    },
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
            filter,
            index,
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
        let values_text = display_filter_values(filter, *index).block(values_field);
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

impl From<AddFilterType> for TransactionFilter {
    fn from(value: AddFilterType) -> Self {
        match value {
            AddFilterType::TransactionType => TransactionFilter::Type(Default::default()),
            AddFilterType::DateRange => TransactionFilter::DateRange((..).into()),
        }
    }
}

fn filters_as_rows(filters: &[TransactionFilter]) -> impl Iterator<Item = Row> {
    filters
        .iter()
        .map(|filter| Row::new(filter_as_cells(filter).into_iter().map(Cell::from)))
}

fn filter_as_cells(filter: &TransactionFilter) -> [String; 2] {
    match filter {
        TransactionFilter::UserId(ids) => [
            String::from("user id must be"),
            Itertools::intersperse(
                ids.iter().map(|v| Cow::from(v.to_string())),
                Cow::from(" or "),
            )
            .collect(),
        ],
        TransactionFilter::Type(transaction_types) => [
            String::from("transaction type must be"),
            Itertools::intersperse(
                transaction_types
                    .kv_pairs()
                    .filter(|&(_, selected)| *selected)
                    .map(|(t_type, _)| Cow::from(t_type.to_string())),
                Cow::from(" or "),
            )
            .collect(),
        ],
        TransactionFilter::DateRange(date_range) => {
            [String::from("date must be within"), date_range.to_string()]
        }
        TransactionFilter::Id(ids) => [
            String::from("transaction id must be"),
            Itertools::intersperse(
                ids.iter().map(|v| Cow::from(v.to_string())),
                Cow::from(" or "),
            )
            .collect(),
        ],
        TransactionFilter::Not(filter) => {
            let mut cells = filter_as_cells(filter);
            cells[0] = cells[0].replace("must ", "must not ");
            cells
        }
    }
}

fn display_filter_values(filter: &TransactionFilter, index: usize) -> Paragraph {
    match filter {
        TransactionFilter::Type(transaction_types) => {
            Paragraph::new(Line::from_iter(Itertools::intersperse(
                transaction_types
                    .kv_pairs()
                    .enumerate()
                    .map(|(i, (t_type, selected))| {
                        let text = Span::from(t_type.to_string());
                        if i == index {
                            text.style(Style::default().fg(Color::Black).bg(Color::LightYellow))
                        } else if *selected {
                            text.style(Style::default().fg(Color::Black).bg(Color::White))
                        } else {
                            text
                        }
                    }),
                Span::from(", "),
            )))
        }
        TransactionFilter::DateRange(date_range) => Paragraph::new(date_range.to_string()),
        TransactionFilter::Not(filter) => display_filter_values(filter, index),
        _ => Paragraph::new(""),
    }
}
