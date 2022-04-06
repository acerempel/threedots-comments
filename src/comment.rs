use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::Extension;
use sqlx::sqlite::{SqliteRow, SqliteValueRef, SqliteTypeInfo};
use sqlx::{FromRow, Row, Sqlite, Decode};

use crate::query::Schema;

#[derive(graphql::SimpleObject)]
pub struct Comment {
    author: String,
    date: String,
    content: String,
    content_type: ContentType,
}

#[derive(Copy, Clone, Eq, PartialEq, graphql::Enum)]
enum ContentType {
    Plain, Html,
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
        };
        Ok(comment)
    }
}