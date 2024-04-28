use crossterm::event::{self, Event, KeyCode};
use ratatui::{prelude::*, widgets::*};
use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};
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
        sqlx::query("CREATE TABLE IF NOT EXISTS transactions (datetime INTEGER PRIMARY KEY NOT NULL, user_id INTEGER NOT NULL, value INTEGER NOT NULL, message TEXT)").execute(&db).await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS users (user_id INTEGER PRIMARY KEY NOT NULL, name TEXT UNIQUE NOT NULL)").execute(&db).await?;
        Ok(Storage { db })
    }

    async fn modify_currency(&self, user: i32, amount: i32, msg: &str) -> Result<(), StorageError> {
        sqlx::query("INSERT INTO transactions (datetime, user_id, value, message) VALUES (unixepoch(), ?, ?, ?)")
            .bind(user)
            .bind(amount)
            .bind(msg)
            .execute(&self.db)
            .await?;
        Ok(())
    }
}
