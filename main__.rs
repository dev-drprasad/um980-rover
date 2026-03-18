use oxidize_pdf::parser::PdfDocument;
use regex::Regex;
use serde_json::json;
use serialport;
use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = PdfDocument::open("1203.pdf").expect("Failed to PDF document");

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

            println!("{}", serde_json::to_string_pretty(&geojson).unwrap());
        }

        // Use chunk.full_text for embeddings (includes heading context)
        // Use chunk.text for display (content only)
    }

    let port_name = "/dev/tty.usbserial-10".to_string();
    let baud_rate = 115200;
    let serial_port = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(1000))
        .open()
        .expect("Failed to open serial port");

    let delay = Duration::from_millis(100);

    // 2. Start the TCP Server
    let tcp_bind_addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(tcp_bind_addr).expect("Failed to bind TCP listener");
    println!("TCP server listening on {}", tcp_bind_addr);
    println!("Waiting for a client to connect...");

    let mut buffer = BufReader::new(serial_port);
    let mut line = String::new();

    // 3. Outer loop: Wait for incoming TCP clients
    for stream_result in listener.incoming() {
        match stream_result {
            Ok(mut tcp_stream) => {
                let peer_addr = tcp_stream
                    .peer_addr()
                    .unwrap_or_else(|_| "Unknown".parse().unwrap());
                println!("New client connected: {}", peer_addr);

                // 4. Inner loop: Read from serial and forward to the connected client
                loop {
                    match buffer.read_line(&mut line) {
                        Ok(bytes_read) => {
                            if bytes_read > 0 {
                                // We received data, forward it to the TCP server
                                if let Err(e) = tcp_stream.write_all(line.as_bytes()) {
                                    eprintln!("Error writing to TCP server: {}", e);
                                    break; // Exit loop if the TCP connection drops
                                }
                                // Optional: flush the TCP stream to ensure data is sent immediately
                                let _ = tcp_stream.flush();
                                println!("Forwarded to TCP server: {}", line.trim());
                                line.clear();
                            }
                        }
                        Err(e) if e.kind() == io::ErrorKind::TimedOut => {
                            // Timeout errors are common when waiting for data; they can be ignored
                            continue;
                        }
                        Err(e) => eprintln!("Error reading from serial port: {}", e),
                    }
                    thread::sleep(delay);
                }
                println!("Waiting for a new client...");
            }
            Err(e) => {
                eprintln!("Error accepting incoming connection: {}", e);
            }
        }
    }

    Ok(())
}
