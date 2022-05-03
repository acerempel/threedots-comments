use sqlx::{Executor, query, Row, Acquire};
use uuid::Uuid;

pub type Pool = sqlx::SqlitePool;

pub async fn init(pool: &Pool) -> Result<(), eyre::Report> {
    let mut conn = pool.acquire().await?;
    let version: i64 = query("PRAGMA user_version").fetch_one(&mut conn).await?.get(0);
    if version < 1 {
        conn.execute("
            CREATE TABLE pages (
                id INTEGER PRIMARY KEY,
                url TEXT UNIQUE NOT NULL
            );
            CREATE TABLE comments (
                date TEXT NOT NULL,
                author TEXT NOT NULL,
                content TEXT NOT NULL,
                content_type INTEGER NOT NULL,
                page_id INTEGER NOT NULL REFERENCES pages(id)
            );
            CREATE INDEX comments_by_page ON comments(page_id);
            PRAGMA user_version = 1;
        ").await?;
    }
    if version < 2 {
        conn.execute("
            ALTER TABLE comments DROP COLUMN content_type;
            PRAGMA user_version = 2;
        ").await?;
    }
    if version < 3 {
        let mut txn = conn.begin().await?;
        txn.execute("
            ALTER TABLE comments ADD COLUMN id TEXT;
            CREATE UNIQUE INDEX comments_id ON comments(id);
        ").await?;
        let rowids = txn.fetch_all("SELECT rowid FROM comments").await?;
        for row in rowids {
            let rowid: i64 = row.get(0);
            let id = Uuid::new_v4().to_string();
            query("UPDATE comments SET id = ? WHERE rowid = ?")
                .bind(&id).bind(rowid)
                .execute(&mut txn).await?;
        }
        txn.execute("PRAGMA user_version = 3;").await?;
        txn.commit().await?;
    }
    Ok(())
}
