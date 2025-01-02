use core::{
    fmt::{self, Formatter},
    ops::{Bound, RangeBounds},
};

use itertools::Itertools;
use sqlx::{QueryBuilder, Sqlite};

use super::TransactionTypeMap;

/// Types of Filters usable for queries
#[derive(Clone)]
pub enum TransactionFilter {
    UserId(Vec<i32>),
    Type(TransactionTypeMap<bool>),
    DateRange(DateRange),
    Id(Vec<i32>),
    Not(Box<TransactionFilter>),
}

/// Allows storing a range because RangeBound is not dyn compatible
#[derive(Clone)]
pub struct DateRange {
    start: Bound<time::PrimitiveDateTime>,
    end: Bound<time::PrimitiveDateTime>,
}

impl TransactionFilter {
    pub fn add_to_builder(&self, builder: &mut QueryBuilder<'_, Sqlite>) {
        match self {
            TransactionFilter::UserId(ids) => {
                builder.push("user_id = ").push_bind(ids[0]);
                for id in &ids[1..] {
                    builder.push(" OR user_id = ").push_bind(*id);
                }
            }
            TransactionFilter::Type(transaction_types) => {
                let mut iter = transaction_types
                    .kv_pairs()
                    .filter(|(_, selected)| **selected);
                if let Some((tran_type, _)) = iter.next() {
                    builder.push("type = ").push_bind(tran_type);
                    for (transaction_type, _) in iter {
                        builder.push(" OR type = ").push_bind(transaction_type);
                    }
                }
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
            TransactionFilter::Not(filter) => {
                builder.push("NOT (");
                filter.add_to_builder(builder);
                builder.push(")");
            }
            TransactionFilter::Id(ids) => {
                builder.push("id = ").push_bind(ids[0]);
                for id in &ids[1..] {
                    builder.push(" OR id = ").push_bind(*id);
                }
            }
        };
    }

    pub fn get_useful(self) -> Option<TransactionFilter> {
        if self.is_useful() {
            Some(self)
        } else {
            None
        }
    }

    fn is_useful(&self) -> bool {
        match self {
            TransactionFilter::UserId(ids) => !ids.is_empty(),
            TransactionFilter::Type(transaction_type_map) => {
                transaction_type_map.values().contains(&true)
            }
            TransactionFilter::DateRange(date_range) => {
                !(matches!(date_range.start, Bound::Unbounded)
                    && matches!(date_range.end, Bound::Unbounded))
            }
            TransactionFilter::Id(ids) => !ids.is_empty(),
            TransactionFilter::Not(transaction_filter) => transaction_filter.is_useful(),
        }
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

impl std::fmt::Display for DateRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.start {
            Bound::Included(inclusive) => write!(f, "[{}", inclusive)?,
            Bound::Excluded(exclusive) => write!(f, "({}", exclusive)?,
            Bound::Unbounded => write!(f, "(")?,
        }
        write!(f, "-")?;
        match self.end {
            Bound::Included(inclusive) => write!(f, "{}]", inclusive),
            Bound::Excluded(exclusive) => write!(f, "{})", exclusive),
            Bound::Unbounded => write!(f, ")"),
        }
    }
}
