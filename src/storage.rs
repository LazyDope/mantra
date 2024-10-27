use std::{
    fmt::Display,
    ops::{Bound, RangeBounds},
};

use async_std::stream::StreamExt;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use sqlx::{migrate::MigrateDatabase, Row, Sqlite, SqlitePool};
use strum::{Display, EnumCount, VariantNames};
use thiserror::Error;
use time::PrimitiveDateTime;

pub struct Storage {
    db: SqlitePool,
}

/// A valid user from the database
pub struct User {
    id: i32,
    name: String,
}

/// Transaction from the database
pub struct Transaction {
    pub trans_id: i32,
    pub datetime: PrimitiveDateTime,
    pub user_id: i32,
    pub value: i32,
    pub transaction_type: TransactionType,
    pub msg: String,
}

pub struct MissingType;

/// The type of a transaction, used for filtering
#[derive(Default, VariantNames, EnumCount, Clone, Copy, Display, FromPrimitive)]
pub enum TransactionType {
    #[default]
    Other = 0,
    Character,
    MissionReward,
}

#[derive(Error, Debug)]
pub enum StorageLoadError {
    #[error(transparent)]
    BaseDirs(#[from] xdg::BaseDirectoriesError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    DB(#[from] sqlx::Error),
}

#[derive(Error, Debug)]
pub enum StorageRunError {
    #[error(transparent)]
    DBError(#[from] sqlx::Error),
    #[error("Expected record could not be found")]
    RecordMissing,
}

impl Storage {
    pub async fn load_or_create() -> Result<Self, StorageLoadError> {
        let db_path = super::base_dirs()?.place_data_file("log.db")?;
        let db_url = format!("sqlite://{}", db_path.display());

        if !Sqlite::database_exists(&db_url).await.unwrap_or(false) {
            Sqlite::create_database(&db_url).await?
        };

        let db = SqlitePool::connect(&db_url).await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS transactions (\
                id INTEGER PRIMARY KEY NOT NULL,\
                datetime INTEGER NOT NULL,\
                user_id INTEGER NOT NULL,\
                value INTEGER NOT NULL,\
                type INTEGER NOT NULL,\
                message TEXT\
            )",
        )
        .execute(&db)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS users (\
                id INTEGER PRIMARY KEY NOT NULL,\
                name TEXT UNIQUE NOT NULL\
            )",
        )
        .execute(&db)
        .await?;
        Ok(Storage { db })
    }

    pub async fn add_transaction(
        &self,
        user: i32,
        amount: i32,
        transaction_type: TransactionType,
        msg: &str,
    ) -> Result<(), StorageRunError> {
        sqlx::query(
            "INSERT INTO transactions (\
                datetime, user_id,\
                value, type,\
                message\
            ) VALUES (unixepoch(), $1, $2, $3, $4)",
        )
        .bind(user)
        .bind(amount)
        .bind(transaction_type as i32)
        .bind(msg)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    pub async fn remove_transactions(&self, filter: &str) -> Result<(), StorageRunError> {
        sqlx::query(&format!("DELETE FROM transactions WHERE {filter}"))
            .execute(&self.db)
            .await?;
        Ok(())
    }

    pub async fn get_transactions<DT>(
        &self,
        user: i32,
        when: DT,
    ) -> Result<Vec<Transaction>, StorageRunError>
    where
        DT: RangeBounds<time::PrimitiveDateTime>,
    {
        let mut query_statement = String::from(
            "SELECT id, datetime, user_id, value, type, message FROM transactions WHERE user_id=$1",
        );
        let mut count = 1;
        match when.start_bound() {
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

        match when.end_bound() {
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

        let query = sqlx::query(&query_statement).bind(user);

        let query = match when.start_bound() {
            Bound::Included(start) => query.bind(start),
            Bound::Excluded(start) => query.bind(start),
            Bound::Unbounded => query,
        };

        let query = match when.end_bound() {
            Bound::Included(end) => query.bind(end),
            Bound::Excluded(end) => query.bind(end),
            Bound::Unbounded => query,
        };

        Ok(query
            .fetch(&self.db)
            .filter_map(|row| {
                row.ok().and_then(|row| {
                    let trans_id = row.get("id");
                    let datetime = row.get("datetime");
                    let user_id = row.get("user_id");
                    let value = row.get("value");
                    let transaction_type = row.get::<i32, _>("type").try_into().ok()?;
                    let msg = row.get("message");
                    Some(Transaction {
                        trans_id,
                        datetime,
                        user_id,
                        value,
                        transaction_type,
                        msg,
                    })
                })
            })
            .collect()
            .await)
    }

    /// Creates a new user, doing nothing if one already exists with the same name
    pub async fn create_user(&self, username: &str) -> Result<(), StorageRunError> {
        let insert_statement = "INSERT OR IGNORE INTO users (name) VALUES ($1)";
        let insert = sqlx::query(insert_statement).bind(username);
        insert
            .execute(&self.db)
            .await
            .expect("Should be able to insert a new user");
        Ok(())
    }

    pub async fn get_user(&self, username: &str) -> Result<User, StorageRunError> {
        let query_statement = "SELECT id, name FROM users WHERE name=$1";
        let query = sqlx::query(query_statement).bind(username);

        let user_record = query
            .fetch(&self.db)
            .next()
            .await
            .ok_or(StorageRunError::RecordMissing)??;
        Ok(User {
            id: user_record.get("id"),
            name: user_record.get("name"),
        })
    }
}

impl User {
    pub fn get_id(&self) -> i32 {
        self.id
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}

impl TransactionType {
    pub fn next(self) -> Self {
        FromPrimitive::from_isize(
            (self as isize + 1).rem_euclid(<Self as EnumCount>::COUNT as isize),
        )
        .expect("Will always be a valid i8 unless TransactionType became an empty enum")
    }

    pub fn prev(self) -> Self {
        FromPrimitive::from_isize(
            (self as isize - 1).rem_euclid(<Self as EnumCount>::COUNT as isize),
        )
        .expect("Will always be a valid i8 unless TransactionType became an empty enum")
    }
}

impl TryFrom<i32> for TransactionType {
    type Error = MissingType;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        FromPrimitive::from_i32(value).ok_or(MissingType)
    }
}

impl Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name.fmt(f)
    }
}
