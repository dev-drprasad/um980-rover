use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router, extract::State, routing::get};
use axum_embed::ServeEmbed;
use oxidize_pdf::PdfReader;
use oxidize_pdf::parser::PdfDocument;
use regex::Regex;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use serialport::SerialPortType;
use std::collections::HashMap;
use std::io::Cursor;
use thiserror::Error;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

#[derive(RustEmbed, Clone)]
#[folder = "web/dist/"]
struct Assets;

#[derive(Debug, Error)]
enum AppError {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("API running on {}", addr);

    let serve_web = ServeEmbed::<Assets>::with_parameters(
        Some("index.html".to_owned()),
        axum_embed::FallbackBehavior::NotFound,
        Some("index.html".to_owned()),
    ); // Creates a service for the embedded files

    let app = Router::new()
        .route("/api/devices", get(get_devices))
        .route("/api/layers", get(get_layers))
        .fallback_service(serve_web)
        .layer(CorsLayer::permissive());
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Serialize)]
struct Err<'a> {
    error: &'a str,
}

// Notice we changed `devnode` to `id` because Windows/Mac don't use Linux /dev/ paths!
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UsbDevice {
    id: String,
    name: String,
    vendor: Option<String>,
    product: Option<String>,
}

async fn get_devices() -> Result<Json<HashMap<String, UsbDevice>>, AppError> {
    let mut map = HashMap::new();
    serialport::available_ports()
        .map_err(|e| AppError::InternalError(anyhow::anyhow!(e)))?
        .iter()
        .filter_map(|port| {
            // Check if the port is a USB device
            if let SerialPortType::UsbPort(usb_info) = &port.port_type {
                // Check if the hardware reported a product name
                if let Some(product_name) = &usb_info.product {
                    // Convert to lowercase for a safer, case-insensitive match
                    // This will catch "USB Serial", "USB-Serial", "USB  Serial", etc.
                    if product_name.to_lowercase().contains("usb serial") {
                        return Some(UsbDevice {
                            id: make_device_id(usb_info),
                            name: usb_info
                                .product
                                .clone()
                                .unwrap_or_else(|| "Unknown USB Serial Device".to_string()),
                            vendor: Some(usb_info.vid.to_string()),
                            product: Some(usb_info.pid.to_string()),
                        });
                    }
                }
            }
            // If it's not a USB port, or doesn't match the name, filter it out
            None
        })
        .for_each(|p| {
            map.insert(p.id.clone(), p);
        });
    return Ok(Json(map));
}

#[derive(Deserialize)]
struct GetLayersSearchParams {
    lpm: Option<String>,
}

async fn get_layers(Query(params): Query<GetLayersSearchParams>) -> Result<Json<Value>, AppError> {
    let lpm = params.lpm.ok_or_else(|| AppError::NotFound)?; // Return 404 if lpm is missing
    if lpm.is_empty() {
        return Err(AppError::NotFound); // Return 404 if lpm is empty
    }
    let geojson = get_lpm_document(lpm).await?;

    Ok(Json(geojson))
}

/// Creates a unique hardware ID string based on the bus and address
fn make_device_id(device: &serialport::UsbPortInfo) -> String {
    format!("{}-{}", device.vid, device.pid)
}

async fn get_lpm_document(lpm: String) -> Result<Value, anyhow::Error> {
    let url = format!(
        "https://bhunaksha.ap.gov.in/bhunakshalpm/rest/Reports/SinglePlotReportPDFUrl?giscode=2434007&state=28&plotno={}",
        lpm
    );
    let response: reqwest::Response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;

    let cursor = Cursor::new(bytes);

    let reader = PdfReader::new(cursor)?;
    let doc = PdfDocument::new(reader);

    let chunks = doc.rag_chunks()?;

    for chunk in &chunks {
        if chunk.element_types.contains(&"table".to_string()) {
            println!("  Section: {}", chunk.text);

            let re = Regex::new(r"^\s*(\d+)\s*\|\s*([0-9.]+)\s*\|\s*([0-9.]+)").unwrap();

            let mut coordinates: Vec<Vec<f64>> = Vec::new();

            for line in chunk.text.lines() {
                if let Some(caps) = re.captures(line) {
                    let lat_str = caps.get(2).unwrap().as_str();
                    let lng_str = caps.get(3).unwrap().as_str();

                    if let (Ok(lat), Ok(lng)) = (lat_str.parse::<f64>(), lng_str.parse::<f64>()) {
                        // Longitude first!
                        coordinates.push(vec![lng, lat]);
                    }
                }
            }

            // A valid polygon needs at least 3 distinct points + 1 closing point
            if coordinates.len() >= 3 {
                let first_point = coordinates.first().unwrap().clone();
                let last_point = coordinates.last().unwrap().clone();

                // Close the ring if the start and end points do not perfectly match
                if first_point != last_point {
                    coordinates.push(first_point);
                }
            } else {
                eprintln!("Warning: Not enough points to form a polygon.");
            }

            // Wrap the single polygon inside a FeatureCollection
            let geojson = json!({
                "type": "FeatureCollection",
                "features": [
                    {
                        "type": "Feature",
                        "geometry": {
                            "type": "Polygon",
                            // Note the array wrapping `coordinates` to define the exterior ring
                            "coordinates": [coordinates]
                        },
                        "properties": {
                            "name": "Extracted Polygon"
                        }
                    }
                ]
            });

            return Ok(geojson);
        }
    }

    return Ok(json!({}));
}
