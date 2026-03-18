use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router, routing::get};
use axum_embed::ServeEmbed;
use nusb::{DeviceInfo, MaybeFuture};
use oxidize_pdf::PdfReader;
use oxidize_pdf::parser::PdfDocument;
use regex::Regex;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
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

    get_available_ports().await?; // List available ports on startup

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

async fn get_devices() -> Result<Json<Vec<UsbDevice>>, AppError> {
    return Ok(Json(get_available_ports().await?));
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

async fn is_cdc_acm(device_info: &DeviceInfo) -> bool {
    // Open the device to inspect descriptors
    let device = match device_info.open().wait() {
        Ok(d) => d,
        Err(_) => return false,
    };

    // Get the active configuration descriptor
    let config = match device.active_configuration() {
        Ok(c) => c,
        Err(_) => return false,
    };

    // Iterate through interfaces
    for interface in config.interfaces() {
        for alt in interface.alt_settings() {
            println!(
                "Checking interface: class {:02x}, subclass {:02x}, protocol {:02x}",
                alt.class(),
                alt.subclass(),
                alt.protocol()
            );
            if alt.class() == 0xff {
                if device_info.vendor_id() == 0x1a86 && device_info.product_id() == 0x7523 {
                    println!("Found CH340 device, treating as CDC ACM");
                    return true;
                }
            }
            // CDC ACM Class/Subclass/Protocol
            // Class 0x02, Subclass 0x02, Protocol 0x01
            if alt.class() == 0x02 && alt.subclass() == 0x02 && alt.protocol() == 0x01 {
                return true;
            }
        }
    }
    false
}

async fn get_available_ports() -> Result<Vec<UsbDevice>, anyhow::Error> {
    let mut serial_devices: Vec<UsbDevice> = Vec::new();
    for device in nusb::list_devices().await? {
        if is_cdc_acm(&device).await {
            println!(
                "Found CDC ACM Device: VID {:04x}, PID {:04x}",
                device.vendor_id(),
                device.product_id()
            );
            serial_devices.push(UsbDevice {
                id: format!("{:04x}-{:04x}", device.vendor_id(), device.product_id()),
                name: device
                    .product_string()
                    .unwrap_or_else(|| "Unknown CDC ACM Device")
                    .to_string(),
                vendor: Some(format!("{:04x}", device.vendor_id())),
                product: Some(format!("{:04x}", device.product_id())),
            });
        }
    }
    Ok(serial_devices)
}
