#![allow(dead_code)]
#![allow(unused_imports)]

extern crate async_graphql as graphql;

use std::net::SocketAddr;

use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{Router, Server, Extension};
use axum::routing::get;
use axum_macros::debug_handler;
use graphql::{EmptyMutation, EmptySubscription};
use sqlx::sqlite::{SqliteConnectOptions, SqliteLockingMode};

use self::database::Pool;
use self::query::{Query, Schema};

mod comment;
mod query;
mod database;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let options = SqliteConnectOptions::new()
        .filename("potato.db")
        .create_if_missing(true)
        .locking_mode(SqliteLockingMode::Exclusive);
    let pool = Pool::connect_with(options).await?;
    let schema = graphql::Schema::build(Query, EmptyMutation, EmptySubscription)
        .data(pool)
        .finish();
    let router = Router::new()
        .route("/", get(graphql))
        .layer(Extension(schema));
    let addr = SocketAddr::from(([127,0,0,1],4000));
    Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;
    Ok(())
}

#[debug_handler]
async fn graphql(request: GraphQLRequest, schema: Extension<Schema>) -> GraphQLResponse {
    schema.execute(request.into_inner()).await.into()
}