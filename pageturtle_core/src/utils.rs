pub mod date {
    use chrono::NaiveDate;
    use serde::{self, Deserialize, Deserializer};

    // TODO: accept multiple date formats
    // TODO: get custom date format from config?
    const FORMAT: &str = "%Y-%m-%d";

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
    where
        D: Deserializer<'de>,
    {
        // TODO: Improve error message when failing to parse date
        let s = String::deserialize(deserializer)?;
        NaiveDate::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}

pub fn default_true() -> bool {
    true
}

pub fn default_empty<T>() -> Vec<T> {
    vec![]
}
