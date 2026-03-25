use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::env;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::sleep;

pub async fn ntrip_client(
    nmea_tx: tokio::sync::broadcast::Sender<String>,
    rtcm_tx: tokio::sync::broadcast::Sender<Vec<u8>>,
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

    let mut rtcm_task = tokio::spawn(async move {
        let mut rtcm_buf = [0u8; 1024];
        loop {
            match tcp_read.read(&mut rtcm_buf).await {
                Ok(bytes_read) => {
                    if bytes_read > 0 {
                        // Shovel the raw binary data straight into the USB OUT endpoint
                        if let Err(e) = rtcm_tx.send(rtcm_buf[..bytes_read].to_vec()) {
                            println!("Failed to send RTCM data: {}", e);
                            break;
                        }
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

    let mut broadcast_rx = nmea_tx.subscribe();
    let mut nmea_task = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            if let Ok(nmea_messages) = serde_json::from_str::<Vec<String>>(&msg) {
                for setentense in nmea_messages {
                    if setentense.contains("GGA,") {
                        if let Err(e) = tcp_write.write_all(setentense.as_bytes()).await {
                            println!("Failed to upload GGA to Caster: {}", e);
                            break;
                        }
                        println!("Uploaded VRS position: {}", setentense.trim());
                    }
                }
            }
        }
    });

    // Run until one of the streams breaks or disconnects
    tokio::select! {
        _ = (&mut rtcm_task) => println!("RTCM task terminated."),
        _ = (&mut nmea_task) => println!("NMEA task terminated."),
    }

    Ok(())
}
