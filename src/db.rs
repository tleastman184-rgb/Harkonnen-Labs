use anyhow::Result;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Row, SqlitePool,
};

use crate::config::Paths;

pub async fn init_db(paths: &Paths) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(&paths.db_file)
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS runs (
            run_id TEXT PRIMARY KEY,
            spec_id TEXT NOT NULL,
            product TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS run_events (
            event_id INTEGER PRIMARY KEY AUTOINCREMENT,
            run_id TEXT NOT NULL,
            phase TEXT NOT NULL,
            episode_id TEXT,
            agent TEXT NOT NULL,
            status TEXT NOT NULL,
            message TEXT NOT NULL,
            created_at TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    ensure_column(
        &pool,
        "run_events",
        "episode_id",
        "ALTER TABLE run_events ADD COLUMN episode_id TEXT",
    )
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS episodes (
            episode_id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL,
            phase TEXT NOT NULL,
            goal TEXT NOT NULL,
            outcome TEXT,
            confidence REAL,
            started_at TEXT NOT NULL,
            ended_at TEXT
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS causal_links (
            link_id TEXT PRIMARY KEY,
            from_event INTEGER NOT NULL REFERENCES run_events(event_id),
            to_event INTEGER NOT NULL REFERENCES run_events(event_id),
            link_type TEXT NOT NULL,
            confidence REAL NOT NULL DEFAULT 0.5,
            created_at TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS lessons (
            lesson_id TEXT PRIMARY KEY,
            source_episode TEXT REFERENCES episodes(episode_id),
            pattern TEXT NOT NULL,
            intervention TEXT,
            tags TEXT NOT NULL,
            strength REAL NOT NULL DEFAULT 1.0,
            recall_count INTEGER NOT NULL DEFAULT 0,
            last_recalled TEXT,
            created_at TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_run_events_run_id_event_id
        ON run_events (run_id, event_id)
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_causal_links_from
        ON causal_links (from_event)
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_causal_links_to
        ON causal_links (to_event)
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_lessons_tags
        ON lessons (tags)
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

async fn ensure_column(pool: &SqlitePool, table: &str, column: &str, alter_sql: &str) -> Result<()> {
    let pragma = format!("PRAGMA table_info({table})");
    let rows = sqlx::query(&pragma).fetch_all(pool).await?;
    let exists = rows
        .iter()
        .any(|row| row.get::<String, _>("name") == column);
    if !exists {
        sqlx::query(alter_sql).execute(pool).await?;
    }
    Ok(())
}
