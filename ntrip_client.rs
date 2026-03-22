use base64::{Engine as _, engine::general_purpose::STANDARD};
use nmea::Nmea;
use nusb::Interface;
use nusb::transfer::{Bulk, In, Out};
use serde_json::json;
use std::env;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::sleep;

pub async fn ntrip_client(
    usb_tx: Interface,
    usb_rx: Interface,
    ep_out: u8,
    ep_in: u8,
    tx: tokio::sync::broadcast::Sender<String>,
) -> anyhow::Result<()> {
    println!("Connecting to NTRIP Caster...");
    let mut tcp_stream = TcpStream::connect("rtk2go.com:2101").await?;
    let ntrip_email = env::var("NTRIP_EMAIL")?;

    // Construct the NTRIP v1 HTTP Request
    let auth = STANDARD.encode(format!("{}:", ntrip_email));
    let request = format!(
        "GET /IndiaTN01 HTTP/1.0\r\n\
         User-Agent: RustNtripClient/1.0\r\n\
         Authorization: Basic {}\r\n\
         \r\n",
        auth
    );

    tcp_stream.write_all(request.as_bytes()).await?;

    // Read the caster's response header to confirm "ICY 200 OK" or "HTTP/1.1 200 OK"
    let mut header_buf = [0u8; 512];
    let n = tcp_stream.read(&mut header_buf).await?;
    let header_text = String::from_utf8_lossy(&header_buf[..n]);
    if !header_text.contains("200 OK") {
        panic!("NTRIP connection failed. Caster replied: \n{}", header_text);
    }
    println!("NTRIP Connected! RTCM streaming started.");

    // Split the TCP socket into a reader (RTCM data) and writer (GGA data)
    let (mut tcp_read, mut tcp_write) = tcp_stream.into_split();

    // ==========================================
    // 3. TASK A: DOWNLOAD RTCM -> WRITE TO USB
    // ==========================================
    let mut usb_writer = usb_tx.endpoint::<Bulk, Out>(ep_out)?.writer(4096);
    let mut rtcm_task = tokio::spawn(async move {
        let mut rtcm_buf = [0u8; 1024];
        loop {
            println!("Waiting for RTCM data from NTRIP...");
            match tcp_read.read(&mut rtcm_buf).await {
                Ok(bytes_read) => {
                    if bytes_read > 0 {
                        // Shovel the raw binary data straight into the USB OUT endpoint
                        if let Err(e) = usb_writer.write_all(&rtcm_buf[..bytes_read]).await {
                            println!("Failed to write RTCM to USB: {}", e);
                            break;
                        }
                        // Force the buffer to push to the hardware immediately
                        if let Err(e) = usb_writer.flush().await {
                            println!("Failed to flush USB writer: {}", e);
                            break;
                        }
                        println!("Received {} bytes of RTCM, forwarded to USB.", bytes_read);
                    } else {
                        println!("NTRIP Stream gave 0 bytes");
                        break;
                    }
                }
                _ => {
                    println!("NTRIP TCP Stream closed by server.");
                    break;
                }
            }
            sleep(Duration::from_millis(250)).await;
        }
    });

    // ==========================================
    // 4. TASK B: READ USB NMEA -> UPLOAD GGA TO CASTER
    // ==========================================
    let mut usb_reader = usb_rx.endpoint::<Bulk, In>(ep_in)?.reader(4096);
    let mut nmea_state = Nmea::default();
    let mut nmea_task = tokio::spawn(async move {
        let mut buf = [0u8; 64];
        let mut line_buffer = String::new();
        // 7. The Read Loop
        loop {
            // This will block until data arrives, exactly like reading from a serial port
            match usb_reader.read(&mut buf).await {
                Ok(bytes_read) => {
                    if bytes_read > 0 {
                        if let Ok(text) = std::str::from_utf8(&buf[..bytes_read]) {
                            // Push the new USB text into our holding buffer
                            line_buffer.push_str(text);

                            // Process complete lines one by one
                            while let Some(idx) = line_buffer.find('\n') {
                                let sentence = line_buffer[..=idx].to_string();
                                line_buffer.drain(..=idx);

                                if sentence.starts_with('$') {
                                    match nmea_state.parse(&sentence) {
                                        Ok(_) => {
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
                                                        eprintln!(
                                                            "Failed to send GPS update: {}",
                                                            e
                                                        );
                                                    }
                                                }
                                            }
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
                                                        eprintln!(
                                                            "Failed to send GPS update: {}",
                                                            e
                                                        );
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
                                            eprintln!(
                                                "Failed to parse NMEA sentence: {}. Error: {}",
                                                sentence.trim(),
                                                e
                                            );
                                        }
                                    }
                                }

                                // Is it a GGA sentence? (Supports both $GPGGA and $GNGGA)
                                if sentence.contains("GGA,") {
                                    // Send the sentence back up the TCP socket to the caster!
                                    if let Err(e) = tcp_write.write_all(sentence.as_bytes()).await {
                                        println!("Failed to upload GGA to Caster: {}", e);
                                        break;
                                    }
                                    println!("Uploaded VRS position: {}", sentence.trim());
                                }

                                // (Optional: You can still pipe the sentence to your `nmea` crate
                                // parser here to update your frontend WebSocket!)
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("USB NMEA Stream closed by device: {}", e);
                    break;
                }
            }

            sleep(Duration::from_millis(700)).await;
        }
    });

    // Run until one of the streams breaks or disconnects
    tokio::select! {
        _ = (&mut rtcm_task) => println!("RTCM task terminated."),
        _ = (&mut nmea_task) => println!("NMEA task terminated."),
    }

    Ok(())
}
