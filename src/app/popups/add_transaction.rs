use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    layout::Flex,
    prelude::*,
    widgets::{Block, Clear, Paragraph},
};
use text::ToText;

use crate::CursoredString;
use crate::{
    app::{App, AppError},
    TransactionType,
};

#[derive(Default)]
pub struct AddTransaction {
    pub trans_type: TransactionType,
    pub amount: i32,
    pub msg: CursoredString,
    pub selected_field: AddTransactionField,
}

#[derive(Default, PartialEq, Eq)]
pub enum AddTransactionField {
    #[default]
    TransactionType,
    Amount,
    Message,
    Submit,
}

fn get_modified_value(modifiers: KeyModifiers) -> i32 {
    let mut value = 1;
    if modifiers.contains(KeyModifiers::SHIFT) {
        value *= 5;
    }
    if modifiers.contains(KeyModifiers::CONTROL) {
        value *= 50;
    }
    if modifiers.contains(KeyModifiers::ALT) {
        value *= 200;
    }

    value
}

impl AddTransaction {
    pub(crate) async fn process_event(
        mut self,
        app: &mut App,
        event: Event,
    ) -> Result<Option<Self>, AppError> {
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
                        AddTransactionField::Amount => {
                            self.amount -= get_modified_value(key.modifiers);
                        }
                        AddTransactionField::Message => self.msg.prev(),
                        AddTransactionField::TransactionType => {
                            self.trans_type = self.trans_type.prev()
                        }
                        _ => (),
                    },
                    KeyCode::Right => match self.selected_field {
                        AddTransactionField::Amount => {
                            self.amount += get_modified_value(key.modifiers);
                        }
                        AddTransactionField::Message => self.msg.next(),
                        AddTransactionField::TransactionType => {
                            self.trans_type = self.trans_type.next()
                        }
                        _ => (),
                    },
                    KeyCode::Enter => match self.selected_field {
                        AddTransactionField::Submit => {
                            let AddTransaction {
                                trans_type,
                                amount,
                                msg,
                                ..
                            } = self;
                            app.storage
                                .add_transaction(
                                    app.current_user.as_ref().map(|v| v.id).unwrap(),
                                    amount,
                                    trans_type,
                                    &msg.text,
                                )
                                .await?;

                            app.status_text = String::from("Added transaction");
                            app.update_table().await?;
                            return Ok(None);
                        }
                        _ => self.selected_field.next(),
                    },
                    KeyCode::Backspace => match self.selected_field {
                        AddTransactionField::Message => self.msg.remove_behind(),
                        _ => (),
                    },
                    KeyCode::Delete => match self.selected_field {
                        AddTransactionField::Message => self.msg.remove_ahead(),
                        _ => (),
                    },
                    KeyCode::Insert => match self.selected_field {
                        AddTransactionField::Message => self.msg.inserting = !self.msg.inserting,
                        _ => (),
                    },
                    KeyCode::Esc => return Ok(None),
                    KeyCode::Char(c) => match self.selected_field {
                        AddTransactionField::Message => self.msg.insert(c),
                        _ => (),
                    },
                    _ => (),
                }
            }
        }
        Ok(Some(self))
    }

    pub(super) fn render_to_frame(&self, area: ratatui::prelude::Rect, frame: &mut Frame)
    where
        Self: Sized,
    {
        let Self {
            trans_type,
            amount,
            msg,
            selected_field,
        } = self;

        const TYPE_HEIGHT: u16 = 1;
        const AMOUNT_HEIGHT: u16 = 1;
        const MSG_HEIGHT: u16 = 3;
        const SUBMIT_HEIGHT: u16 = 1;
        const BORDER_SIZE: u16 = 1;
        const SUBMIT_TEXT: &str = "Submit";

        let [area] = Layout::vertical([Constraint::Length(
            TYPE_HEIGHT + AMOUNT_HEIGHT + MSG_HEIGHT + 10 * BORDER_SIZE,
        )])
        .flex(Flex::Center)
        .areas(area);
        let [area] = Layout::horizontal([Constraint::Percentage(40)])
            .flex(Flex::Center)
            .areas(area);
        let block = Block::bordered().title("Add Transaction");
        frame.render_widget(Clear, area);
        frame.render_widget(block, area);
        let area = area.inner(Margin::new(BORDER_SIZE, BORDER_SIZE));
        let [type_area, amount_area, msg_area, submit_area] = Layout::vertical([
            Constraint::Length(TYPE_HEIGHT + BORDER_SIZE * 2),
            Constraint::Length(AMOUNT_HEIGHT + BORDER_SIZE * 2),
            Constraint::Length(MSG_HEIGHT + BORDER_SIZE * 2),
            Constraint::Length(SUBMIT_HEIGHT + BORDER_SIZE * 2),
        ])
        .areas(area);

        let mut type_field = Block::bordered().title("Type");
        let mut amount_field = Block::bordered().title("Amount");
        let mut msg_field = Block::bordered().title("Message");
        let mut submit_field = Block::bordered();

        let active_style = Style::default().bg(Color::LightYellow).fg(Color::Black);

        {
            use AddTransactionField::*;
            match selected_field {
                TransactionType => type_field = type_field.style(active_style),
                Amount => amount_field = amount_field.style(active_style),
                Message => {
                    msg_field = msg_field.style(active_style);
                    frame.set_cursor_position(Position::new(
                        msg_area.x + msg.index as u16 + 1,
                        msg_area.y + 1,
                    ));
                }
                Submit => submit_field = submit_field.style(active_style),
            };
        }

        let type_text = Paragraph::new(trans_type.to_text()).block(type_field);
        let amount_text = Paragraph::new(amount.to_text()).block(amount_field);
        let msg_text = Paragraph::new(msg.to_text()).block(msg_field);
        let submit_text = Paragraph::new(SUBMIT_TEXT)
            .block(submit_field)
            .alignment(Alignment::Center);

        frame.render_widget(type_text, type_area);
        frame.render_widget(amount_text, amount_area);
        frame.render_widget(msg_text, msg_area);
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

impl AddTransactionField {
    fn next(&mut self) {
        use AddTransactionField::*;
        *self = match self {
            TransactionType => Amount,
            Amount => Message,
            Message => Submit,
            Submit => TransactionType,
        }
    }

    fn prev(&mut self) {
        use AddTransactionField::*;
        *self = match self {
            TransactionType => Submit,
            Amount => TransactionType,
            Message => Amount,
            Submit => Message,
        }
    }
}
