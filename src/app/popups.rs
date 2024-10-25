use std::fmt::Display;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};

use crate::TransactionType;

use super::{App, AppError};

pub enum Popup {
    AddTransaction(AddTransaction),
}

#[derive(Default)]
pub struct AddTransaction {
    pub trans_type: TransactionType,
    pub amount: i32,
    pub msg: CursoredString,
    pub selected_field: AddTransactionField,
}

#[derive(Default)]
pub struct CursoredString {
    pub text: String,
    pub index: usize,
    pub inserting: bool,
}

#[derive(Default, PartialEq, Eq)]
pub enum AddTransactionField {
    #[default]
    TransactionType,
    Amount,
    Message,
    Submit,
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
                .map(|inner| Self::AddTransaction(inner))),
        }
    }
}

fn get_modified_value(modifiers: KeyModifiers) -> i32 {
    let mut value = 1;
    if modifiers.contains(KeyModifiers::SHIFT) {
        value *= 5;
    }
    if modifiers.contains(KeyModifiers::CONTROL) {
        value *= 100;
    }
    if modifiers.contains(KeyModifiers::ALT) {
        value *= 1000;
    }

    value
}

impl AddTransaction {
    pub async fn process_event(
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
                        _ => (),
                    },
                    KeyCode::Right => match self.selected_field {
                        AddTransactionField::Amount => {
                            self.amount += get_modified_value(key.modifiers);
                        }
                        AddTransactionField::Message => self.msg.next(),
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
                    KeyCode::Char(c) => match self.selected_field {
                        AddTransactionField::Message => self.msg.insert(c),
                        _ => (),
                    },
                    KeyCode::Backspace => match self.selected_field {
                        AddTransactionField::Message => self.msg.remove_behind(),
                        _ => (),
                    },
                    KeyCode::Delete => match self.selected_field {
                        AddTransactionField::Message => self.msg.remove_ahead(),
                        _ => (),
                    },
                    KeyCode::Esc => return Ok(None),
                    _ => (),
                }
            }
        }
        Ok(Some(self))
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
                if index == self.index {
                    return false;
                }
                index += 1;
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
        if !self.inserting {
            self.index += 1
        }
    }
}

impl Display for CursoredString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.text.fmt(f)
    }
}
