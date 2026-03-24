pub async fn connect_to_device(tx: broadcast::Sender<String>) -> Result<(), anyhow::Error> {
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

    ntrip_client::ntrip_client(interface.clone(), interface.clone(), ep_out, ep_in, tx).await?;
    // parse_nmea::parse_nmea(interface, ep_in, &tx);

    Ok(())
}
