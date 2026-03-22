use crate::appstate;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::{collections::HashMap, sync::Arc};

// Define our global file path so we don't misspell it in different functions
const FILE_PATH: &str = "tracking_data.json";

type CoordinatesPayload = Vec<Vec<f64>>;
// Type alias for our entire saved JSON structure
type TrackingData = HashMap<String, Vec<Vec<f64>>>;

// 3. The GET Handler (Retrieve all JSON data)
pub async fn get_all_coordinates(
    State(state): State<Arc<appstate::AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Lock the file so we don't read it while a POST request is halfway through writing it
    let _guard = state.file_lock.lock().await;

    match tokio::fs::read_to_string(FILE_PATH).await {
        Ok(contents) => {
            // Parse the string into our HashMap structure
            let data: TrackingData = serde_json::from_str(&contents).unwrap_or_default();
            // Axum's Json extractor automatically adds the correct "application/json" headers
            Ok((StatusCode::OK, Json(data)))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read tracking file: {}", e),
        )),
    }
}

// 4. The POST Handler (Append new data)
pub async fn append_coordinates(
    Path(id): Path<String>,
    State(state): State<Arc<appstate::AppState>>,
    Json(new_coords): Json<CoordinatesPayload>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let _guard = state.file_lock.lock().await;

    // Read existing data
    let mut data: TrackingData = match tokio::fs::read_to_string(FILE_PATH).await {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => HashMap::new(),
    };

    // Append new coordinates
    data.entry(id.clone())
        .or_insert_with(Vec::new)
        .extend(new_coords);

    // Save back to disk
    let updated_json = match serde_json::to_string_pretty(&data) {
        Ok(json) => json,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    };

    if let Err(e) = tokio::fs::write(FILE_PATH, updated_json.clone()).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("File write error: {}", e),
        ));
    }

    Ok((StatusCode::OK, updated_json))
}
