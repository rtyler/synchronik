use sqlx::SqlitePool;

use crate::models::*;

#[derive(Clone, Debug)]
pub struct Run {
    run: RunRow,
    project: Project,
    scm_info: ScmInfo,
    definition: RunDefinition,
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
        .await?;

        sqlx::query!(
            r#"INSERT INTO run_definition (uuid, definition, created_at) VALUES (?, ?, ?)"#,
            run.definition.uuid,
            run.definition.definition,
            run.definition.created_at,
        )
        .execute(&mut tx)
        .await?;

        sqlx::query!(
                "INSERT INTO runs (uuid, num, status, log_url, definition, scm_info, project) VALUES (?, ?, ?, ?, ?, ?, ?)",
                run.run.uuid,
                run.run.num,
                run.run.status,
                run.run.log_url,
                run.definition.uuid,
                run.scm_info.uuid,
                run.project.uuid,
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

        let project = sqlx::query_as!(
            Project,
            "SELECT * FROM projects WHERE uuid = ?",
            row.project
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
            project,
            definition,
        })
    }
}

impl Default for Run {
    fn default() -> Self {
        Self {
            run: RunRow::default(),
            project: Project::default(),
            scm_info: ScmInfo::default(),
            definition: RunDefinition::default(),
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
        let _ = pretty_env_logger::try_init();
        let pool = setup_database().await;
        let project = crate::models::Project::new("test");
        Project::create(&project, &pool).await.unwrap();

        let mut run = Run::default();
        run.project = project;
        let _result = Run::create(&run, &pool).await.unwrap();
        let fetched_run = Run::find_by(&run.run.uuid, &pool).await.unwrap();
        assert_eq!(run.run.uuid, fetched_run.run.uuid);
    }
}
