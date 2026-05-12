use std::sync::Arc;

use axum::{Json, extract::State};
use serde_json::{Value, json};

use crate::{appstate, mynmea::parse_nmea::parse_nmea};

pub async fn get_current_latlng(
    State(state): State<Arc<appstate::AppState>>,
) -> Result<Json<Value>, appstate::AppError> {
    let mut broadcast_rx = state.nmea_tx.subscribe();

    // 3. TASK 1: The "Writing" Task
    // This background task listens for messages on the global broadcast channel
    // and pushes them down the WebSocket to the client.

    while let Ok(msg) = broadcast_rx.recv().await {
        if let Ok(nmea_messages) = serde_json::from_str::<Vec<String>>(&msg) {
            match parse_nmea(nmea_messages).await {
                Ok(live_status) => {
                    if let (Some(_), Some(_)) = (live_status.latitude, live_status.longitude) {
                        return Ok(Json(json!(live_status)));
                    } else {
                        return Err(appstate::AppError::NotFound);
                    }
                }
                Err(e) => eprintln!("Failed to parse NMEA messages: {}", e),
            }
        }
    }
    Err(appstate::AppError::NotFound)
}
