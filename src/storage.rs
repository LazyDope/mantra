//! This module interfaces with the local sqlite database
use std::{
    fmt::Display,
    marker::PhantomData,
    ops::{Bound, RangeBounds},
};

use async_std::stream::StreamExt;
use sqlx::{migrate::MigrateDatabase, QueryBuilder, Row, Sqlite, SqlitePool, Type};
use strum::{Display, EnumCount, FromRepr, VariantNames};
use thiserror::Error;
use time::PrimitiveDateTime;

/// Wrapper for the sqlite database
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

/// Types of Filters usable for queries
pub enum TransactionFilter {
    UserId(i32),
    Type(TransactionType),
    DateRange(DateRange),
    Id(i32),
    And(Vec<TransactionFilter>),
    Or(Vec<TransactionFilter>),
    Not(Box<TransactionFilter>),
}

/// Allows storing a range because RangeBound is not dyn compatible
pub struct DateRange {
    start: Bound<time::PrimitiveDateTime>,
    end: Bound<time::PrimitiveDateTime>,
}

/// Error that may occur when converting type id to the enum variant
#[derive(Error)]
pub struct MissingVariant<T, U>(T, PhantomData<U>);

/// The type of a transaction, used for filtering
#[derive(Default, VariantNames, EnumCount, Clone, Copy, Display, FromRepr, Type)]
#[repr(i32)]
pub enum TransactionType {
    #[default]
    Other = 0,
    Character,
    MissionReward,
}

/// Possible errors that may occur when first loading the db from the sqlite file
#[derive(Error, Debug)]
pub enum StorageLoadError {
    #[error(transparent)]
    BaseDirs(#[from] xdg::BaseDirectoriesError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    DB(#[from] sqlx::Error),
}

/// Possible errors that may occur when accessing the active db
#[derive(Error, Debug)]
pub enum StorageRunError {
    #[error(transparent)]
    DBError(#[from] sqlx::Error),
    #[error("Expected record could not be found")]
    RecordMissing,
}

impl Storage {
    /// Load the db from known location, or create new with table set up
    pub async fn load_or_create() -> Result<Self, StorageLoadError> {
        let db_path = super::base_dirs()?.place_data_file("log.db")?;
        let db_url = format!("sqlite://{}", db_path.display());

        if !Sqlite::database_exists(&db_url).await.unwrap_or(false) {
            Sqlite::create_database(&db_url).await?
        };

        let db = SqlitePool::connect(&db_url).await?;

        // transaction table, all rows must be filled and non-null except the message
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

        // user table, usernames must be unique, but still better to identify by an id internally
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

    /// Adds a new transaction to the database using the current time
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

    /// Removes all transactions that match a filter.
    /// Do not pass user input directly into this function.
    pub async fn remove_transactions(
        &self,
        filter: TransactionFilter,
    ) -> Result<(), StorageRunError> {
        let mut query_builder = QueryBuilder::new("DELETE FROM transactions WHERE ");
        filter.add_to_builder(&mut query_builder);

        let query = query_builder.build();

        query.execute(&self.db).await?;
        Ok(())
    }

    /// Get all transactions for a user within a date range
    pub async fn get_transactions(
        &self,
        filter: TransactionFilter,
    ) -> Result<Vec<Transaction>, StorageRunError> {
        let mut query_builder = QueryBuilder::new(
            "SELECT id, datetime, user_id, value, type, message FROM transactions WHERE ",
        );

        filter.add_to_builder(&mut query_builder);

        let query = query_builder.build();

        Ok(query
            .fetch(&self.db)
            .filter_map(|row| {
                row.ok().map(|row| Transaction {
                    trans_id: row.get("id"),
                    datetime: row.get("datetime"),
                    user_id: row.get("user_id"),
                    value: row.get("value"),
                    transaction_type: row.get("type"),
                    msg: row.get("message"),
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

    /// Gets a user if they exist, otherwise errors
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
    /// Returns the table id of the user
    pub fn get_id(&self) -> i32 {
        self.id
    }

    /// Returns a string slice of the username
    pub fn get_name(&self) -> &str {
        &self.name
    }
}

impl TransactionType {
    /// Returns the next type of transaction from the enum
    pub fn next(self) -> Self {
        Self::from_repr((self as i32 + 1).rem_euclid(<Self as EnumCount>::COUNT as i32))
            .expect("TransactionType is non-zero count so will always succeed")
    }

    /// Returns the previous type of transaction from the enum
    pub fn prev(self) -> Self {
        Self::from_repr((self as i32 - 1).rem_euclid(<Self as EnumCount>::COUNT as i32))
            .expect("TransactionType is non-zero count so will always succeed")
    }
}

impl TransactionFilter {
    pub fn add_to_builder<'s, 'args>(&'s self, builder: &mut QueryBuilder<'args, Sqlite>)
    where
        's: 'args,
    {
        match self {
            TransactionFilter::UserId(id) => {
                builder.push("user_id = ").push_bind(id);
            }
            TransactionFilter::Type(transaction_type) => {
                builder.push("type = ").push_bind(transaction_type);
            }
            TransactionFilter::DateRange(date_range) => {
                let mut separated = builder.separated(" AND ");
                match date_range.start {
                    Bound::Included(start) => {
                        separated.push("datetime >= ").push_bind_unseparated(start);
                    }
                    Bound::Excluded(start) => {
                        separated.push("datetime > ").push_bind_unseparated(start);
                    }
                    Bound::Unbounded => {}
                }
                match date_range.end {
                    Bound::Included(end) => {
                        separated.push("datetime <= ").push_bind_unseparated(end);
                    }
                    Bound::Excluded(end) => {
                        separated.push("datetime < ").push_bind_unseparated(end);
                    }
                    Bound::Unbounded => {
                        separated.push("1=1");
                    }
                }
            }
            TransactionFilter::And(filters) => {
                let mut push_sep = false;
                builder.push("(");
                for filter in filters {
                    if push_sep {
                        builder.push(") AND (");
                    } else {
                        push_sep = true;
                    }
                    filter.add_to_builder(builder);
                }
                builder.push(")");
            }
            TransactionFilter::Or(filters) => {
                let mut push_sep = false;
                builder.push("(");
                for filter in filters {
                    if push_sep {
                        builder.push(") OR (");
                    } else {
                        push_sep = true
                    }
                    filter.add_to_builder(builder);
                }
                builder.push(")");
            }
            TransactionFilter::Not(filter) => {
                builder.push("NOT (");
                filter.add_to_builder(builder);
                builder.push(")");
            }
            TransactionFilter::Id(id) => {
                builder.push("id = ").push_bind(id);
            }
        };
    }
}

impl TryFrom<i32> for TransactionType {
    type Error = MissingVariant<i32, Self>;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        Self::from_repr(value).ok_or(MissingVariant(value, PhantomData))
    }
}

impl From<TransactionType> for i32 {
    fn from(value: TransactionType) -> Self {
        value as i32
    }
}

impl Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name.fmt(f)
    }
}

impl<T, U> Display for MissingVariant<T, U>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Missing Variant for: {} in {}",
            self.0,
            std::any::type_name::<U>()
        )
    }
}

impl<T> From<T> for DateRange
where
    T: RangeBounds<time::PrimitiveDateTime>,
{
    fn from(value: T) -> Self {
        Self {
            start: value.start_bound().cloned(),
            end: value.end_bound().cloned(),
        }
    }
}
