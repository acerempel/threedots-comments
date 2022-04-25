use std::net::{SocketAddr, IpAddr};
use std::path::PathBuf;
use std::str::FromStr;

use argh::FromArgs;
use axum::{Router, Extension};
use axum::routing::get;
use axum_server::tls_rustls::RustlsConfig;
use sqlx::sqlite::{SqliteConnectOptions, SqliteLockingMode};
use tower_http::trace::TraceLayer;
use tracing::metadata::LevelFilter;
use tracing::{Instrument, info_span};
use tracing_subscriber::filter::Targets;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber;
use tracing_subscriber::util::SubscriberInitExt;

use self::comment::{list_comments, new_comment};
use self::database::Pool;

mod comment;
mod database;
mod error;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let options: Options = argh::from_env();

    let journald_layer =
        (options.log_dest == Logging::Journald)
        .then(tracing_journald::layer).transpose()?;
    let stdout_layer =
        (options.log_dest == Logging::Fmt)
        .then(|| tracing_subscriber::fmt::layer()
            .with_span_events(FmtSpan::CLOSE)
            .map_event_format(|ef| ef.compact()));
    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(journald_layer)
        .with(options.filter_logs)
        // This method also sets up the subscriber to consume `log` records
        .try_init()?;

    let conn_opts = SqliteConnectOptions::new()
        .filename(options.db_file)
        .create_if_missing(true)
        .locking_mode(SqliteLockingMode::Exclusive);
    let pool = Pool::connect_with(conn_opts)
        .instrument(info_span!("creating connection pool")).await?;
    database::init(&pool)
        .instrument(info_span!("initializing database")).await?;

    let service = Router::new()
        .route("/comments", get(list_comments).post(new_comment))
        .layer(Extension(pool))
        .layer(TraceLayer::new_for_http())
        .into_make_service();
    let addr = SocketAddr::from((options.address,options.port));

    if let (Some(cert_file), Some(key_file)) = (options.cert_file, options.key_file) {
        let tls_config = RustlsConfig::from_pem_file(cert_file, key_file)
            .instrument(info_span!("loading TLS configuration")).await?;
        axum_server::bind_rustls(addr, tls_config).serve(service).await?;
    } else {
        if ! cfg!(debug_assertions) {
            eyre::bail!("TLS certificate is required in release mode!");
        }
        axum::Server::bind(&addr).serve(service).await?;
    }

    Ok(())
}

#[derive(Eq, PartialEq)]
enum Logging { Fmt, Journald, }

impl FromStr for Logging {
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "stdout" => Ok(Self::Fmt),
            "journald" => Ok(Self::Journald),
            _ => Err(eyre::eyre!("not a kind of logging: {}", s))
        }
    }

    type Err = eyre::Report;
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
    cert_file: Option<PathBuf>,

    /// key file (pem format)
    #[argh(option)]
    key_file: Option<PathBuf>,

    /// log to where (journald, stdout)
    #[argh(option, default = "Logging::Fmt")]
    log_dest: Logging,

    /// filter logs
    #[argh(option, default = "Targets::new().with_default(LevelFilter::INFO)")]
    filter_logs: Targets,
}
