use chrono::{NaiveDateTime, Utc};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct ScmInfo {
    pub uuid: String,
    pub git_url: String,
    pub r#ref: String,
    pub created_at: NaiveDateTime,
}

impl Default for ScmInfo {
    fn default() -> Self {
        Self {
            uuid: Uuid::new_v4().hyphenated().to_string(),
            git_url: "https://example.com/some/repo.git".into(),
            r#ref: "main".into(),
            created_at: Utc::now().naive_utc(),
        }
    }
}
