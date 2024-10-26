use std::fmt::Display;

use time::PrimitiveDateTime;
use xdg::BaseDirectories;

pub mod app;
mod config;
mod serde;
mod storage;

pub struct User {
    id: i32,
    name: String,
}

pub struct MissingType;

pub struct Transaction {
    pub trans_id: i32,
    pub datetime: PrimitiveDateTime,
    pub user_id: i32,
    pub value: i32,
    pub transaction_type: TransactionType,
    pub msg: String,
}

#[derive(Default)]
pub enum TransactionType {
    #[default]
    Other = 0,
    Character = 1,
}

fn base_dirs() -> Result<BaseDirectories, xdg::BaseDirectoriesError> {
    BaseDirectories::with_prefix("mantra")
}

impl TransactionType {
    fn next(self) -> Self {
        use TransactionType::*;
        match self {
            Other => Character,
            Character => Other,
        }
    }

    fn prev(self) -> Self {
        use TransactionType::*;
        match self {
            Other => Character,
            Character => Other,
        }
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

impl Display for TransactionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TransactionType::Other => "Other",
                TransactionType::Character => "Character",
            }
        )
    }
}

#[derive(Default)]
pub struct CursoredString {
    pub text: String,
    pub index: usize,
    pub inserting: bool,
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
                index += 1;
                if index - 1 == self.index {
                    return false;
                }
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
        self.index += 1
    }
}

impl Display for CursoredString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.text.fmt(f)
    }
}
