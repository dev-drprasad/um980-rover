use std::sync::OnceLock;

use nmea::Nmea;
use regex::Regex;

#[derive(serde::Serialize)]
pub struct LiveStatus {
    latitude: Option<f64>,
    longitude: Option<f64>,
    altitude: Option<f32>,
    speed_over_ground: Option<f32>,
    fix_type: Option<nmea::sentences::FixType>,
    satellites: usize,
    fix_satellites: Option<u32>,
    accuracy: Option<HorizontalAccuracy>,
    hdop: Option<f32>,
    vdop: Option<f32>,
    pdop: Option<f32>,
}

pub async fn parse_nmea(messages: Vec<String>) -> Result<LiveStatus, anyhow::Error> {
    let mut nmea_state = Nmea::default();
    let mut live_status = LiveStatus {
        latitude: None,
        longitude: None,
        altitude: None,
        speed_over_ground: None,
        fix_type: None,
        satellites: 0,
        fix_satellites: None,
        accuracy: None,
        hdop: None,
        vdop: None,
        pdop: None,
    };
    for sentence in messages {
        if sentence.starts_with('$') {
            match nmea_state.parse(&sentence) {
                Ok(_) => {
                    if let (Some(lat), Some(lng)) = (nmea_state.latitude, nmea_state.longitude) {
                        live_status.latitude = Some(lat);
                        live_status.longitude = Some(lng);
                    }
                    if let Some(alt) = nmea_state.altitude {
                        live_status.altitude = Some(alt);
                    }
                    if let Some(speed) = nmea_state.speed_over_ground {
                        live_status.speed_over_ground = Some(speed);
                    }

                    if let Some(fix_type) = nmea_state.fix_type() {
                        live_status.fix_type = Some(fix_type);
                    }

                    live_status.satellites = nmea_state.satellites().len();
                    live_status.fix_satellites = nmea_state.fix_satellites();
                    live_status.hdop = nmea_state.hdop();
                    live_status.vdop = nmea_state.vdop;
                    live_status.pdop = nmea_state.pdop;
                }
                Err(e) => {
                    eprintln!("Failed to parse NMEA sentence: {}", e);
                }
            }
        }
        if sentence.contains("GST") {
            if let Some(accuracy) = parse_gst_accuracy(&sentence) {
                live_status.accuracy = Some(accuracy);
            }
        }
    }

    Ok(live_status)
}

/// Holds the calculated horizontal accuracy metrics in meters.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
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
