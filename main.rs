use axum::extract::Query;
use axum::{
    Json, Router,
    routing::{get, post},
};
use axum_embed::ServeEmbed;
use nusb::{DeviceInfo, MaybeFuture};
use oxidize_pdf::PdfReader;
use oxidize_pdf::parser::PdfDocument;
use regex::Regex;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::Cursor;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
mod appstate;
mod get_current_latlng;
mod mynmea;
mod ntrip_client;
mod send_live_status_to_client;
mod send_nmea_to_client;
mod track;

#[derive(RustEmbed, Clone)]
#[folder = "web/dist/"]
struct Assets;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(addr).await?;

    // Create a broadcast channel with a buffer capacity of 100 messages
    let (rtcm_tx, _rx) = broadcast::channel(100);
    let (nmea_tx, _rx) = broadcast::channel(100);
    let app_state = Arc::new(appstate::AppState {
        rtcm_tx: rtcm_tx.clone(),
        nmea_tx: nmea_tx.clone(),
        file_lock: Mutex::new(()),
    });
    let nmea_tx_for_device = nmea_tx.clone();
    let rtcm_tx_for_device = rtcm_tx.clone();
    tokio::spawn(async move {
        let _ = mynmea::read_nmea_and_broadcast(nmea_tx_for_device, rtcm_tx_for_device).await;
    });

    let rtcm_tx_for_ntrip = rtcm_tx.clone();
    let nmea_tx_for_ntrip = nmea_tx.clone();
    tokio::spawn(async move {
        let _ = ntrip_client::ntrip_client(nmea_tx_for_ntrip, rtcm_tx_for_ntrip)
            .await
            .expect("Failed to setup ntrip client");
    });

    let serve_web = ServeEmbed::<Assets>::with_parameters(
        Some("index.html".to_owned()),
        axum_embed::FallbackBehavior::NotFound,
        Some("index.html".to_owned()),
    ); // Creates a service for the embedded files

    let app = Router::new()
        .route("/api/devices", get(get_devices))
        .route("/api/layers", get(get_layers))
        .route("/api/latlng", get(get_current_latlng::get_current_latlng)) // GET route
        .route("/api/ntrip-settings", get(ntrip_client::get_ntrip_settings)) // GET route
        .route(
            "/api/ntrip-settings",
            post(ntrip_client::set_ntrip_settings),
        ) // POST route
        .route("/api/track", get(track::get_all_coordinates)) // GET route
        .route("/api/draft", get(track::get_draft_points_handler)) // GET route
        .route("/api/draft/save", post(track::save_draft_handler)) // POST route
        .route("/api/draft/undo", post(track::undo_draft_handler)) // POST route
        .route("/api/track/:id", post(track::append_coordinates)) // POST route
        .route(
            "/ws",
            get(send_live_status_to_client::send_live_status_to_client),
        )
        .route("/nmea-ws", get(send_nmea_to_client::send_nmea_to_client))
        .fallback_service(serve_web)
        .layer(CorsLayer::permissive())
        .with_state(app_state);
    println!("API running on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

// Notice we changed `devnode` to `id` because Windows/Mac don't use Linux /dev/ paths!
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UsbDevice {
    id: String,
    name: String,
    vendor: Option<String>,
    product: Option<String>,
}

async fn get_devices() -> Result<Json<Vec<UsbDevice>>, appstate::AppError> {
    return Ok(Json(get_available_ports().await?));
}

#[derive(Deserialize)]
struct GetLayersSearchParams {
    lpm: Option<String>,
}

async fn get_layers(
    Query(params): Query<GetLayersSearchParams>,
) -> Result<Json<Value>, appstate::AppError> {
    let lpm = params.lpm.ok_or_else(|| appstate::AppError::NotFound)?; // Return 404 if lpm is missing
    if lpm.is_empty() {
        return Err(appstate::AppError::NotFound); // Return 404 if lpm is empty
    }
    let geojson = get_lpm_document(lpm).await?;

    Ok(Json(geojson))
}

async fn get_lpm_document(lpm: String) -> Result<Value, anyhow::Error> {
    let url = format!(
        "https://bhunaksha.ap.gov.in/bhunakshalpm/rest/Reports/SinglePlotReportPDFUrl?giscode=2434007&state=28&plotno={}",
        lpm
    );
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;
    let response: reqwest::Response = client.get(url).send().await?;
    let bytes = response.bytes().await?;

    let cursor = Cursor::new(bytes);

    let reader = PdfReader::new(cursor)?;
    let doc = PdfDocument::new(reader);
    // let doc = PdfDocument::open("mock/document.pdf")?;

    let chunks = doc.extract_text()?;
    println!("{:?}", doc.metadata());

    let re =
        Regex::new(r"(?m)^(\d+)\s+([\d.]+)\s+([\d.]+)\s+(\d+\.\d)(\d{2}\.\d+)\s+([\d.]+)\s+(\d+)")
            .unwrap();
    let mut coordinates: Vec<Vec<f64>> = Vec::new();
    for chunk in &chunks {
        println!("Chunk: {}", chunk.text);
        for caps in re.captures_iter(&chunk.text) {
            println!(
                "{}\t{}\t{}",
                caps.get(1).unwrap().as_str(),
                caps.get(2).unwrap().as_str(),
                caps.get(5).unwrap().as_str(),
            );
            let lng_str = caps.get(2).unwrap().as_str();
            let lat_str = caps.get(5).unwrap().as_str();

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

async fn is_cdc_acm(device_info: &DeviceInfo) -> bool {
    // Open the device to inspect descriptors
    let device = match device_info.open().wait() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error opening device: {}", e);
            return false;
        }
    };

    // Get the active configuration descriptor
    let config = match device.active_configuration() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error getting active configuration: {}", e);
            return false;
        }
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
        println!(
            "Checking device: VID {:04x}, PID {:04x}",
            device.vendor_id(),
            device.product_id()
        );
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
