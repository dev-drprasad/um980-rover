use crate::appstate;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::io::AsyncWriteExt;
use tokio::time::{Duration, sleep};
use uuid::Uuid;

// Define our global file path so we don't misspell it in different functions
const FILE_PATH: &str = "tracking_data.json";

type Lng = f64;
type Lat = f64;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SavePayload {
    #[serde(rename = "track")]
    Track {
        name: String,
        coordinates: Vec<CoordinatesPayload>,
    },
    #[serde(rename = "bookmark")]
    Bookmark {
        name: String,
        coordinates: CoordinatesPayload,
    },
    #[serde(rename = "draft")]
    Draft {
        name: String,
        coordinates: Vec<CoordinatesPayload>,
    },
}

type CoordinatesPayload = (Lat, Lng);
// Type alias for our entire saved JSON structure
type TrackingData = HashMap<String, SavePayload>;

async fn write_with_flush(contents: &str) -> Result<(), (StatusCode, String)> {
    let mut file = tokio::fs::File::create(FILE_PATH).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("File create error: {}", e),
        )
    })?;

    file.write_all(contents.as_bytes()).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("File write error: {}", e),
        )
    })?;

    file.sync_all().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("File sync error: {}", e),
        )
    })?;

    file.flush().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("File flush error: {}", e),
        )
    })?;

    // above code is not helping with the issue of file not being updated in time for the next read,
    // so adding an explicit sleep to give the OS time to update the file system
    sleep(Duration::from_millis(2500)).await;

    Ok(())
}

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
            let processed_data = get_track_polygon(data);
            Ok((StatusCode::OK, Json(processed_data)))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read tracking file: {}", e),
        )),
    }
}

pub async fn get_draft_points_handler(
    State(state): State<Arc<appstate::AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Lock the file so we don't read it while a POST request is halfway through writing it
    let _guard = state.file_lock.lock().await;

    match tokio::fs::read_to_string(FILE_PATH).await {
        Ok(contents) => {
            // Parse the string into our HashMap structure
            let data: TrackingData = serde_json::from_str(&contents).unwrap_or_default();
            let processed_data = get_draft_points(data);
            Ok((StatusCode::OK, Json(processed_data)))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read tracking file: {}", e),
        )),
    }
}

#[derive(Deserialize)]
pub struct SaveDraftPayload {
    name: String,
}

pub async fn save_draft_handler(
    State(state): State<Arc<appstate::AppState>>,
    Json(payload): Json<SaveDraftPayload>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let _guard = state.file_lock.lock().await;

    let mut data: TrackingData = match tokio::fs::read_to_string(FILE_PATH).await {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => HashMap::new(),
    };

    let Some(draft_payload) = data.remove("draft") else {
        return Err((StatusCode::NOT_FOUND, "Draft not found".to_string()));
    };

    let new_key = Uuid::new_v4().to_string();
    match draft_payload {
        SavePayload::Draft { coordinates, .. } => {
            data.insert(
                new_key,
                SavePayload::Track {
                    name: payload.name,
                    coordinates,
                },
            );
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "The draft entry has an invalid payload type".to_string(),
            ));
        }
    }

    let updated_json = match serde_json::to_string_pretty(&data) {
        Ok(json) => json,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    };

    write_with_flush(&updated_json).await?;
    Ok((StatusCode::OK, Json(get_draft_points(data))))
}

pub async fn undo_draft_handler(
    State(state): State<Arc<appstate::AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let _guard = state.file_lock.lock().await;

    let mut data: TrackingData = match tokio::fs::read_to_string(FILE_PATH).await {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => HashMap::new(),
    };
    data.entry("draft".to_string()).and_modify(|existing| {
        if let SavePayload::Draft { name, coordinates } = existing {
            coordinates.pop();
        }
    });

    // Save updated data back to file
    let updated_json = match serde_json::to_string_pretty(&data) {
        Ok(json) => json,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    };

    write_with_flush(&updated_json).await?;

    Ok((StatusCode::OK, Json(get_draft_points(data))))
}

// 4. The POST Handler (Append new data)
pub async fn append_coordinates(
    Path(id): Path<String>,
    State(state): State<Arc<appstate::AppState>>,
    Json(new_coords): Json<SavePayload>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let _guard = state.file_lock.lock().await;

    // Read existing data
    let mut data: TrackingData = match tokio::fs::read_to_string(FILE_PATH).await {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => HashMap::new(),
    };

    println!("{:?}", new_coords);

    // Append new coordinates
    data.entry(id.clone())
        .and_modify(|existing| {
            if let SavePayload::Draft { name, coordinates } = existing {
                if let SavePayload::Draft {
                    coordinates: new_coords,
                    ..
                } = &new_coords
                {
                    *coordinates = new_coords.clone();
                }
            }
        })
        .or_insert_with(|| new_coords);

    // Save back to disk
    let updated_json = match serde_json::to_string_pretty(&data) {
        Ok(json) => json,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    };

    write_with_flush(&updated_json).await?;

    Ok((StatusCode::OK, updated_json))
}

pub struct Coordinate {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Serialize, Debug)]
pub struct SideData {
    pub side: [CoordinatesPayload; 2], // Exactly two coordinates: [start, end]
    pub distance_in_cm: f64,
}

#[derive(Serialize, Debug)]
pub struct LngLat {
    pub lng: f64,
    pub lat: f64,
}

// Updated main result struct
#[derive(Serialize, Debug)]
pub struct PolygonResult {
    pub id: String,
    pub name: String,
    pub area: f64,
    pub area_in_cents: f64,
    pub points: Vec<LngLat>,  // Original coordinates for the polygon
    pub sides: Vec<SideData>, // Replaces 'coordinates' and 'side_distances_in_cm'
}

/// Calculates the distance between two [lat, lng] points in centimeters using Haversine
fn calculate_distance_cm(p1: &[f64], p2: &[f64]) -> f64 {
    const R: f64 = 637_100_000.0; // Earth's radius in centimeters

    let lat1 = p1[0].to_radians();
    let lon1 = p1[1].to_radians();
    let lat2 = p2[0].to_radians();
    let lon2 = p2[1].to_radians();

    let d_lat = lat2 - lat1;
    let d_lon = lon2 - lon1;

    let a = (d_lat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (d_lon / 2.0).sin().powi(2);

    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    R * c
}

/// Calculates the spherical area of a polygon in square meters
fn calculate_polygon_area_sq_m(path: &Vec<CoordinatesPayload>) -> f64 {
    let mut unique_path = path.to_vec();

    // Chamberlain & Duquette requires unique vertices.
    // If the path is closed (last == first), remove the closing point for the math.
    if unique_path.len() > 1 && unique_path.first() == unique_path.last() {
        unique_path.pop();
    }

    let n = unique_path.len();
    if n < 3 {
        return 0.0; // Not a polygon
    }

    const R: f64 = 6378137.0; // Earth's equatorial radius in meters (WGS84)
    let mut area = 0.0;

    for i in 0..n {
        let lower_index = if i == 0 { n - 1 } else { i - 1 };
        let middle_index = i;
        let upper_index = if i == n - 1 { 0 } else { i + 1 };

        let p1 = &unique_path[lower_index];
        let p2 = &unique_path[middle_index];
        let p3 = &unique_path[upper_index];

        let lon1 = p1.1.to_radians();
        let lat2 = p2.0.to_radians();
        let lon3 = p3.1.to_radians();

        area += (lon3 - lon1) * lat2.sin();
    }

    (area * R * R / 2.0).abs()
}

pub fn get_polygon_result(
    coordinates: Vec<(f64, f64)>,
    id: String,
    name: String,
    close: bool,
) -> PolygonResult {
    let area = calculate_polygon_area_sq_m(&coordinates);
    let mut sides = Vec::with_capacity(coordinates.len().saturating_sub(1));

    if coordinates.len() > 1 {
        for i in 0..(coordinates.len() - 1) {
            let p1 = coordinates[i];
            let p2 = coordinates[i + 1];
            let dist = calculate_distance_cm(&[p1.0, p1.1], &[p2.0, p2.1]);

            sides.push(SideData {
                side: [(p1.0, p1.1), (p2.0, p2.1)],
                distance_in_cm: dist,
            });
        }
    }

    if close && coordinates.len() > 2 {
        if let Some(p1) = coordinates.last() {
            if let Some(p2) = coordinates.first() {
                let dist = calculate_distance_cm(&[p1.0, p1.1], &[p2.0, p2.1]);

                sides.push(SideData {
                    side: [(p1.0, p1.1), (p2.0, p2.1)],
                    distance_in_cm: dist,
                });
            }
        }
    }

    PolygonResult {
        id,
        name,
        area,
        area_in_cents: sq_m_to_cents(area),
        points: coordinates
            .iter()
            .map(|(lat, lng)| LngLat {
                lng: *lng,
                lat: *lat,
            })
            .collect(),
        sides,
    }
}

pub fn get_track_polygon(data: TrackingData) -> Vec<PolygonResult> {
    let mut result = Vec::<PolygonResult>::new();

    for (key, path) in data {
        if matches!(
            path,
            SavePayload::Bookmark { .. } | SavePayload::Draft { .. }
        ) {
            continue;
        }

        if let SavePayload::Track { name, coordinates } = path {
            result.push(get_polygon_result(coordinates, key, name, true));
        }
    }

    result
}

pub fn get_draft_points(data: TrackingData) -> Option<PolygonResult> {
    for (key, path) in data {
        if matches!(
            path,
            SavePayload::Bookmark { .. } | SavePayload::Track { .. }
        ) {
            continue;
        }

        if let SavePayload::Draft { name, coordinates } = path {
            return Some(get_polygon_result(coordinates, key, name, false));
        }
    }

    return None;
}

// pub fn get_polygon_result(coordinates: Vec<(f64, f64)>, key: String) -> PolygonResult {
//     let mut result = Vec::<PolygonResult>::new();

//     let area = calculate_polygon_area(&coordinates);
//     let mut sides = Vec::with_capacity(coordinates.len().saturating_sub(1));

//     if coordinates.len() > 1 {
//         for i in 0..(coordinates.len() - 1) {
//             let p1 = coordinates[i];
//             let p2 = coordinates[i + 1];
//             let dist = calculate_distance_cm(&[p1.0, p1.1], &[p2.0, p2.1]);

//             sides.push(SideData {
//                 side: [(p1.0, p1.1), (p2.0, p2.1)],
//                 distance_in_cm: dist,
//             });
//         }
//     }

//     return PolygonResult {
//         id: key,
//         area,
//         sides,
//     };

//     // let mut sides = Vec::with_capacity(path.len().saturating_sub(1));

//     // // Group the points into [start, end] pairs and calculate distance
//     // if path.len() > 1 {
//     //     for i in 0..(path.len() - 1) {
//     //         let p1 = path[i].clone();
//     //         let p2 = path[i + 1].clone();
//     //         let dist = calculate_distance_cm(&p1, &p2);

//     //         sides.push(SideData {
//     //             side: [p1, p2],
//     //             distance_in_cm: dist,
//     //         });
//     //     }
//     // }

//     // result.push(PolygonResult {
//     //     id: key,
//     //     area,
//     //     sides,
//     // });
// }

fn sq_m_to_cents(area_sq_m: f64) -> f64 {
    area_sq_m * 0.0247105
}
