use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Timestamp {
    #[serde(with = "time::serde::rfc3339")]
    Rfc3339(OffsetDateTime),
    #[serde(with = "time::serde::timestamp")]
    Unix(OffsetDateTime),
    UnixFloat(f64),
    Other(String),
}

impl Timestamp {
    pub fn to_unix(&self) -> i64 {
        match self {
            Timestamp::Rfc3339(dt) => dt.unix_timestamp(),
            Timestamp::Unix(dt) => dt.unix_timestamp(),
            Timestamp::UnixFloat(dt) => *dt as i64,
            Timestamp::Other(dt) => panic!("found => {}", dt),
        }
    }
}
