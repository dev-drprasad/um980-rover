use std::sync::Arc;

use axum::extract::{
    State, WebSocketUpgrade,
    ws::{Message, WebSocket},
};
use futures::{SinkExt, StreamExt};
use serde_json::json;

use crate::appstate;
use crate::mynmea::parse_nmea::parse_nmea;

pub async fn send_live_status_to_client(
    ws: WebSocketUpgrade,
    State(state): State<Arc<appstate::AppState>>,
) -> axum::response::Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<appstate::AppState>) {
    println!("A new client connected!");

    // 1. Split the WebSocket into a Send half and a Receive half
    let (mut socket_sender, mut socket_receiver) = socket.split();

    // 2. Subscribe this specific client to the global broadcast channel
    let mut broadcast_rx = state.nmea_tx.subscribe();

    // 3. TASK 1: The "Writing" Task
    // This background task listens for messages on the global broadcast channel
    // and pushes them down the WebSocket to the client.
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            if let Ok(nmea_messages) = serde_json::from_str::<Vec<String>>(&msg) {
                match parse_nmea(nmea_messages).await {
                    Ok(live_status) => {
                        let json =
                            json!({ "event": "live_status", "data": live_status }).to_string();
                        if let Err(e) = socket_sender.send(Message::Text(json)).await {
                            eprintln!("Failed to send message to client: {}", e);
                        };
                    }
                    Err(e) => eprintln!("Failed to parse NMEA messages: {}", e),
                }
            }
        }
    });

    // 4. TASK 2: The "Reading" Task
    // This background task listens to the client's WebSocket. When the client
    // sends a message, it forwards it to the global broadcast channel.
    let broadcast_tx = state.tx.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = socket_receiver.next().await {
            // Send the client's message to everyone else!
            let _ = broadcast_tx.send(format!("User says: {}", text));
        }
    });

    // 5. The Concurrency Manager
    // Wait until either the sending task or receiving task finishes.
    // If a client disconnects, `recv_task` finishes. We then abort the `send_task`
    // so we don't leak memory keeping a dead connection alive.
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    println!("A client disconnected.");
}
