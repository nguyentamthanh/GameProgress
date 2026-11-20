use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

use crate::AppState;

pub fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://studio:studio@127.0.0.1:5432/studio".to_string())
}

pub async fn connect() -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url())
        .await
}

pub async fn ensure_schema(pool: &PgPool) -> Result<(), sqlx::Error> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM information_schema.tables
            WHERE table_schema = 'public' AND table_name = 'studios'
        )",
    )
    .fetch_one(pool)
    .await?;
    if exists {
        return Ok(());
    }
    sqlx::raw_sql(include_str!(
        "../../../migrations/0001_studio_user_game.sql"
    ))
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn ensure_dev_actor(pool: &PgPool) -> Result<AppState, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(741_932_i64)
        .execute(&mut *tx)
        .await?;

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM studios")
        .fetch_one(&mut *tx)
        .await?;

    if count == 0 {
        let studio_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();
        sqlx::query("INSERT INTO studios (id, name, plan) VALUES ($1, 'Local Studio', 'free')")
            .bind(studio_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO users (id, studio_id, email, name, studio_role)
             VALUES ($1, $2, 'owner@local', 'Owner', 'admin')",
        )
        .bind(user_id)
        .bind(studio_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        return Ok(AppState {
            pool: pool.clone(),
            studio_id,
            user_id,
        });
    }

    let (studio_id, user_id) = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT studio_id, id
         FROM users
         WHERE email = 'owner@local' AND deleted_at IS NULL
         ORDER BY updated_at ASC
         LIMIT 1",
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(AppState {
        pool: pool.clone(),
        studio_id,
        user_id,
    })
}
