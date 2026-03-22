use tokio::sync::Mutex;
use tokio::sync::broadcast;

pub struct AppState {
    pub tx: broadcast::Sender<String>,
    pub file_lock: Mutex<()>,
}
