pub mod date {
    use chrono::{NaiveDate, TimeZone, Utc};
    use serde::{self, Deserialize, Deserializer};

    // TODO: accept multiple date formats
    // TODO: get custom date format from config?
    const FORMAT: &str = "%Y-%m-%dT%H:%M:%SZ";

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Utc.datetime_from_str(&s, FORMAT)
            .map(|d| d.naive_utc().date())
            .map_err(serde::de::Error::custom)
    }
}

pub fn default_true() -> bool {
    true
}

pub fn default_empty<T>() -> Vec<T> {
    vec![]
}
