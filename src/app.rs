use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    layout::Flex,
    prelude::*,
    widgets::{Block, Clear, Paragraph, Row, Table, TableState},
};
use text::ToText;
use thiserror::Error;

use crate::{
    config::{Config, ConfigError},
    storage::{Storage, StorageLoadError, StorageRunError},
    Transaction, TransactionType, User,
};

pub mod popups;
use popups::{AddTransaction, AddTransactionField, Popup};

pub struct App {
    config: Config,
    storage: Storage,
    current_user: Option<User>,
    transactions: Vec<Transaction>,
    table_state: TableState,
    status_text: String,
    popup: Option<Popup>,
}

#[derive(Error, Debug)]
pub enum AppInitError {
    #[error(transparent)]
    ConfigError(#[from] ConfigError),
    #[error(transparent)]
    StorageLoadError(#[from] StorageLoadError),
    #[error(transparent)]
    StorageRunError(#[from] StorageRunError),
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    StorageRunError(#[from] StorageRunError),
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
}

pub enum AppMode {
    Normal,
    Delete,
    Create,
}

impl App {
    pub async fn init(username: String) -> Result<Self, AppInitError> {
        let config = Config::load_or_create();
        let storage = Storage::load_or_create().await?;
        let user = storage.get_or_create_user(username.to_lowercase()).await?;
        Ok(App {
            config: config.await?,
            transactions: storage.get_transactions(user.id, ..).await?,
            storage,
            current_user: Some(user),
            table_state: TableState::default(),
            status_text: String::new(),
            popup: None,
        })
    }

    pub fn ui(&mut self, frame: &mut Frame<'_>) {
        let widths = [
            Constraint::Fill(1),
            Constraint::Fill(3),
            Constraint::Fill(1),
        ];

        let rows: Vec<_> = self
            .transactions
            .iter()
            .map(|trans| {
                Row::new([
                    format!("{}", trans.value),
                    trans.msg.clone(),
                    trans
                        .datetime
                        .assume_utc()
                        .to_offset(self.config.timezone)
                        .format(time::macros::format_description!(
                            "[year]-[month]-[day] [hour]:[minute]"
                        ))
                        .unwrap(),
                ])
            })
            .collect();
        let block = Block::bordered()
            .border_style(Style::new().white())
            .title("MAN/TRA");
        let table_widget = Table::new(rows, widths)
            .block(block)
            .header(Row::new(["Amount", "Note", "Date/Time"]).underlined())
            .highlight_style(Style::new().black().on_white());
        let [table_area, status_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).areas(frame.area());
        frame.render_stateful_widget(&table_widget, table_area, &mut self.table_state);
        frame.render_widget(
            Paragraph::new(self.status_text.clone()).block(Block::bordered().title("Status")),
            status_area,
        );

        match &self.popup {
            Some(Popup::AddTransaction(AddTransaction {
                trans_type,
                amount,
                msg,
                selected_field,
            })) => {
                const TYPE_HEIGHT: u16 = 1;
                const AMOUNT_HEIGHT: u16 = 1;
                const MSG_HEIGHT: u16 = 3;
                const SUBMIT_HEIGHT: u16 = 1;
                const BORDER_SIZE: u16 = 1;
                const SUBMIT_TEXT: &'static str = "Submit";

                let [area] = Layout::vertical([Constraint::Length(
                    TYPE_HEIGHT + AMOUNT_HEIGHT + MSG_HEIGHT + 10 * BORDER_SIZE,
                )])
                .flex(Flex::Center)
                .areas(table_area);
                let [area] = Layout::horizontal([Constraint::Percentage(40)])
                    .flex(Flex::Center)
                    .areas(area);
                let block = Block::bordered().title("Add Transaction");
                frame.render_widget(Clear, area);
                frame.render_widget(block, area);
                let [_, area, _] = Layout::horizontal([
                    Constraint::Length(BORDER_SIZE),
                    Constraint::Fill(1),
                    Constraint::Length(BORDER_SIZE),
                ])
                .areas(area);
                let [_, type_area, amount_area, msg_area, submit_area, _] = Layout::vertical([
                    Constraint::Length(BORDER_SIZE),
                    Constraint::Length(TYPE_HEIGHT + BORDER_SIZE * 2),
                    Constraint::Length(AMOUNT_HEIGHT + BORDER_SIZE * 2),
                    Constraint::Length(MSG_HEIGHT + BORDER_SIZE * 2),
                    Constraint::Length(SUBMIT_HEIGHT + BORDER_SIZE * 2),
                    Constraint::Length(BORDER_SIZE),
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
            None => (),
        }
    }

    pub async fn run(&mut self) -> Result<bool, AppError> {
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Some(popup) = self.popup.take() {
                self.popup = popup.process_event(self, event::read()?).await?;
            } else {
                if let Event::Key(key) = event::read()? {
                    if key.kind == event::KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') => {
                                return Ok(false);
                            }
                            KeyCode::Char('a') => {
                                self.popup = Some(Popup::AddTransaction(AddTransaction::default()));
                            }
                            KeyCode::Char('c') => {
                                self.storage
                                    .remove_transactions(&format!(
                                        "user_id = {}",
                                        self.current_user.as_ref().map(|v| v.id).unwrap()
                                    ))
                                    .await?;

                                self.status_text = String::from("Cleared log");
                                self.update_table().await?;
                            }
                            KeyCode::Char('d') => {
                                if let Some(index) = self.table_state.selected() {
                                    let transaction = &self.transactions[index];
                                    self.storage
                                        .remove_transactions(&format!(
                                            "id = {}",
                                            transaction.trans_id
                                        ))
                                        .await?;
                                    self.status_text = format!(
                                        "Deleted \"{} | {}\"",
                                        transaction.value, transaction.msg
                                    );
                                    self.update_table().await?
                                }
                            }
                            KeyCode::Down => self.table_state.select_next(),
                            KeyCode::Up => self.table_state.select_previous(),
                            _ => (),
                        }
                    }
                }
            }
        }
        Ok(true)
    }

    pub async fn update_table(&mut self) -> Result<(), AppError> {
        self.transactions = self
            .storage
            .get_transactions(self.current_user.as_ref().map(|v| v.id).unwrap(), ..)
            .await?;
        Ok(())
    }
}
