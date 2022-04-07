#![allow(dead_code)]
#![allow(unused_imports)]

use std::net::SocketAddr;

use axum::{Router, Server, Extension};
use axum::routing::get;
use sqlx::sqlite::{SqliteConnectOptions, SqliteLockingMode};

use self::comment::comments;
use self::database::Pool;

mod comment;
mod database;
mod error;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let options = SqliteConnectOptions::new()
        .filename("potato.db")
        .create_if_missing(true)
        .locking_mode(SqliteLockingMode::Exclusive);
    let pool = Pool::connect_with(options).await?;
    let router = Router::new()
        .route("/comments", get(list_comments))
        .layer(Extension(pool));
    let addr = SocketAddr::from(([127,0,0,1],4000));
    Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;
    Ok(())
}
