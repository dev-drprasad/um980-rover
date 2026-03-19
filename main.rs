use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{
    Json, Router,
    extract::State,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    routing::get,
};
use axum_embed::ServeEmbed;
use futures::{sink::SinkExt, stream::StreamExt};
use nmea::Nmea;
use nusb::transfer::{Bulk, ControlOut, ControlType, In, Recipient};
use nusb::{DeviceInfo, MaybeFuture, list_devices};
use oxidize_pdf::PdfReader;
use oxidize_pdf::parser::PdfDocument;
use regex::Regex;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::Cursor;
use std::io::{Error, ErrorKind, Read};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::time::sleep;
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
// We use an AppState to hold our global broadcast channel.
// Any message sent into this channel will be pushed to all subscribers.
struct AppState {
    tx: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(addr).await?;

    // Create a broadcast channel with a buffer capacity of 100 messages
    let (tx, _rx) = broadcast::channel(100);
    let app_state = Arc::new(AppState { tx: tx.clone() });
    tokio::spawn(async move {
        // The spawned task now owns `quantity` and `item_name`
        let _ = get_nmea_messages(tx.clone()).await;
    });

    let serve_web = ServeEmbed::<Assets>::with_parameters(
        Some("index.html".to_owned()),
        axum_embed::FallbackBehavior::NotFound,
        Some("index.html".to_owned()),
    ); // Creates a service for the embedded files

    let app = Router::new()
        .route("/api/devices", get(get_devices))
        .route("/api/layers", get(get_layers))
        .route("/ws", get(ws_handler))
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

// The handler extracts the WebSocket and our shared State
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> axum::response::Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    println!("A new client connected!");

    // 1. Split the WebSocket into a Send half and a Receive half
    let (mut socket_sender, mut socket_receiver) = socket.split();

    // 2. Subscribe this specific client to the global broadcast channel
    let mut broadcast_rx = state.tx.subscribe();

    // 3. TASK 1: The "Writing" Task
    // This background task listens for messages on the global broadcast channel
    // and pushes them down the WebSocket to the client.
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            if socket_sender.send(Message::Text(msg)).await.is_err() {
                break; // Exit if the connection dropped
            }
        }
    });

    // 4. TASK 2: The "Reading" Task
    // This background task listens to the client's WebSocket. When the client
    // sends a message, it forwards it to the global broadcast channel.
    let broadcast_tx = state.tx.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = socket_receiver.next().await {
            // Send the client's message to everyone else!
            let _ = broadcast_tx.send(format!("User says: {}", text));
        }
    });

    // 5. The Concurrency Manager
    // Wait until either the sending task or receiving task finishes.
    // If a client disconnects, `recv_task` finishes. We then abort the `send_task`
    // so we don't leak memory keeping a dead connection alive.
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    println!("A client disconnected.");
}

// async fn start_coordinate_stream(tx: broadcast::Sender<String>) {
//     let coordinates = vec![
//         vec![78.901093, 13.557264],
//         vec![78.901209, 13.557210],
//         vec![78.901018, 13.556719],
//     ];

//     println!("Starting continuous coordinate broadcast...");
//     // .cycle() turns our vector into an infinite loop.
//     // When it reaches the last coordinate, it seamlessly starts over at the first one.
//     let mut coord_iter = coordinates.iter().cycle();

//     loop {
//         let pt = coord_iter.next().unwrap();

//         // Assuming your vector is [lng, lat] based on your previous GeoJSON code
//         let lng = pt[0];
//         let lat = pt[1];

//         // Format the data as a JSON string so the browser can easily parse it
//         let payload = json!({
//             "event": "liveLocation",
//             "data": {
//                 "latitude": lat,
//                 "longitude": lng
//             }
//         });

//         // Broadcast to all connected WebSocket clients.
//         // We intentionally ignore the Result here (using `let _ =`).
//         // tx.send() returns an error if NO clients are currently connected,
//         // which is perfectly fine—we just keep broadcasting to the empty room until someone joins!
//         let _ = tx.send(payload.to_string());

//         // Wait for 1 second before sending the next point
//         sleep(Duration::from_millis(1000)).await;
//     }
// }

async fn get_nmea_messages(tx: broadcast::Sender<String>) -> Result<(), anyhow::Error> {
    // 1. Find the CH340 device (VID: 0x1A86, PID: 0x7523)
    // Note: list_devices() is usually synchronous in nusb, so we just iterate it.
    let device_info = list_devices()
        .await?
        .find(|dev| dev.vendor_id() == 0x1a86 && dev.product_id() == 0x7523)
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "CH340 device not found"))?;

    println!("Found device: {:?}", device_info);

    // 2. Open the device
    // We use .wait()? to resolve the MaybeFuture synchronously
    let device = device_info.open().wait()?;

    // 3. Claim the interfaces (0 for Control, 1 for Data)
    let interface = device.detach_and_claim_interface(0).await?;
    // let data_interface = device.detach_and_claim_interface(1).await?;

    // 4. Assert DTR/RTS to wake up the serial chip
    // This tells the CH340 that a terminal is ready to receive data.
    // control_interface
    //     .control_out(
    //         ControlOut {
    //             control_type: ControlType::Class,
    //             recipient: Recipient::Interface,
    //             request: 0x22,
    //             value: 0x03,
    //             index: 0x00,
    //             data: &[],
    //         },
    //         Duration::from_millis(100),
    //     )
    //     .wait()?;

    // 4. CH340 Proprietary Initialization & Baud Rate Setup
    // Use ControlType::Vendor instead of Class for CH340 chips

    // A. Initialize the chip
    interface
        .control_out(
            ControlOut {
                control_type: ControlType::Vendor,
                recipient: Recipient::Device,
                request: 0xA1,
                value: 0x0000,
                index: 0x0000,
                data: &[],
            },
            Duration::from_millis(100),
        )
        .wait()?;

    // B. Set the Baud Rate
    // Most standard GPS modules use 9600. If yours is newer, it might be 115200.
    // 9600 Baud   = index: 0xB202
    // 115200 Baud = index: 0xCC03
    interface
        .control_out(
            ControlOut {
                control_type: ControlType::Vendor,
                recipient: Recipient::Device,
                request: 0x9A,
                value: 0x1312,
                index: 0xCC03, // <--- Change this to 0xCC03 if 9600 gives you garbage!
                data: &[],
            },
            Duration::from_millis(100),
        )
        .wait()?;

    // C. Set Line Control (8 data bits, No parity, 1 stop bit) & Enable UART
    interface
        .control_out(
            ControlOut {
                control_type: ControlType::Vendor,
                recipient: Recipient::Device,
                request: 0x9A,
                value: 0x2518,
                index: 0x00C3,
                data: &[],
            },
            Duration::from_millis(100),
        )
        .wait()?;

    println!("CH340 Initialized. Baud rate set. Listening...");

    println!("DTR/RTS asserted. Device should start transmitting...");

    // 5. Find the Bulk IN endpoint dynamically
    // (For CH340 it is almost always 0x82, but it's safest to check)
    let bulk_in_ep = 0x82;
    // for interface in device_info.active_config().unwrap().interfaces() {
    //     for alt in interface.alt_settings() {
    //         for ep in alt.endpoints() {
    //             if ep.direction() == nusb::descriptors::Direction::In
    //                 && ep.transfer_type() == nusb::descriptors::TransferType::Bulk
    //             {
    //                 bulk_in_ep = ep.address();
    //             }
    //         }
    //     }
    // }

    println!("Listening for data on Endpoint: 0x{:02X}", bulk_in_ep);

    // 6. Create a standard std::io::Read reader from the endpoint
    // This handles all the underlying queueing and buffer management automatically!
    let mut reader = interface.endpoint::<Bulk, In>(bulk_in_ep)?.reader(4096);
    let mut buf = [0u8; 64];

    // 1. Create a buffer to reconstruct split lines
    let mut line_buffer = String::new();

    // 2. Initialize the NMEA state machine
    let mut nmea_state = Nmea::default();

    // 7. The Read Loop
    loop {
        // This will block until data arrives, exactly like reading from a serial port
        match reader.read(&mut buf) {
            Ok(bytes_read) => {
                if bytes_read > 0 {
                    if let Ok(text) = std::str::from_utf8(&buf[..bytes_read]) {
                        // Push the new USB text into our holding buffer
                        line_buffer.push_str(text);

                        // Process complete lines one by one
                        while let Some(newline_index) = line_buffer.find('\n') {
                            // Extract the sentence and remove the trailing \r\n
                            let sentence = line_buffer[..=newline_index].trim().to_string();

                            // Remove the parsed sentence from the buffer so we don't read it again
                            line_buffer.drain(..=newline_index);

                            // Only parse lines that actually look like NMEA data
                            if sentence.starts_with('$') {
                                match nmea_state.parse(&sentence) {
                                    Ok(_) => {
                                        // The state machine successfully updated!
                                        // Let's print out the useful info:
                                        println!("--- GPS UPDATE ---");

                                        if let (Some(lat), Some(lng)) =
                                            (nmea_state.latitude, nmea_state.longitude)
                                        {
                                            println!("Location: {:.6}, {:.6}", lat, lng);
                                            match tx.send(
                                                json!({
                                                    "event": "latLngUpdate",
                                                    "data": {
                                                        "latitude": lat,
                                                        "longitude": lng
                                                    }
                                                })
                                                .to_string(),
                                            ) {
                                                Ok(_) => {} // Message sent successfully
                                                Err(e) => {
                                                    eprintln!("Failed to send GPS update: {}", e);
                                                }
                                            }
                                        }
                                        if let Some(alt) = nmea_state.altitude {
                                            match tx.send(
                                                json!({
                                                    "event": "altitudeUpdate",
                                                    "data": {
                                                        "altitudeMtrs": alt
                                                    }
                                                })
                                                .to_string(),
                                            ) {
                                                Ok(_) => {} // Message sent successfully
                                                Err(e) => {
                                                    eprintln!(
                                                        "Failed to send altitude update: {}",
                                                        e
                                                    );
                                                }
                                            }
                                        }
                                        if let Some(speed) = nmea_state.speed_over_ground {
                                            println!("Speed: {} knots", speed);
                                        }

                                        if let Some(fix_type) = nmea_state.fix_type() {
                                            println!("Fix Type: {:?}", fix_type);
                                            match tx.send(
                                                json!({
                                                    "event": "fixUpdate",
                                                    "data": {
                                                        "fixType": fix_type,
                                                    }
                                                })
                                                .to_string(),
                                            ) {
                                                Ok(_) => {} // Message sent successfully
                                                Err(e) => {
                                                    eprintln!("Failed to send GPS update: {}", e);
                                                }
                                            }
                                        }

                                        println!(
                                            "Satellites Tracked: {}",
                                            nmea_state.satellites().len()
                                        );
                                        match tx.send(
                                            json!({
                                                "event": "statusUpdate",
                                                "data": {
                                                    "satellites": nmea_state.satellites().len()
                                                }
                                            })
                                            .to_string(),
                                        ) {
                                            Ok(_) => {} // Message sent successfully
                                            Err(e) => {
                                                eprintln!("Failed to send GPS update: {}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to parse NMEA sentence: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("Error reading from USB: {}", e);
                sleep(Duration::from_secs(1)).await;
                break;
            }
        }
        sleep(Duration::from_millis(700)).await;
    }

    Ok(())
}
