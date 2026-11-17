use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("activity name is empty")]
    EmptyName,
    #[error("could not understand duration {0:?}")]
    InvalidDuration(String),
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("the data file is from a newer version of tally")]
    NewerDatabase,
    #[error("could not serialize data: {0}")]
    Json(#[from] serde_json::Error),
    #[error("could not write CSV: {0}")]
    Csv(#[from] csv::Error),
    #[error("time calculation failed: {0}")]
    Time(#[from] jiff::Error),
    #[error("activity {0} does not exist")]
    UnknownActivity(i64),
}
