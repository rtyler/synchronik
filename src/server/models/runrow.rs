use chrono::{NaiveDateTime, Utc};
use uuid::Uuid;

/*
 * The RunRow is the struct for the deserialization/serialization of the runs table
 * unfortunately this is a little bit of misdirection due to the inability to make
 * nested structs with sqlx work well
 */
#[derive(Clone, Debug)]
pub struct RunRow {
    // Unique identifier for the Run
    pub uuid: String,
    // User-identifiable number for the Run, monotonically increasing
    pub num: i64,
    // Unix status return code from the run, zero is success
    pub status: i64,
    // Globally resolvable URL for fetching raw logs
    pub log_url: String,
    // Foreign key to projects
    pub project: String,
    // Foreign key to run_definition
    pub definition: String,
    // Foreign key to scm_info
    pub scm_info: String,
    pub created_at: NaiveDateTime,
}

impl Default for RunRow {
    fn default() -> Self {
        Self {
            uuid: Uuid::new_v4().hyphenated().to_string(),
            num: 42,
            status: 0,
            log_url: "https://example.com/console.log".into(),
            definition: Uuid::new_v4().hyphenated().to_string(),
            project: Uuid::new_v4().hyphenated().to_string(),
            scm_info: Uuid::new_v4().hyphenated().to_string(),
            created_at: Utc::now().naive_utc(),
        }
    }
}
