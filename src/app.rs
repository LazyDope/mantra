use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    prelude::*,
    widgets::{Block, Row, Table, TableState},
};
use thiserror::Error;

use crate::{
    config::{Config, ConfigError},
    storage::{Storage, StorageLoadError, StorageRunError},
    Transaction, TransactionType, User,
};

pub struct App {
    config: Config,
    storage: Storage,
    current_user: Option<User>,
    transactions: Vec<Transaction>,
    table_state: TableState,
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
}

impl App {
    pub async fn init(username: String) -> Result<Self, AppInitError> {
        let config = Config::load_or_create();
        let storage = Storage::load_or_create().await?;
        let user = storage.get_or_create_user(username.to_lowercase()).await?;
        let transactions: Vec<Transaction> = storage.get_transactions(user.id, ..).await?;
        Ok(App {
            config: config.await?,
            storage,
            current_user: Some(user),
            transactions,
            table_state: TableState::new(),
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
            .header(Row::new(["Amount", "Note", "Date/Time"]).underlined());
        let [table_area, status_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).areas(frame.area());
        frame.render_stateful_widget(&table_widget, table_area, &mut self.table_state);
        frame.render_widget(Block::bordered().title("Status"), status_area);
    }

    pub async fn run(&mut self) -> Result<bool, AppError> {
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => {
                            return Ok(false);
                        }
                        KeyCode::Char('a') => {
                            self.storage
                                .add_transaction(
                                    self.current_user.as_ref().map(|v| v.id).unwrap(),
                                    1000,
                                    TransactionType::Other,
                                    "Testing!",
                                )
                                .await?;

                            self.update_table().await?;
                        }
                        KeyCode::Char('c') => {
                            self.storage
                                .remove_transactions(&format!(
                                    "user_id = {}",
                                    self.current_user.as_ref().map(|v| v.id).unwrap()
                                ))
                                .await?;

                            self.update_table().await?;
                        }
                        KeyCode::Down => self.table_state.select_next(),
                        KeyCode::Up => self.table_state.select_previous(),
                        _ => (),
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
