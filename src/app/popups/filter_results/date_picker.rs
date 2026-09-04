//! Calendar-based date picker used for creating `DateRange` filters

use core::ops::Bound;

use crossterm::event::KeyCode;
use ratatui::{
    prelude::*,
    style::{Color, Style},
    widgets::{
        calendar::{DateStyler, Monthly},
        Block, Paragraph,
    },
};
use time::{Date, Duration, OffsetDateTime};

/// How the date selected by the picker is stored on the `DateRange` filter
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum DateBoundKind {
    #[default]
    Included,
    Excluded,
    Unbounded,
}

/// What [`DatePicker::handle_key`] wants the popup to do after handling a key
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DateBoundAction {
    None,
    /// Switch which bound of the range (start/end) is being edited
    SwitchBound,
    /// Store the selected bound and close the picker
    Select,
    /// Close the picker without changing the bound
    Close,
}

/// Month title, weekday header, up to six weeks, and the calendar block borders
const CALENDAR_HEIGHT: u16 = 10;
/// Kind/year line and two hint lines underneath the calendar
const FOOTER_HEIGHT: u16 = 3;
const BORDER_SIZE: u16 = 1;

/// State for selecting a single date by navigating a month calendar
#[derive(Clone, Debug)]
pub struct DatePicker {
    /// The bound being edited; the `Bound`'s tag is the bound kind and its date is the cursor
    cursor: Bound<Date>,
    /// Optional year being typed, shown instead of the kind selector while `Some`
    year_input: Option<String>,
}

impl Default for DatePicker {
    fn default() -> Self {
        Self::new()
    }
}

impl DatePicker {
    /// Start the picker on today's date
    pub fn new() -> Self {
        Self::from_bound(Bound::Included(today()))
    }

    /// Start the picker on the date inside `bound` if one is set, otherwise today
    pub fn from_bound(bound: Bound<Date>) -> Self {
        Self {
            cursor: match bound {
                Bound::Unbounded => Bound::Included(today()),
                other => other,
            },
            year_input: None,
        }
    }

    /// The date currently highlighted by the cursor, if the bound is not `Unbounded`
    pub fn selected(&self) -> Option<Date> {
        match self.cursor {
            Bound::Included(date) | Bound::Excluded(date) => Some(date),
            Bound::Unbounded => None,
        }
    }

    /// How the selected date is stored on the `DateRange` filter
    pub fn kind(&self) -> DateBoundKind {
        match self.cursor {
            Bound::Included(_) => DateBoundKind::Included,
            Bound::Excluded(_) => DateBoundKind::Excluded,
            Bound::Unbounded => DateBoundKind::Unbounded,
        }
    }

    /// Change how the picked date is stored, keeping the cursor date if one is set
    pub fn set_kind(&mut self, kind: DateBoundKind) {
        self.cursor = match kind {
            DateBoundKind::Included => Bound::Included(self.selected().unwrap_or_else(today)),
            DateBoundKind::Excluded => Bound::Excluded(self.selected().unwrap_or_else(today)),
            DateBoundKind::Unbounded => Bound::Unbounded,
        };
    }

    /// The selected bound, ready to store on a `DateRange` filter
    pub fn bound(&self) -> Bound<Date> {
        self.cursor
    }

    /// Move the cursor forward one day
    pub fn next_day(&mut self) {
        self.cursor = self.cursor.map(|mut date| {
            date += Duration::DAY;
            date
        });
    }

    /// Move the cursor back one day
    pub fn prev_day(&mut self) {
        self.cursor = self.cursor.map(|mut date| {
            date -= Duration::DAY;
            date
        });
    }

    /// Move the cursor forward one week
    pub fn next_week(&mut self) {
        self.cursor = self.cursor.map(|mut date| {
            date += Duration::WEEK;
            date
        });
    }

    /// Move the cursor back one week
    pub fn prev_week(&mut self) {
        self.cursor = self.cursor.map(|mut date| {
            date -= Duration::WEEK;
            date
        });
    }

    /// Move the cursor forward about a month
    pub fn next_month(&mut self) {
        self.cursor = self.cursor.map(|mut date| {
            date += 4 * Duration::WEEK;
            date
        });
    }

    /// Move the cursor back about a month
    pub fn prev_month(&mut self) {
        self.cursor = self.cursor.map(|mut date| {
            date -= 4 * Duration::WEEK;
            date
        });
    }

    /// Jump the cursor to the same month and day of `year`
    pub fn set_year(&mut self, year: i32) {
        self.cursor = match self.cursor {
            Bound::Included(date) => Bound::Included(clamp_year(date, year)),
            Bound::Excluded(date) => Bound::Excluded(clamp_year(date, year)),
            Bound::Unbounded => Bound::Unbounded,
        };
    }

    /// Handle a key press while the picker is open
    pub fn handle_key(&mut self, key: KeyCode) -> DateBoundAction {
        if self.year_input.is_some() {
            match key {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    if let Some(year_input) = self.year_input.as_mut() {
                        if year_input.len() < 4 {
                            year_input.push(c);
                        }
                    }
                }
                KeyCode::Backspace => {
                    if let Some(year_input) = self.year_input.as_mut() {
                        year_input.pop();
                    }
                }
                KeyCode::Esc => self.year_input = None,
                KeyCode::Enter => {
                    let year = self
                        .year_input
                        .as_deref()
                        .and_then(|digits| digits.parse::<i32>().ok());
                    self.year_input = None;
                    if let Some(year) = year {
                        self.set_year(year);
                    }
                }
                _ => (),
            }
            return DateBoundAction::None;
        }
        match key {
            KeyCode::Up => self.prev_week(),
            KeyCode::Down => self.next_week(),
            KeyCode::Left => self.prev_day(),
            KeyCode::Right => self.next_day(),
            KeyCode::PageUp | KeyCode::Char('<') => self.prev_month(),
            KeyCode::PageDown | KeyCode::Char('>') => self.next_month(),
            KeyCode::Char('i') => self.set_kind(DateBoundKind::Included),
            KeyCode::Char('e') => self.set_kind(DateBoundKind::Excluded),
            KeyCode::Char('u') => self.set_kind(DateBoundKind::Unbounded),
            KeyCode::Char('y') if self.kind() != DateBoundKind::Unbounded => {
                self.year_input = Some(String::new())
            }
            KeyCode::Tab => return DateBoundAction::SwitchBound,
            KeyCode::Enter => return DateBoundAction::Select,
            KeyCode::Esc => return DateBoundAction::Close,
            _ => (),
        }
        DateBoundAction::None
    }

    /// Height the picker needs to render, calendar only while the bound has a date
    pub fn height(&self) -> u16 {
        let calendar = if self.kind() == DateBoundKind::Unbounded {
            0
        } else {
            CALENDAR_HEIGHT + 2 * BORDER_SIZE
        };
        calendar + FOOTER_HEIGHT
    }

    /// Render the calendar (if the bound has a date) and the kind/year selector and hints
    pub fn render_to_frame(&self, area: Rect, frame: &mut Frame, title: &str, active_style: Style) {
        let calendar_height = self.height() - FOOTER_HEIGHT;
        let [calendar_area, footer_area] = Layout::vertical([
            Constraint::Length(calendar_height),
            Constraint::Length(FOOTER_HEIGHT),
        ])
        .areas(area);

        if calendar_height > 0 {
            let cursor = self
                .selected()
                .expect("calendar only renders when a date is set");
            let styler = PickerStyler {
                cursor,
                cursor_style: active_style,
                today_style: Style::default().fg(Color::Blue),
            };
            let block = Block::bordered().title(title).border_style(active_style);
            let calendar = Monthly::new(cursor, styler)
                .block(block)
                .show_month_header(Style::default().bold())
                .show_weekdays_header(Style::default().bold().fg(Color::DarkGray))
                .show_surrounding(Style::default().fg(Color::DarkGray))
                .default_style(Style::default());
            frame.render_widget(calendar, calendar_area);
        }

        self.render_footer(frame, footer_area, active_style);
    }

    /// Status and hint lines shown under the calendar
    fn render_footer(&self, frame: &mut Frame, area: Rect, active_style: Style) {
        let [kind_area, hint_area, hint_area_2] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(area);
        let bound_kind = self.kind();
        let kind_span = |kind: DateBoundKind, text: &'static str| {
            let span = Span::from(text);
            if kind == bound_kind {
                span.style(active_style)
            } else {
                span
            }
        };
        let kind_line = match &self.year_input {
            Some(digits) => Paragraph::new(Line::from_iter([
                Span::from("Year: "),
                Span::from(format!("{digits}_")),
            ])),
            None => Paragraph::new(Line::from_iter([
                Span::from("Kind: "),
                kind_span(DateBoundKind::Included, "Included"),
                Span::from("  "),
                kind_span(DateBoundKind::Excluded, "Excluded"),
                Span::from("  "),
                kind_span(DateBoundKind::Unbounded, "Unbounded"),
            ])),
        };
        frame.render_widget(kind_line, kind_area);
        let navigation_hint = if bound_kind == DateBoundKind::Unbounded {
            "Unbounded: no date set; press i or e to set one"
        } else {
            "Arrows: move  < >: month  Tab: other bound"
        };
        frame.render_widget(Paragraph::new(navigation_hint), hint_area);
        let kind_hint = if bound_kind == DateBoundKind::Unbounded {
            "i/e: bound kind  Enter: select  Esc: back"
        } else {
            "i/e/u: bound kind  y: year  Enter: select  Esc: back"
        };
        frame.render_widget(Paragraph::new(kind_hint), hint_area_2);
    }
}

/// Clamp the day of `date` to fit within the same month of `year`
fn clamp_year(date: Date, year: i32) -> Date {
    let day = date.day().min(date.month().length(year));
    Date::from_calendar_date(year, date.month(), day).expect("The clamped day is always valid")
}

/// Style the cursor and today within a [`Monthly`] calendar
struct PickerStyler {
    cursor: Date,
    cursor_style: Style,
    today_style: Style,
}

impl DateStyler for PickerStyler {
    fn get_style(&self, date: Date) -> Style {
        if date == self.cursor {
            self.cursor_style
        } else if date == today() {
            self.today_style
        } else {
            Style::default()
        }
    }
}

/// The current date in the local timezone, falling back to UTC
fn today() -> Date {
    OffsetDateTime::now_local()
        .map(|datetime| datetime.date())
        .unwrap_or_else(|_| OffsetDateTime::now_utc().date())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};
    use time::Month;

    #[test]
    fn navigates_months() {
        let jan_31 = Date::from_calendar_date(2024, Month::January, 31).unwrap();
        let mut picker = DatePicker::from_bound(Bound::Included(jan_31));

        picker.next_month();
        assert_eq!(
            picker.selected().unwrap(),
            Date::from_calendar_date(2024, Month::February, 28).unwrap()
        );

        picker.next_month();
        assert_eq!(
            picker.selected().unwrap(),
            Date::from_calendar_date(2024, Month::March, 27).unwrap()
        );

        picker.prev_month();
        picker.prev_month();
        assert_eq!(
            picker.selected().unwrap(),
            Date::from_calendar_date(2024, Month::January, 31).unwrap()
        );

        picker.prev_month();
        assert_eq!(
            picker.selected().unwrap(),
            Date::from_calendar_date(2024, Month::January, 3).unwrap()
        );
    }

    #[test]
    fn navigates_days_and_weeks() {
        let third = Date::from_calendar_date(2026, Month::September, 3).unwrap();
        let mut picker = DatePicker::from_bound(Bound::Included(third));

        picker.next_day();
        picker.next_day();
        assert_eq!(
            picker.selected().unwrap(),
            Date::from_calendar_date(2026, Month::September, 5).unwrap()
        );

        picker.next_week();
        assert_eq!(
            picker.selected().unwrap(),
            Date::from_calendar_date(2026, Month::September, 12).unwrap()
        );

        picker.prev_week();
        picker.prev_day();
        assert_eq!(
            picker.selected().unwrap(),
            Date::from_calendar_date(2026, Month::September, 4).unwrap()
        );
    }

    #[test]
    fn sets_year_clamping_the_day() {
        let feb_29 = Date::from_calendar_date(2024, Month::February, 29).unwrap();
        let mut picker = DatePicker::from_bound(Bound::Included(feb_29));

        picker.set_year(2026);
        assert_eq!(
            picker.selected().unwrap(),
            Date::from_calendar_date(2026, Month::February, 28).unwrap()
        );

        picker.set_year(2028);
        assert_eq!(
            picker.selected().unwrap(),
            Date::from_calendar_date(2028, Month::February, 28).unwrap()
        );
    }

    #[test]
    fn toggles_bound_kind_and_reads_bound() {
        let date = Date::from_calendar_date(2026, Month::March, 14).unwrap();
        let mut picker = DatePicker::from_bound(Bound::Included(date));

        assert_eq!(picker.kind(), DateBoundKind::Included);
        assert_eq!(picker.bound(), Bound::Included(date));

        picker.set_kind(DateBoundKind::Excluded);
        assert_eq!(picker.kind(), DateBoundKind::Excluded);
        assert_eq!(picker.bound(), Bound::Excluded(date));

        picker.set_kind(DateBoundKind::Unbounded);
        assert_eq!(picker.kind(), DateBoundKind::Unbounded);
        assert_eq!(picker.bound(), Bound::Unbounded);

        picker.set_kind(DateBoundKind::Included);
        assert_eq!(picker.kind(), DateBoundKind::Included);
        // the date is gone after going Unbounded, so Included starts on today again
        assert_eq!(picker.selected(), Some(today()));
    }

    #[test]
    fn handles_keys_for_kind_actions_and_year_input() {
        let date = Date::from_calendar_date(2026, Month::March, 14).unwrap();
        let mut picker = DatePicker::from_bound(Bound::Included(date));

        assert_eq!(picker.handle_key(KeyCode::Char('e')), DateBoundAction::None);
        assert_eq!(picker.kind(), DateBoundKind::Excluded);
        assert_eq!(
            picker.handle_key(KeyCode::Tab),
            DateBoundAction::SwitchBound
        );
        assert_eq!(picker.handle_key(KeyCode::Enter), DateBoundAction::Select);
        assert_eq!(picker.handle_key(KeyCode::Esc), DateBoundAction::Close);

        picker.handle_key(KeyCode::Char('y'));
        picker.handle_key(KeyCode::Char('2'));
        picker.handle_key(KeyCode::Char('0'));
        picker.handle_key(KeyCode::Char('3'));
        picker.handle_key(KeyCode::Char('1'));
        picker.handle_key(KeyCode::Enter);
        assert_eq!(
            picker.selected(),
            Some(Date::from_calendar_date(2031, Month::March, 14).unwrap())
        );
    }

    #[test]
    fn renders_calendar() {
        let mut terminal = Terminal::new(TestBackend::new(40, 15)).unwrap();
        let cursor = Date::from_calendar_date(2026, Month::September, 10).unwrap();
        let picker = DatePicker::from_bound(Bound::Included(cursor));
        terminal
            .draw(|frame| {
                picker.render_to_frame(
                    frame.area(),
                    frame,
                    "Start Date",
                    Style::default().bg(Color::LightYellow).fg(Color::Black),
                );
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let text: String = buffer.content().iter().map(|cell| cell.symbol()).collect();
        assert!(text.contains("September 2026"));
        assert!(text.contains("Start Date"));
    }
}
