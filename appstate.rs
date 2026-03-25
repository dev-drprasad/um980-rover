use tokio::sync::Mutex;
use tokio::sync::broadcast;

pub struct AppState {
    pub rtcm_tx: broadcast::Sender<Vec<u8>>,
    pub nmea_tx: broadcast::Sender<String>,
    pub file_lock: Mutex<()>,
}
