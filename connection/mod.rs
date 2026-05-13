use anyhow::Result;
use bluer::adv::Advertisement;
use bluer::gatt::local::{
    Application, Characteristic, CharacteristicNotify, CharacteristicRead, Service,
};
use bluer::{AdapterEvent, Session};
use local_ip_address::local_ip;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

const SERVICE_UUID: &str = "12345678-1234-5678-1234-56789abcdef0";
const CHAR_UUID: &str = "abcdef01-1234-5678-1234-56789abcdef0";

pub async fn broadcast_ip() -> Result<()> {
    let session = Session::new().await?;
    let adapter = session.default_adapter().await?;

    adapter.set_powered(true).await?;

    println!("Bluetooth adapter: {}", adapter.name());
    println!("Starting BLE GATT server...");

    let ip_string = Arc::new(Mutex::new(get_ip_string()));

    // Build GATT application
    let mut app = Application {
        services: Vec::new(),
        ..Default::default()
    };

    let service_uuid = Uuid::parse_str(SERVICE_UUID)?;
    let char_uuid = Uuid::parse_str(CHAR_UUID)?;

    let ip_clone = ip_string.clone();

    let service = Service {
        uuid: service_uuid,
        primary: true,
        characteristics: vec![Characteristic {
            uuid: char_uuid,

            read: Some(CharacteristicRead {
                read: true,

                fun: Box::new(move |_req| {
                    let ip_clone = ip_clone.clone();

                    Box::pin(async move {
                        let ip = ip_clone.lock().await.clone();

                        Ok(ip.into_bytes())
                    })
                }),

                ..Default::default()
            }),

            notify: Some(CharacteristicNotify {
                notify: true,
                ..Default::default()
            }),
            ..Default::default()
        }],
        ..Default::default()
    };

    app.services.insert(0, service);

    let _app_handle = adapter.serve_gatt_application(app).await?;

    // Advertise
    let adv = Advertisement {
        service_uuids: vec![service_uuid].into_iter().collect(),
        local_name: Some("RadxaZero3W".to_string()),
        discoverable: Some(true),
        ..Default::default()
    };

    let _adv_handle = adapter.advertise(adv).await?;

    println!("BLE advertising started.");
    println!("Current IP: {}", get_ip_string());

    // Keep running
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

        let new_ip = get_ip_string();
        let mut stored_ip = ip_string.lock().await;

        if *stored_ip != new_ip {
            *stored_ip = new_ip.clone();
            println!("IP updated: {}", new_ip);
        }
    }
}

fn get_ip_string() -> String {
    match local_ip() {
        Ok(ip) => ip.to_string(),
        Err(_) => "0.0.0.0".to_string(),
    }
}
