use std::ops::{Bound, RangeBounds};

use async_std::stream::{Stream, StreamExt};
use crossterm::event::{self, Event, KeyCode};
use ratatui::{prelude::*, widgets::*};
use sqlx::{migrate::MigrateDatabase, Row, Sqlite, SqlitePool};
use thiserror::Error;
use xdg::BaseDirectories;

mod config;
use config::{Config, ConfigError};

pub struct App {
    config: Config,
    storage: Storage,
    current_user: Option<User>,
}

pub struct User {
    id: i32,
    name: String,
}

struct Storage {
    db: SqlitePool,
}

pub struct MissingType;

pub struct Transaction {
    datetime: time::PrimitiveDateTime,
    user_id: i32,
    value: i32,
    transaction_type: TransactionType,
    msg: String,
}

pub struct TransactionStream {
    stream: Box<dyn Stream<Item = Transaction>>,
    query: *mut str,
}

#[derive(Error, Debug)]
pub enum AppInitError {
    #[error(transparent)]
    ConfigError(#[from] ConfigError),
    #[error(transparent)]
    StorageLoadError(#[from] StorageLoadError),
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum StorageLoadError {
    #[error(transparent)]
    BaseDirsError(#[from] xdg::BaseDirectoriesError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    DBError(#[from] sqlx::Error),
}

#[derive(Error, Debug)]
pub enum StorageError {
    #[error(transparent)]
    DBError(#[from] sqlx::Error),
}

pub enum TransactionType {
    Other = 0,
    Character = 1,
}

fn base_dirs() -> Result<BaseDirectories, xdg::BaseDirectoriesError> {
    BaseDirectories::with_prefix("mantra")
}

impl App {
    pub async fn init() -> Result<Self, AppInitError> {
        let config = Config::load_or_create();
        let storage = Storage::load_or_create();
        Ok(App {
            config: config.await?,
            storage: storage.await?,
            current_user: None,
        })
    }

    pub fn ui<'a>(&self, frame: &mut Frame<'a>) {
        frame.render_widget(
            Paragraph::new("Hello World!")
                .block(Block::default().title("Greeting").borders(Borders::ALL)),
            frame.size(),
        );
    }

    pub async fn run(&self) -> Result<bool, AppError> {
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

impl Storage {
    async fn load_or_create() -> Result<Self, StorageLoadError> {
        let db_path = base_dirs()?.place_data_file("log.db")?;
        let db_url = format!("sqlite://{}", db_path.display());
        println!("{}", db_url);

        if !Sqlite::database_exists(&db_url).await.unwrap_or(false) {
            Sqlite::create_database(&db_url).await?
        };

        let db = SqlitePool::connect(&db_url).await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS transactions (datetime INTEGER PRIMARY KEY NOT NULL, user_id INTEGER NOT NULL, value INTEGER NOT NULL, type INTEGER NOT NULL, message TEXT)").execute(&db).await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS users (user_id INTEGER PRIMARY KEY NOT NULL, name TEXT UNIQUE NOT NULL)").execute(&db).await?;
        Ok(Storage { db })
    }

    async fn add_transaction(
        &self,
        user: i32,
        amount: i32,
        transaction_type: TransactionType,
        msg: &str,
    ) -> Result<(), StorageError> {
        sqlx::query("INSERT INTO transactions (datetime, user_id, value, type, message) VALUES (unixepoch(), $1, $2, $3, $4)")
            .bind(user)
            .bind(amount)
            .bind(transaction_type as i32)
            .bind(msg)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    async fn get_transactions<DT>(
        &self,
        user: i32,
        when: Option<DT>,
    ) -> Result<TransactionStream, StorageError>
    where
        DT: RangeBounds<time::PrimitiveDateTime>,
    {
        let mut query_statement = String::from(
            "SELECT datetime, user, value, type, message FROM transactions WHERE user=$1",
        );
        let mut count = 1;
        if let Some(when_range) = &when.as_ref() {
            match when_range.start_bound() {
                Bound::Included(_) => {
                    count += 1;
                    query_statement.push_str(&format!(" AND datetime >= ${count}"))
                }
                Bound::Excluded(_) => {
                    count += 1;
                    query_statement.push_str(&format!(" AND datetime > ${count}"))
                }
                Bound::Unbounded => {}
            }

            match when_range.end_bound() {
                Bound::Included(_) => {
                    count += 1;
                    query_statement.push_str(&format!(" AND datetime <= ${count}"))
                }
                Bound::Excluded(_) => {
                    count += 1;
                    query_statement.push_str(&format!(" AND datetime < ${count}"))
                }
                Bound::Unbounded => {}
            }
        };

        let query_statement = Box::into_raw(query_statement.into_boxed_str());
        let query = sqlx::query(unsafe { &*query_statement }).bind(user);

        let query = if let Some(when_range) = when {
            let query = match when_range.start_bound() {
                Bound::Included(start) => query.bind(start.clone()),
                Bound::Excluded(start) => query.bind(start.clone()),
                Bound::Unbounded => query,
            };

            match when_range.end_bound() {
                Bound::Included(end) => query.bind(end.clone()),
                Bound::Excluded(end) => query.bind(end.clone()),
                Bound::Unbounded => query,
            }
        } else {
            query
        };

        Ok(TransactionStream {
            stream: Box::new(query.fetch(&self.db).filter_map(|row| {
                row.ok()
                    .map(|row| {
                        let datetime = row.get("datetime");
                        let user_id = row.get("user_id");
                        let value = row.get("value");
                        let transaction_type = row.get::<i32, _>("type").try_into().ok()?;
                        let msg = row.get("message");
                        Some(Transaction {
                            datetime,
                            user_id,
                            value,
                            transaction_type,
                            msg,
                        })
                    })
                    .flatten()
            })),
            query: query_statement,
        })
    }
}

impl TryFrom<i32> for TransactionType {
    type Error = MissingType;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Other),
            1 => Ok(Self::Character),
            _ => Err(MissingType),
        }
    }
}

impl Drop for TransactionStream {
    fn drop(&mut self) {
        drop(unsafe { Box::from_raw(self.query) })
    }
}
