use chrono::{NaiveDateTime, Utc};
use serde::Serialize;
use sqlx::sqlite::SqliteQueryResult;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct Project {
    pub uuid: String,
    pub name: String,
    pub created_at: NaiveDateTime,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            uuid: Uuid::new_v4().hyphenated().to_string(),
            name: "Default Project".into(),
            created_at: Utc::now().naive_utc(),
        }
    }
}

impl Project {
    pub fn new(name: &str) -> Self {
        Self {
            uuid: Uuid::new_v4().hyphenated().to_string(),
            name: name.into(),
            created_at: Utc::now().naive_utc(),
        }
    }

    pub async fn by_name(name: &str, pool: &SqlitePool) -> Result<Project, sqlx::Error> {
        sqlx::query_as!(Project, "SELECT * FROM projects WHERE name = ?", name)
            .fetch_one(pool)
            .await
    }

    pub async fn list(pool: &SqlitePool) -> Result<Vec<Project>, sqlx::Error> {
        sqlx::query_as!(Project, "SELECT * FROM projects")
            .fetch_all(pool)
            .await
    }

    pub async fn create(
        project: &Project,
        tx: &SqlitePool,
    ) -> Result<SqliteQueryResult, sqlx::Error> {
        sqlx::query!(
            r#"INSERT INTO projects (uuid, name, created_at) VALUES (?, ?, ?)"#,
            project.uuid,
            project.name,
            project.created_at,
        )
        .execute(tx)
        .await
    }
}
