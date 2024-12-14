use core::{
    fmt::{self, Formatter},
    ops::{Bound, RangeBounds},
};

use sqlx::{QueryBuilder, Sqlite};

use super::TransactionType;

/// Types of Filters usable for queries
#[derive(Clone)]
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
#[derive(Clone)]
pub struct DateRange {
    start: Bound<time::PrimitiveDateTime>,
    end: Bound<time::PrimitiveDateTime>,
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
