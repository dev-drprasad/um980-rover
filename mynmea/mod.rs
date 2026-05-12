use nusb::{
    Device, MaybeFuture,
    io::{EndpointRead, EndpointWrite},
    list_devices,
    transfer::{Bulk, ControlOut, ControlType, In, Out, Recipient},
};
use serde_json::json;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::{io::AsyncReadExt, time::interval};
pub mod parse_nmea;

pub async fn read_nmea_and_broadcast(
    nmea_tx: tokio::sync::broadcast::Sender<String>,
    rtcm_tx: tokio::sync::broadcast::Sender<Vec<u8>>,
) {
    let (mut usb_reader, mut usb_writer) = connect_to_device()
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

    let mut rtcm_receiver = rtcm_tx.subscribe();
    _ = tokio::spawn(async move {
        while let Ok(msg) = rtcm_receiver.recv().await {
            if let Err(e) = usb_writer.write_all(&msg).await {
                println!("Failed to write RTCM to USB: {}", e);
                break;
            }
            // Force the buffer to push to the hardware immediately
            if let Err(e) = usb_writer.flush().await {
                // handle `hardware fault or protocol violation` and try `usbreset` command using `nusb`
                println!("Failed to flush USB writer: {}", e);
                break;
            }
            println!("Received {} bytes of RTCM, forwarded to USB.", msg.len());
        }
    })
}

async fn connect_to_device() -> Result<(EndpointRead<Bulk>, EndpointWrite<Bulk>), anyhow::Error> {
    // 1. Find the CH340 device (VID: 0x1A86, PID: 0x7523)
    // Note: list_devices() is usually synchronous in nusb, so we just iterate it.
    let devices_info = list_devices()
        .await?
        .filter(|dev| dev.vendor_id() == 0x1a86 && dev.product_id() == 0x7523)
        .collect::<Vec<_>>();

    let device_count = devices_info.len();
    if device_count == 0 {
        return Err(anyhow::anyhow!(
            "No CH340 device found. Please plug in your GPS module."
        ));
    }
    println!("Found {} devices", device_count);

    let mut device: Option<Device> = None;
    for d in devices_info {
        match d.open().wait() {
            Ok(d) => {
                device = Some(d);
                break;
            }
            Err(e) => {
                eprintln!("Error opening device: {}", e);
                continue;
            }
        }
    }

    // 2. Open the device
    let device = match device {
        Some(d) => d,
        None => return Err(anyhow::anyhow!("Failed to open any CH340 device.")),
    };

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

    let usb_reader = interface.endpoint::<Bulk, In>(ep_in)?.reader(4096);
    let usb_writer = interface.endpoint::<Bulk, Out>(ep_out)?.writer(4096);
    Ok((usb_reader, usb_writer))
}
