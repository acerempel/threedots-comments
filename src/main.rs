#![allow(dead_code)]
#![allow(unused_imports)]

use std::net::{SocketAddr, IpAddr};
use std::path::PathBuf;

use argh::FromArgs;
use axum::{Router, Server, Extension};
use axum::routing::get;
use sqlx::sqlite::{SqliteConnectOptions, SqliteLockingMode};

use self::comment::list_comments;
use self::database::Pool;

mod comment;
mod database;
mod error;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let options: Options = argh::from_env();
    let conn_opts = SqliteConnectOptions::new()
        .filename(options.db_file)
        .create_if_missing(true)
        .locking_mode(SqliteLockingMode::Exclusive);
    let pool = Pool::connect_with(conn_opts).await?;
    database::init(&pool).await?;
    let router = Router::new()
        .route("/comments", get(list_comments))
        .layer(Extension(pool));
    let addr = SocketAddr::from((options.address,options.port));
    Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;
    Ok(())
}

/// Backend for threedots
#[derive(FromArgs)]
struct Options {
    /// addresss to listen on
    #[argh(option, default = "IpAddr::from([127,0,0,1])")]
    address: IpAddr,
    /// port to listen on
    #[argh(option, default = "3000")]
    port: u16,
    /// database filename
    #[argh(option, default = "PathBuf::from(\"threedots.db\")")]
    db_file: PathBuf,
}
