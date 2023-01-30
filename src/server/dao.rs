/*
 * The DAO module contains all the necessary structs for interacting with the database
 */

use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use url::Url;
use uuid::Uuid;

#[derive(Clone, Debug)]
struct Run {
    run: RunRow,
    scm_info: ScmInfo,
    definition: RunDefinition,
}

#[derive(Clone, Debug)]
struct RunRow {
    // Unique identifier for the Run
    uuid: String,
    // User-identifiable number for the Run, monotonically increasing
    num: i64,
    // Unix status return code from the run, zero is success
    status: i64,
    // Globally resolvable URL for fetching raw logs
    log_url: String,
    definition: String,
    scm_info: String,
    created_at: NaiveDateTime,
}

/* The basic implementation for Run has all the database access operations
 */
impl Run {
    /*
     * Create the Run in the database given the appropriate struct
     */
    async fn create(run: &Run, pool: &SqlitePool) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;

        sqlx::query!(
            r#"INSERT INTO scm_info (uuid, git_url, ref, created_at) VALUES (?, ?, ?, ?)"#,
            run.scm_info.uuid,
            run.scm_info.git_url,
            run.scm_info.r#ref,
            run.scm_info.created_at
        )
        .execute(&mut tx)
        .await;

        sqlx::query!(
            r#"INSERT INTO run_definition (uuid, definition, created_at) VALUES (?, ?, ?)"#,
            run.definition.uuid,
            run.definition.definition,
            run.definition.created_at,
        )
        .execute(&mut tx)
        .await?;

        sqlx::query!(
                "INSERT INTO runs (uuid, num, status, log_url, definition, scm_info) VALUES ($1, $2, $3, $4, $5, $6)",
                run.run.uuid,
                run.run.num,
                run.run.status,
                run.run.log_url,
                run.definition.uuid,
                run.scm_info.uuid,
            )
            .execute(&mut tx)
            .await?;
        tx.commit().await
    }

    /*
     * Allow finding a Run by the given Uuid
     */
    async fn find_by(uuid: &str, pool: &SqlitePool) -> Result<Run, sqlx::Error> {
        let row = sqlx::query_as!(RunRow, "SELECT * FROM runs WHERE uuid = ?", uuid)
            .fetch_one(pool)
            .await?;
        let scm_info = sqlx::query_as!(
            ScmInfo,
            "SELECT * FROM scm_info WHERE uuid = ?",
            row.scm_info
        )
        .fetch_one(pool)
        .await?;
        let definition = sqlx::query_as!(
            RunDefinition,
            "SELECT * FROM run_definition WHERE uuid = ?",
            row.definition
        )
        .fetch_one(pool)
        .await?;

        Ok(Run {
            run: row,
            scm_info,
            definition,
        })
    }
}

impl Default for Run {
    fn default() -> Self {
        Self {
            run: RunRow::default(),
            scm_info: ScmInfo::default(),
            definition: RunDefinition::default(),
        }
    }
}

impl Default for RunRow {
    fn default() -> Self {
        Self {
            uuid: Uuid::new_v4().hyphenated().to_string(),
            num: 42,
            status: 0,
            log_url: "https://example.com/console.log".into(),
            definition: Uuid::new_v4().hyphenated().to_string(),
            scm_info: Uuid::new_v4().hyphenated().to_string(),
            created_at: Utc::now().naive_utc(),
        }
    }
}

#[derive(Clone, Debug)]
struct ScmInfo {
    uuid: String,
    git_url: String,
    r#ref: String,
    created_at: NaiveDateTime,
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

#[derive(Clone, Debug)]
struct RunDefinition {
    uuid: String,
    definition: String,
    created_at: NaiveDateTime,
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

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_database() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:")
            .await
            .expect("Failed to setup_database()");
        sqlx::migrate!()
            .run(&pool)
            .await
            .expect("Failed to run migrations in a test");
        pool
    }

    #[async_std::test]
    async fn test_create_a_run() {
        pretty_env_logger::try_init();
        let pool = setup_database().await;
        let run = Run::default();
        let result = Run::create(&run, &pool).await.unwrap();
        let fetched_run = Run::find_by(&run.run.uuid, &pool).await.unwrap();
        assert_eq!(run.run.uuid, fetched_run.run.uuid);
    }
}