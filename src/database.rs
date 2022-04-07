use sqlx::{query_as, Executor, query, Row};

pub type Pool = sqlx::SqlitePool;

pub async fn init(pool: &Pool) -> Result<(), eyre::Report> {
    let mut conn = pool.acquire().await?;
    let version: i64 = query("PRAGMA user_version").fetch_one(&mut conn).await?.get(0);
    if version < 1 {
        conn.execute("
            CREATE TABLE comments (
                date TEXT NOT NULL,
                author TEXT NOT NULL,
                content TEXT NOT NULL,
                content_type INTEGER NOT NULL,
                page_url TEXT NOT NULL
            )").await?;
        conn.execute("CREATE INDEX comments_by_page ON comments(page_url)").await?;
    }
    Ok(())
}
