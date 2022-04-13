use axum::{Extension, Json};
use axum::extract::Query;
use axum_macros::debug_handler;
use serde::{Serialize, Deserialize};
use sqlx::sqlite::{SqliteRow, SqliteValueRef, SqliteTypeInfo};
use sqlx::{FromRow, Row, Sqlite, Decode, query_as, Executor, query};

use crate::database::Pool;
use crate::error::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct Comment {
    author: String,
    date: String,
    content: String,
    content_type: ContentType,
    page_url: String,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
enum ContentType {
    Plain, Html,
}

impl<'r> sqlx::Encode<'r, Sqlite> for ContentType {
    fn encode_by_ref(&self, buf: &mut <Sqlite as sqlx::database::HasArguments<'r>>::ArgumentBuffer) -> sqlx::encode::IsNull {
        let int: i64 = match self {
            &ContentType::Plain => 0,
            &ContentType::Html => 1,
        };
        int.encode_by_ref(buf)
    } 
}

impl<'r> sqlx::Decode<'r, Sqlite> for ContentType {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        match i64::decode(value)? {
            0 => Ok(Self::Plain),
            1 => Ok(Self::Html),
            _ => Err(eyre::eyre!("bad value!").into())
        }
    }
}

impl sqlx::Type<Sqlite> for ContentType {
    fn type_info() -> SqliteTypeInfo {
        i64::type_info()
    }
}

impl FromRow<'_, SqliteRow> for Comment{
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        let comment = Self {
            author: row.get("author"),
            date: row.get("date"),
            content: row.get("content"),
            content_type: row.get("content_type"),
            page_url: row.get("page_url"),
        };
        Ok(comment)
    }
}

#[debug_handler]
pub async fn list_comments(pool: Extension<Pool>, page_url: Query<String>) -> Result<Json<Vec<Comment>>, Error> {
    let mut conn = pool.acquire().await?;
    let comments = query_as("
        SELECT author, date, content_type, content, url as page_url
        FROM comments JOIN pages ON comments.page_id = pages.id
        WHERE url = ?
    ").bind(page_url.as_str()).fetch_all(&mut conn).await?;
    Ok(Json(comments))
}

#[debug_handler]
pub async fn new_comment(pool: Extension<Pool>, comment: Json<Comment>) -> Result<(), Error> {
    let mut conn = pool.acquire().await?;
    let page_id: i64 = query("INSERT INTO pages (url) VALUES ? ON CONFLICT (url) DO NOTHING RETURNING id")
        .bind(&comment.page_url)
        .fetch_one(&mut conn)
        .await?
        .get(0);
    query( "INSERT INTO comments (author, date, content_type, content, page_id)
            VALUES (?, ?, ?, ?, ?)")
        .bind(&comment.author).bind(&comment.date)
        .bind(&comment.content_type).bind(&comment.content)
        .bind(page_id)
        .execute(&mut conn)
        .await?;
    Ok(())
}
