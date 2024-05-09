use serde::{Deserialize, Deserializer, Serialize};
use time::macros::format_description;
use time::{OffsetDateTime, PrimitiveDateTime};

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Timestamp {
    #[serde(with = "time::serde::rfc3339")]
    Rfc3339(OffsetDateTime),
    #[serde(with = "time::serde::timestamp")]
    Unix(OffsetDateTime),
    #[serde(deserialize_with = "parse_datetime_without_timezone")]
    Rfc3339Utc(OffsetDateTime),
    UnixFloat(f64),
    Unknown(String),
}

fn parse_datetime_without_timezone<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    let format = format_description!(
        version = 2,
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond]"
    );
    let dt = PrimitiveDateTime::parse(&s, &format).map_err(serde::de::Error::custom)?;

    Ok(dt.assume_utc())
}

impl Timestamp {
    pub fn to_unix(&self) -> i64 {
        match self {
            Timestamp::Rfc3339(dt) => dt.unix_timestamp(),
            Timestamp::Rfc3339Utc(dt) => dt.unix_timestamp(),
            Timestamp::Unix(dt) => dt.unix_timestamp(),
            Timestamp::UnixFloat(dt) => *dt as i64,
            Timestamp::Unknown(x) => panic!("invalid time => {x}"),
        }
    }
}
