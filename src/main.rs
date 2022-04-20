#![allow(dead_code)]
#![allow(unused_imports)]

use std::net::{SocketAddr, IpAddr};
use std::path::PathBuf;

use argh::FromArgs;
use axum::{Router, Server, Extension};
use axum::routing::get;
use axum_server::tls_rustls::RustlsConfig;
use comment::new_comment;
use sqlx::sqlite::{SqliteConnectOptions, SqliteLockingMode};
use tower_http::trace::TraceLayer;
use tracing::{Instrument, info_span};
use tracing_subscriber;

use self::comment::list_comments;
use self::database::Pool;

mod comment;
mod database;
mod error;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();
    let options: Options = argh::from_env();
    let conn_opts = SqliteConnectOptions::new()
        .filename(options.db_file)
        .create_if_missing(true)
        .locking_mode(SqliteLockingMode::Exclusive);
    let pool = Pool::connect_with(conn_opts)
        .instrument(info_span!("creating connection pool")).await?;
    database::init(&pool)
        .instrument(info_span!("initializing database")).await?;
    let router = Router::new()
        .route("/comments", get(list_comments).post(new_comment))
        .layer(Extension(pool))
        .layer(TraceLayer::new_for_http());
    let addr = SocketAddr::from((options.address,options.port));
    let tls_config = RustlsConfig::from_pem_file(options.cert_file, options.key_file)
        .instrument(info_span!("loading TLS configuration")).await?;
    axum_server::bind_rustls(addr, tls_config)
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
    /// certificate file (pem format)
    #[argh(option)]
    cert_file: PathBuf,
    /// key file (pem format)
    #[argh(option)]
    key_file: PathBuf,
}
