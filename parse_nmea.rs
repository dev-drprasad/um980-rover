use std::io::Read;

use nmea::Nmea;
use nusb::Interface;
use nusb::transfer::{Bulk, In};
use serde_json::json;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::sleep;

pub async fn parse_nmea(
    interface: Interface,
    ep_in: u8,
    tx: &broadcast::Sender<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = interface.endpoint::<Bulk, In>(ep_in)?.reader(4096);
    let mut buf = [0u8; 64];
    let mut nmea_state = Nmea::default();
    let mut line_buffer = String::new();
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
                            parse_nmea_(sentence, &mut nmea_state, &tx);
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

fn parse_nmea_(sentence: String, nmea_state: &mut nmea::Nmea, tx: &broadcast::Sender<String>) {
    if sentence.starts_with('$') {
        match nmea_state.parse(&sentence) {
            Ok(_) => {
                // The state machine successfully updated!
                // Let's print out the useful info:
                println!("--- GPS UPDATE ---");

                if let (Some(lat), Some(lng)) = (nmea_state.latitude, nmea_state.longitude) {
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
                            eprintln!("Failed to send altitude update: {}", e);
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

                println!("Satellites Tracked: {}", nmea_state.satellites().len());
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
