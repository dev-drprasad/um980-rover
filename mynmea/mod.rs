use nusb::{
    Device, DeviceId, DeviceInfo, MaybeFuture,
    hotplug::HotplugEvent,
    io::{EndpointRead, EndpointWrite},
    list_devices,
    transfer::{Bulk, ControlOut, ControlType, In, Out, Recipient},
};
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::{io::AsyncReadExt, time::interval};
pub mod parse_nmea;
use futures::executor;
use nusb::watch_devices;

const INITIALIZE_CHIP: ControlOut = ControlOut {
    control_type: ControlType::Vendor,
    recipient: Recipient::Device,
    request: 0xA1,
    value: 0x0000,
    index: 0x0000,
    data: &[],
};

const BAUD_RATE_CONTROL: ControlOut = ControlOut {
    control_type: ControlType::Vendor,
    recipient: Recipient::Device,
    request: 0x9A,
    value: 0x1312,
    index: 0xCC03,
    data: &[],
};

const LINE_CONTROL: ControlOut = ControlOut {
    control_type: ControlType::Vendor,
    recipient: Recipient::Device,
    request: 0x9A,
    value: 0x2518,
    index: 0x00C3,
    data: &[],
};

pub fn listen_for_device_changes() {
    println!("Watching USB devices...");

    println!("Listening for USB events (Blocking)...");

    // Initialize the hotplug watcher
    let watcher = nusb::watch_devices().expect("Failed to initialize device watcher");

    let mut blocking_stream = executor::block_on_stream(watcher);

    // Iterate over the stream synchronously
    for event in blocking_stream {
        match event {
            HotplugEvent::Connected(device) => {
                // device is of type DeviceInfo
                println!(
                    "🔌 DEVICE CONNECTED: VID {:04x}:{:04x} (Bus: {}, Addr: {})",
                    device.vendor_id(),
                    device.product_id(),
                    device.bus_id(),
                    device.device_address()
                );

                // You can get additional info like manufacturer strings if available
                if let Some(name) = device.product_string() {
                    println!("   -> Product: {}", name);
                }
            }
            HotplugEvent::Disconnected(device_id) => {
                // device_id is an opaque DeviceId
                println!("❌ DEVICE DISCONNECTED: ID {:?}", device_id);
            }
        }
    }
}

pub async fn read_nmea_and_broadcast(
    nmea_tx: tokio::sync::broadcast::Sender<String>,
    rtcm_tx: tokio::sync::broadcast::Sender<Vec<u8>>,
) {
    loop {
        let (mut usb_reader, mut usb_writer) = loop {
            match connect_to_device().await {
                Ok(io) => break io,
                Err(e) => {
                    println!(
                        "Failed to connect to GPS device: {}. Retrying in 10 seconds...",
                        e
                    );
                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
            }
        };

        let nmea_tx = nmea_tx.clone();

        let read_task = async {
            let mut buf = [0u8; 256];
            let mut ticker = interval(Duration::from_millis(500));

            let mut collected_sentences: Vec<String> = Vec::new();
            let mut line_buffer = String::new();

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if !collected_sentences.is_empty() {
                            let payload = json!(collected_sentences).to_string();
                            if let Err(e) = nmea_tx.send(payload) {
                                eprintln!("Broadcasting nmea failed: {}", e);
                            }
                            collected_sentences.clear();
                        }
                    }

                    read_result = usb_reader.read(&mut buf) => {
                        match read_result {
                            Ok(bytes_read) if bytes_read > 0 => {
                                if let Ok(text) = std::str::from_utf8(&buf[..bytes_read]) {
                                    line_buffer.push_str(text);

                                    while let Some(idx) = line_buffer.find('\n') {
                                        let sentence = line_buffer[..=idx].trim().to_string();
                                        line_buffer.drain(..=idx);
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
                            _ => {}
                        }
                    }
                }
            }
        };

        let mut rtcm_receiver = rtcm_tx.subscribe();
        let write_task = async {
            while let Ok(msg) = rtcm_receiver.recv().await {
                if let Err(e) = usb_writer.write_all(&msg).await {
                    println!("Failed to write RTCM to USB: {}", e);
                    break;
                }
                if let Err(e) = usb_writer.flush().await {
                    println!("Failed to flush USB writer: {}", e);
                    break;
                }
                // println!("Received {} bytes of RTCM, forwarded to USB.", msg.len());
            }
        };

        tokio::select! {
            _ = read_task => {
                println!("USB Reader disconnected.");
            }
            _ = write_task => {
                println!("USB Writer disconnected.");
            }
        }

        println!("Retrying USB connection in 10 seconds...");
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
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

    // A. Initialize the chip
    interface
        .control_out(INITIALIZE_CHIP, Duration::from_millis(100))
        .wait()?;

    // B. Set the Baud Rate
    // Most standard GPS modules use 9600. If yours is newer, it might be 115200.
    // 9600 Baud   = index: 0xB202
    // 115200 Baud = index: 0xCC03
    interface
        .control_out(BAUD_RATE_CONTROL, Duration::from_millis(100))
        .wait()?;

    // C. Set Line Control (8 data bits, No parity, 1 stop bit) & Enable UART
    interface
        .control_out(LINE_CONTROL, Duration::from_millis(100))
        .wait()?;

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

    let usb_reader = interface.endpoint::<Bulk, In>(ep_in)?.reader(4096);
    let usb_writer = interface.endpoint::<Bulk, Out>(ep_out)?.writer(4096);
    Ok((usb_reader, usb_writer))
}
