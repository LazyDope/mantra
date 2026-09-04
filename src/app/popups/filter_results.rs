use core::iter::Iterator;
use std::{borrow::Cow, marker::PhantomData};

use crossterm::event::{self, Event, KeyCode};
use itertools::Itertools;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use ratatui::{
    layout::{Constraint, Flex, Layout, Margin},
    prelude::*,
    style::{Color, Style},
    widgets::{Block, Cell, Clear, Paragraph, Row, Table, TableState, Tabs},
    Frame,
};
use strum::{EnumCount, VariantNames};

use crate::{
    app::{App, AppError},
    storage::{MissingVariant, TransactionFilter, TransactionType},
};

use super::{Popup, PopupHandler};

/// Popup for viewing and editing filters
pub struct FilterResults {
    filters: Vec<TransactionFilter>,
    table_state: TableState,
}

/// Popup that goes over the filter results for adding new filters
pub struct AddFilter {
    pop_under: FilterResults,
    filter: TransactionFilter,
    selected_field: AddFilterField,
    selected_type: AddFilterType,
    // Used to index which position in the values is currently selected
    index: i16,
}

#[derive(Default, PartialEq, Eq, FromPrimitive, EnumCount, Clone, Copy)]
#[repr(u8)]
enum AddFilterField {
    #[default]
    Type = 0,
    Value,
    Submit,
}

#[derive(Clone, Copy, VariantNames, FromPrimitive, EnumCount, Debug)]
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
            table_state: Default::default(),
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
            selected_type: AddFilterType::try_from(&filter).expect(
                "The Filter Type we get from editing a custom filter should always be valid",
            ),
            filter,
            selected_field: AddFilterField::Type,
            index: 0,
        }
    }

    pub fn next_index(&mut self) {
        self.index = (self.index + 1).rem_euclid(self.selected_type.value_count())
    }

    pub fn prev_index(&mut self) {
        self.index = (self.index - 1).rem_euclid(self.selected_type.value_count())
    }

    pub fn clamp_index(&mut self) {
        self.index = self.index.clamp(0, self.selected_type.value_count() - 1)
    }

    pub fn next_field(&mut self) {
        self.selected_field.next();
        self.clamp_index();
    }

    pub fn prev_field(&mut self) {
        self.selected_field.prev();
        self.clamp_index();
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
    fn next(&mut self) -> TransactionFilter {
        *self =
            FromPrimitive::from_i8((*self as i8 + 1).rem_euclid(<Self as EnumCount>::COUNT as i8))
                .expect("Will always be a valid i8 unless AddFilterType became an empty enum");

        (*self).into()
    }

    /// Switch the selected field to the previous one
    fn prev(&mut self) -> TransactionFilter {
        *self =
            FromPrimitive::from_i8((*self as i8 - 1).rem_euclid(<Self as EnumCount>::COUNT as i8))
                .expect("Will always be a valid i8 unless AddFilterType became an empty enum");
        (*self).into()
    }

    /// How many possibilities available for the value selector
    fn value_count(&self) -> i16 {
        match self {
            AddFilterType::TransactionType => TransactionType::COUNT
                .try_into()
                .expect("There should never be more variants of TransactionType than fit into i16"),
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
                        self.table_state.select_previous();
                    }
                    KeyCode::Down => {
                        self.table_state.select_next();
                    }
                    KeyCode::Esc => {
                        app.data.transaction_filters = self.filters;
                        app.data.update_table().await?;
                        return Ok(None);
                    }
                    KeyCode::Char('d') => {
                        if let Some(index) = self.table_state.selected() {
                            let index = index.clamp(0, self.filters.len() - 1);
                            self.filters.remove(index);
                        }
                    }
                    KeyCode::Char('a') => return Ok(Some(Popup::AddFilter(AddFilter::new(self)))),
                    KeyCode::Char('e') => {
                        if let Some(index) = self.table_state.selected() {
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
        .block(table_block)
        .row_highlight_style(Style::default().light_yellow().on_black());

        frame.render_stateful_widget(filter_table, table_area, &mut self.table_state);
    }
}

impl PopupHandler for AddFilter {
    async fn handle_event(
        mut self,
        _app: &mut App,
        event: &Event,
    ) -> Result<Option<Popup>, AppError> {
        if let Event::Key(key) = event {
            if key.kind == event::KeyEventKind::Press {
                match key.code {
                    KeyCode::Up => self.prev_field(),
                    KeyCode::Down => self.next_field(),
                    KeyCode::Left => match self.selected_field {
                        AddFilterField::Type => {
                            self.filter = self.selected_type.prev();
                        }
                        AddFilterField::Value => {
                            self.prev_index();
                        }
                        AddFilterField::Submit => (),
                    },
                    KeyCode::Right => match self.selected_field {
                        AddFilterField::Type => {
                            self.filter = self.selected_type.next();
                        }
                        AddFilterField::Value => {
                            self.next_index();
                        }
                        AddFilterField::Submit => (),
                    },
                    KeyCode::Enter => match self.selected_field {
                        AddFilterField::Type => (),
                        AddFilterField::Value => match self.filter {
                            TransactionFilter::Type(ref mut transaction_type_map) => {
                                let t_type = TransactionType::from_repr(self.index)
                                    .expect("AddFilter index should always be a valid repr");
                                transaction_type_map[t_type] = !transaction_type_map[t_type]
                            }
                            TransactionFilter::DateRange(_date_range) => todo!(),
                            TransactionFilter::Not(_transaction_filter) => todo!(),
                            x => panic!("Custom filter should never be of type {x:?}"),
                        },
                        AddFilterField::Submit => {
                            self.pop_under.filters.push(self.filter);
                            return Ok(Some(Popup::FilterResults(self.pop_under)));
                        }
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
        let mut value_index = None;

        {
            use AddFilterField::*;
            match selected_field {
                Submit => submit_field = submit_field.style(active_style),
                Type => type_field = type_field.style(active_style),
                Value => {
                    values_field = values_field.style(active_style);
                    value_index = Some(*index);
                }
            };
        }

        let type_text = Tabs::new(<AddFilterType as VariantNames>::VARIANTS.iter().copied())
            .select(*selected_type as usize)
            .block(type_field);
        let values_text = display_filter_values(filter, value_index).block(values_field);
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

impl TryFrom<&TransactionFilter> for AddFilterType {
    type Error = MissingVariant<TransactionFilter, AddFilterType>;
    fn try_from(value: &TransactionFilter) -> Result<Self, Self::Error> {
        match value {
            TransactionFilter::Type(_) => Ok(AddFilterType::TransactionType),
            TransactionFilter::DateRange(_) => Ok(AddFilterType::DateRange),
            TransactionFilter::Not(transaction_filter) => {
                AddFilterType::try_from(transaction_filter.as_ref())
            }
            _ => Err(MissingVariant(value.clone(), PhantomData)),
        }
    }
}

fn filters_as_rows(filters: &[TransactionFilter]) -> impl Iterator<Item = Row<'_>> {
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

fn display_filter_values(filter: &TransactionFilter, index: Option<i16>) -> Paragraph<'_> {
    let active_style = Style::default().light_yellow().on_black();
    match filter {
        TransactionFilter::Type(transaction_types) => {
            Paragraph::new(Line::from_iter(Itertools::intersperse(
                transaction_types
                    .kv_pairs()
                    .enumerate()
                    .map(|(i, (t_type, selected))| {
                        let text = Span::from(t_type.to_string());
                        if index.is_some_and(|index| i as i16 == index) {
                            text.style(active_style)
                        } else if *selected {
                            text.style(Style::default().fg(Color::Black).bg(Color::White))
                        } else {
                            text
                        }
                    }),
                Span::from(", "),
            )))
        }
        TransactionFilter::DateRange(date_range) => {
            let mut start = Span::from(date_range.start_bound_string());
            let mut end = Span::from(date_range.end_bound_string());
            match index {
                Some(1) => end = end.style(active_style),
                Some(_) => start = start.style(active_style),
                None => (),
            };
            Paragraph::new(Line::from_iter([start, Span::from("-"), end]))
        }
        TransactionFilter::Not(filter) => display_filter_values(filter, index),
        _ => Paragraph::new(""),
    }
}
