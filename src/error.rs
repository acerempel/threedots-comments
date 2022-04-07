use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::marker::{Sync, Send};
use std::error::Error as StdError;

pub struct Error(eyre::Report);

impl<E: 'static + StdError + Sync + Send> From<E> for Error {
    fn from(error: E) -> Self {
        Self(error.into())
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_string()).into_response()
    }
}
