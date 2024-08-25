use time::PrimitiveDateTime;
use xdg::BaseDirectories;

pub mod app;
mod config;
mod storage;

pub struct User {
    id: i32,
    name: String,
}

pub struct MissingType;

pub struct Transaction {
    pub datetime: PrimitiveDateTime,
    pub user_id: i32,
    pub value: i32,
    pub transaction_type: TransactionType,
    pub msg: String,
}

pub enum TransactionType {
    Other = 0,
    Character = 1,
}

fn base_dirs() -> Result<BaseDirectories, xdg::BaseDirectoriesError> {
    BaseDirectories::with_prefix("mantra")
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
