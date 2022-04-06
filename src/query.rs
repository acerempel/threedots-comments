use async_graphql::{EmptyMutation, EmptySubscription};
use graphql::Context;
use sqlx::query_as;

use crate::database::Pool;
use crate::comment::Comment;

pub struct Query;

pub type Schema = graphql::Schema<Query, EmptyMutation, EmptySubscription>;

#[graphql::Object]
impl Query {
    async fn comments(&self, ctx: &Context<'_>, post_url: String) -> graphql::Result<Vec<Comment>> {
        let pool = ctx.data::<Pool>()?;
        let mut conn = pool.acquire().await?;
        let comments = query_as(
            "SELECT author, date, content_type, content FROM comments"
        ).fetch_all(&mut conn).await?;
        Ok(comments)
    }
}