use axum::headers::{Origin, AccessControlAllowOrigin};
use axum::response::{Html, IntoResponse};
use axum::{Extension, Json, TypedHeader};
use axum::extract::Query;
use axum_macros::debug_handler;
use chrono::{Utc, DateTime};
use serde::{Serialize, Deserialize};
use sqlx::sqlite::{SqliteRow, SqliteValueRef, SqliteTypeInfo};
use sqlx::{FromRow, Row, Sqlite, query_as, query, TypeInfo};
use tracing::{instrument, info};

use crate::database::Pool;
use crate::error::Error;

#[derive(Debug, Serialize)]
pub struct Comment {
    author: String,
    date: DateTime<Utc>,
    content: String,
    page_url: String,
}

#[derive(Debug, Deserialize)]
pub struct NewComment {
    author: String,
    content: String,
    content_type: ContentType,
    page_url: String,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
enum ContentType {
    #[serde(rename = "plain")]
    Plain,
    #[serde(rename = "html")]
    Html,
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

    fn compatible(ty: &<Sqlite as sqlx::Database>::TypeInfo) -> bool {
        ! ty.is_null() && ty.name() == "INTEGER"
    }
}

impl FromRow<'_, SqliteRow> for Comment {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        let comment = Self {
            author: row.get("author"),
            date: row.get("date"),
            content: row.get("content"),
            page_url: row.get("page_url"),
        };
        Ok(comment)
    }
}

pub(crate) enum CommentResponse {
    Json(Json<Vec<Comment>>),
    Html(Html<String>),
}

impl IntoResponse for CommentResponse {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Json(json) => json.into_response(),
            Self::Html(html) => html.into_response(),
        }
    }
}

#[debug_handler]
pub(crate) async fn list_comments(
    pool: Extension<Pool>, TypedHeader(origin): TypedHeader<Origin>, data: Query<CommentRequest>
) -> Result<impl IntoResponse, Error> {
    let mut conn = pool.acquire().await?;
    let comments = query_as("
        SELECT author, date, content, url as page_url
        FROM comments JOIN pages ON comments.page_id = pages.id
        WHERE url = ?
    ").bind(data.page_url.as_str()).fetch_all(&mut conn).await?;
    let response = CommentResponse::Json(Json(comments));
    let acao_header = access_control_header(origin);
    Ok((TypedHeader(acao_header), response))
}

fn access_control_header(origin: Origin) -> AccessControlAllowOrigin {
    if origin.hostname() == "threedots.ca" || origin.hostname().ends_with(".threedots.ca") {
        AccessControlAllowOrigin::try_from(format!("{}://{}", origin.scheme(), origin.hostname()).as_str()).unwrap()
    } else if origin.hostname() == "reverent-euclid-2bfb78.netlify.app" || origin.hostname().ends_with("--reverent-euclid-2bfb78.netlify.app") {
        AccessControlAllowOrigin::try_from(format!("{}://{}", origin.scheme(), origin.hostname()).as_str()).unwrap()
    } else {
        AccessControlAllowOrigin::NULL
    }
}

#[derive(Deserialize, Debug)]
pub struct CommentRequest {
    page_url: String,
}

#[debug_handler]
pub(crate) async fn new_comment(
    pool: Extension<Pool>, TypedHeader(origin): TypedHeader<Origin>, Json(mut comment): Json<NewComment>
) -> Result<impl IntoResponse, Error> {
    if comment.content_type == ContentType::Plain {
        comment.content = format!("<p>{}</p>", html_escape::encode_text(&comment.content));
        comment.content_type = ContentType::Html;
    }
    let mut conn = pool.acquire().await?;
    let page_id: i64 = query("INSERT INTO pages (url) VALUES (?) ON CONFLICT (url) DO NOTHING RETURNING id")
        .bind(&comment.page_url)
        .fetch_one(&mut conn).await?
        .get(0);
    let date = Utc::now();
    query( "INSERT INTO comments (author, date, content, page_id)
            VALUES (?, ?, ?, ?)")
        .bind(&comment.author).bind(&date)
        .bind(&comment.content)
        .bind(page_id)
        .execute(&mut conn).await?;
    Ok((TypedHeader(access_control_header(origin)), ()))
}
