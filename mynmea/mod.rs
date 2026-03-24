use nusb::{
    MaybeFuture,
    io::EndpointRead,
    list_devices,
    transfer::{Bulk, ControlOut, ControlType, In, Recipient},
};
use regex::Regex;
use serde_json::json;
use std::io::{Error, ErrorKind};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::{
    io::AsyncReadExt,
    time::{interval, sleep},
};
pub mod parse_nmea;

/// Holds the calculated horizontal accuracy metrics in meters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HorizontalAccuracy {
    pub lat_err: f64,
    pub lon_err: f64,
    pub drms: f64,
    pub twice_drms: f64,
}

// We use OnceLock so the Regex is only compiled once for maximum loop performance
static GST_REGEX: OnceLock<Regex> = OnceLock::new();

/// Parses any GST NMEA string ($GPGST, $GNGST, $GLGST) and returns the accuracy.
pub fn parse_gst_accuracy(raw_sentence: &str) -> Option<HorizontalAccuracy> {
    // 1. Get or initialize the compiled Regex
    // This pattern skips the first 5 comma-separated values and explicitly captures fields 6 and 7
    let re = GST_REGEX.get_or_init(|| {
        Regex::new(r"^\$[A-Z]{2}GST,(?:[^,]*,){5}([^,]+),([^,]+),[^*]+\*[0-9A-Fa-f]{2}").unwrap()
    });

    // 2. Execute the Regex against the incoming string
    let caps = re.captures(raw_sentence)?;

    // 3. Extract and parse the Latitude (Capture Group 1) and Longitude (Capture Group 2)
    let lat_err: f64 = caps.get(1)?.as_str().parse().ok()?;
    let lon_err: f64 = caps.get(2)?.as_str().parse().ok()?;

    // 4. Calculate DRMS and 2DRMS
    let drms = (lat_err.powi(2) + lon_err.powi(2)).sqrt();
    let twice_drms = drms * 2.0;

    Some(HorizontalAccuracy {
        lat_err,
        lon_err,
        drms,
        twice_drms,
    })
}

pub async fn read_nmea_and_broadcast(nmea_tx: tokio::sync::broadcast::Sender<String>) {
    let mut usb_reader = connect_to_device()
        .await
        .expect("Failed to connect to GPS device");
    tokio::spawn(async move {
        let mut buf = [0u8; 256];

        // 1. Set up a ticker that fires exactly every 500ms
        let mut ticker = interval(Duration::from_millis(500));

        // 2. Create a temporary vector to hold the sentences we collect during that window
        let mut collected_sentences: Vec<String> = Vec::new();
        let mut line_buffer = String::new();

        loop {
            tokio::select! {
                // --- BRANCH 1: The 500ms Timer ---
                _ = ticker.tick() => {
                    // When 500ms passes, check if we collected anything
                    if !collected_sentences.is_empty() {
                        // Package the entire array into one JSON string
                        let payload = json!(collected_sentences).to_string();

                        // Blast it out to the clients
                        if let Err(e) = nmea_tx.send(payload) {
                            eprintln!("Broadcasting nmea failed: {}", e);
                        }

                        // Clear the vector so it's empty for the next 500ms window
                        collected_sentences.clear();
                    }
                }

                // --- BRANCH 2: The Continuous USB Reader ---
                read_result = usb_reader.read(&mut buf) => {
                    match read_result {
                        Ok(bytes_read) if bytes_read > 0 => {
                            if let Ok(text) = std::str::from_utf8(&buf[..bytes_read]) {
                                line_buffer.push_str(text);

                                while let Some(idx) = line_buffer.find('\n') {
                                    // Extract the sentence and trim any \r or whitespace
                                    let sentence = line_buffer[..=idx].trim().to_string();
                                    line_buffer.drain(..=idx);

                                    // Instead of sending, just push it into our collection bucket
                                    if !sentence.is_empty() {
                                        collected_sentences.push(sentence);
                                    }
                                }
                            }
                        }
                        Ok(0) => {
                            println!("USB stream closed (0 bytes read). Device likely unplugged.");
                            break;
                        }
                        Err(e) => {
                            println!("USB NMEA Stream closed by device: {}", e);
                            break;
                        }
                        _ => {} // Ignore Ok(_) where bytes_read == 0 without EOF, just in case
                    }
                }
            }
        }
    });
}

async fn connect_to_device() -> Result<EndpointRead<Bulk>, anyhow::Error> {
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
    let mut ep_in = 0;
    let mut ep_out = 0;
    for interface in device.active_configuration()?.interfaces() {
        for alt in interface.alt_settings() {
            for ep in alt.endpoints() {
                if ep.transfer_type() == nusb::descriptors::TransferType::Bulk {
                    if ep.direction() == nusb::transfer::Direction::In {
                        ep_in = ep.address();
                    }
                    if ep.direction() == nusb::transfer::Direction::Out {
                        ep_out = ep.address();
                    }
                }
            }
        }
    }

    println!("USB IN: 0x{:02X}, USB OUT: 0x{:02X}", ep_in, ep_out);

    let mut usb_reader = interface.endpoint::<Bulk, In>(ep_in)?.reader(4096);
    Ok(usb_reader)
}
