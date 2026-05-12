use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::sync::broadcast;

pub struct AppState {
    pub rtcm_tx: broadcast::Sender<Vec<u8>>,
    pub nmea_tx: broadcast::Sender<String>,
    pub file_lock: Mutex<()>,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Item not found")]
    NotFound,
    #[error("Internal server error: {0}")]
    InternalError(#[from] anyhow::Error), // Example for generic errors
}

// Implement IntoResponse for the custom error type
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()).into_response(),
            AppError::InternalError(ref err) => {
                // Log the error detail here if desired
                eprintln!("Internal error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "An internal error occurred",
                )
                    .into_response()
            }
        }
    }
}
