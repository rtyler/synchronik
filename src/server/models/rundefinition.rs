use chrono::{NaiveDateTime, Utc};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct RunDefinition {
    pub uuid: String,
    pub definition: String,
    pub created_at: NaiveDateTime,
}

impl Default for RunDefinition {
    fn default() -> Self {
        Self {
            uuid: Uuid::new_v4().hyphenated().to_string(),
            definition: String::new(),
            created_at: Utc::now().naive_utc(),
        }
    }
}
