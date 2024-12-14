use crossterm::event::{self, Event, KeyCode};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use ratatui::{
    layout::Flex,
    prelude::{Rect, *},
    widgets::{Block, Clear, Paragraph, Tabs, Wrap},
};
use strum::{EnumCount, VariantNames};
use text::ToText;

use crate::CursoredString;
use crate::{
    app::{App, AppError},
    storage::TransactionType,
};

use super::{Popup, PopupHandler};

/// Handles the creation of new transactions
#[derive(Default)]
pub struct AddTransaction {
    pub trans_type: TransactionType,
    pub amount: i32,
    pub msg: CursoredString,
    pub selected_field: AddTransactionField,
}

/// Selectable fields for [`AddTransaction`]
#[derive(Default, PartialEq, Eq, FromPrimitive, EnumCount, Clone, Copy)]
pub enum AddTransactionField {
    #[default]
    TransactionType = 0,
    Amount,
    Message,
    Submit,
}

impl AddTransactionField {
    /// Switch the selected field to the next one
    fn next(&mut self) {
        *self = FromPrimitive::from_isize(
            (*self as isize + 1).rem_euclid(<Self as EnumCount>::COUNT as isize),
        )
        .expect("Will always be a valid isize unless AddTransactionField became an empty enum")
    }

    /// Switch the selected field to the previous one
    fn prev(&mut self) {
        *self = FromPrimitive::from_isize(
            (*self as isize - 1).rem_euclid(<Self as EnumCount>::COUNT as isize),
        )
        .expect("Will always be a valid isize unless AddTransactionField became an empty enum")
    }
}

impl PopupHandler for AddTransaction {
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
                        AddTransactionField::Amount => {
                            self.amount -= crate::value_from_modifiers(key.modifiers);
                        }
                        AddTransactionField::Message => self.msg.right(),
                        AddTransactionField::TransactionType => {
                            self.trans_type = self.trans_type.prev()
                        }
                        _ => (),
                    },
                    KeyCode::Right => match self.selected_field {
                        AddTransactionField::Amount => {
                            self.amount += crate::value_from_modifiers(key.modifiers);
                        }
                        AddTransactionField::Message => self.msg.left(),
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
                            app.data
                                .storage
                                .add_transaction(
                                    app.data.current_user.as_ref().map(|v| v.get_id()).unwrap(),
                                    amount,
                                    trans_type,
                                    &msg.buf,
                                )
                                .await?;

                            app.data.status_text = String::from("Added transaction");
                            app.data.update_table().await?;
                            return Ok(None);
                        }
                        _ => self.selected_field.next(),
                    },
                    KeyCode::Backspace => {
                        if let AddTransactionField::Message = self.selected_field {
                            self.msg.remove_behind()
                        }
                    }
                    KeyCode::Delete => {
                        if let AddTransactionField::Message = self.selected_field {
                            self.msg.remove_ahead()
                        }
                    }
                    KeyCode::Insert => {
                        if let AddTransactionField::Message = self.selected_field {
                            self.msg.inserting = !self.msg.inserting
                        }
                    }
                    KeyCode::Esc => return Ok(None),
                    KeyCode::Char(c) => {
                        if let AddTransactionField::Message = self.selected_field {
                            self.msg.insert(c)
                        }
                    }
                    _ => (),
                }
            }
        }
        Ok(Some(Popup::AddTransaction(self)))
    }

    fn render_to_frame(&mut self, area: Rect, frame: &mut Frame)
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
                    let inner_area = msg_area.inner(Margin {
                        horizontal: 1,
                        vertical: 1,
                    });
                    let mapped_index = (msg.cursor_index() as u16)
                        .clamp(0, inner_area.width * inner_area.height - 1);
                    frame.set_cursor_position(Position::new(
                        msg_area.x + mapped_index % inner_area.width + 1,
                        msg_area.y + mapped_index / inner_area.width + 1,
                    ));
                }
                Submit => submit_field = submit_field.style(active_style),
            };
        }

        let type_text = Tabs::new(<TransactionType as VariantNames>::VARIANTS.iter().copied())
            .select(*trans_type as usize)
            .block(type_field);
        let amount_text = Paragraph::new(amount.to_text()).block(amount_field);
        let msg_text = Paragraph::new(msg.as_str())
            .wrap(Wrap { trim: false })
            .block(msg_field);
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
