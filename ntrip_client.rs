use axum::{Json, http::StatusCode, response::IntoResponse};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::sleep;

const FILE_PATH: &str = "ntrip-settings.json";

pub async fn get_ntrip_settings() -> Result<impl IntoResponse, (StatusCode, String)> {
    let settings = tokio::fs::read_to_string(FILE_PATH)
        .await
        .ok()
        .and_then(|content| serde_json::from_str::<NTRIPSettings>(&content).ok())
        .unwrap_or(NTRIPSettings {
            username: "".to_string(),
            password: "".to_string(),
            mountpoint: "".to_string(),
            host: "".to_string(),
        });
    Ok(Json(settings))
}

#[derive(Deserialize, Serialize)]
pub struct NTRIPSettings {
    pub username: String,
    pub mountpoint: String,
    pub password: String,
    pub host: String,
}

pub async fn set_ntrip_settings(
    Json(payload): Json<NTRIPSettings>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let payload_str = serde_json::to_string(&payload).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serialize payload: {}", e),
        )
    })?;
    tokio::fs::write(FILE_PATH, payload_str)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to write settings: {}", e),
            )
        })?;

    Ok(Json(payload))
}

pub async fn ntrip_client(
    nmea_tx: tokio::sync::broadcast::Sender<String>,
    rtcm_tx: tokio::sync::broadcast::Sender<Vec<u8>>,
) -> anyhow::Result<()> {
    let ntrip_email = env::var("NTRIP_EMAIL")?;

    let default_ntrip_settings = NTRIPSettings {
        username: ntrip_email.clone(),
        password: "".to_string(),
        mountpoint: "IndiaTN02".to_string(),
        host: "rtk2go.com:2101".to_string(),
    };

    let ntrip_settings = tokio::fs::read_to_string("ntrip-settings.json")
        .await
        .ok()
        .and_then(|content| serde_json::from_str::<NTRIPSettings>(&content).ok())
        .unwrap_or(default_ntrip_settings);

    // Construct the NTRIP v1 HTTP Request
    let auth = STANDARD.encode(format!(
        "{}:{}",
        ntrip_settings.username, ntrip_settings.password
    ));
    let request = format!(
        "GET /{} HTTP/1.0\r\n\
     User-Agent: RustNtripClient/1.0\r\n\
     Authorization: Basic {}\r\n\
     \r\n",
        ntrip_settings.mountpoint, auth
    );
    println!(
        "Connecting to NTRIP at {} with mountpoint '{}'",
        ntrip_settings.host, ntrip_settings.mountpoint
    );
    let mut tcp_stream = TcpStream::connect(ntrip_settings.host).await?;
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
                for sentence in nmea_messages {
                    if sentence.contains("GGA,") {
                        println!("VRS position: {}", sentence.trim());
                        if sentence.contains(",N,") {
                            if let Err(e) = tcp_write.write_all(sentence.as_bytes()).await {
                                println!("Failed to upload GGA to Caster: {}", e);
                                break;
                            }
                            println!("Uploaded VRS position: {}", sentence.trim());
                        }
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
