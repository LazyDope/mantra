use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    prelude::*,
    widgets::{Row, Table},
};
use thiserror::Error;

use crate::{
    config::{Config, ConfigError},
    storage::{Storage, StorageLoadError, StorageRunError},
    TransactionType, User,
};

pub struct App<'a> {
    config: Config,
    storage: Storage,
    current_user: Option<User>,
    transactions: Option<Table<'a>>,
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

impl<'a> App<'a> {
    pub async fn init(username: String) -> Result<Self, AppInitError> {
        let config = Config::load_or_create();
        let storage = Storage::load_or_create().await?;
        let user = storage.get_or_create_user(username.to_lowercase()).await?;
        let widths = [
            Constraint::Fill(1),
            Constraint::Fill(3),
            Constraint::Fill(1),
        ];
        let rows: Vec<_> = storage
            .get_transactions(user.id, ..)
            .await
            .unwrap()
            .into_iter()
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
        let table_widget = Table::new(rows, widths);
        Ok(App {
            config: config.await?,
            storage,
            current_user: Some(user),
            transactions: Some(table_widget),
        })
    }

    pub fn ui<'b>(&self, frame: &mut Frame<'b>)
    where
        'b: 'a,
    {
        frame.render_widget(&self.transactions, frame.area());
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
                            let rows: Vec<_> = self
                                .storage
                                .get_transactions(
                                    self.current_user.as_ref().map(|v| v.id).unwrap(),
                                    ..,
                                )
                                .await
                                .unwrap()
                                .into_iter()
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
                            self.transactions = Some(self.transactions.take().unwrap().rows(rows));
                        }
                        _ => (),
                    }
                }
            }
        }
        Ok(true)
    }
}
