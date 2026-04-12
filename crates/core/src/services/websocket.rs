use crate::events::SystemEvent;
use serde_json::Error as SerdeError;
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tokio::sync::{
    Mutex, RwLock, broadcast,
    mpsc::{self, UnboundedReceiver, UnboundedSender},
};
use tracing::{debug, error, info, warn};

pub type ClientId = u64;

/// Broadcasts serialized `SystemEvent`s to all registered WebSocket clients.
pub struct WebSocketBroadcaster {
    event_rx: Mutex<Option<broadcast::Receiver<SystemEvent>>>,
    clients: Arc<RwLock<HashMap<ClientId, UnboundedSender<String>>>>,
    next_client_id: AtomicU64,
}

impl WebSocketBroadcaster {
    pub fn new(event_rx: broadcast::Receiver<SystemEvent>) -> Self {
        Self {
            event_rx: Mutex::new(Some(event_rx)),
            clients: Arc::new(RwLock::new(HashMap::new())),
            next_client_id: AtomicU64::new(1),
        }
    }

    /// Main loop that receives events from the bus and fan-outs to connected clients.
    pub async fn run(self: Arc<Self>) {
        info!("WebSocket broadcaster service started");

        let mut event_rx = {
            let mut guard = self.event_rx.lock().await;
            guard
                .take()
                .expect("WebSocketBroadcaster::run must only be called once")
        };

        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    if let Err(err) = self.broadcast_event(event).await {
                        error!(error = ?err, "Failed to serialize event for WebSocket clients");
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(skipped, "WebSocket broadcaster lagged; events dropped");
                }
                Err(broadcast::error::RecvError::Closed) => {
                    info!("Event bus closed; stopping WebSocket broadcaster");
                    break;
                }
            }
        }
    }

    async fn broadcast_event(&self, event: SystemEvent) -> Result<(), SerdeError> {
        let payload = serde_json::to_string(&event)?;
        let mut stale_clients = Vec::new();

        {
            let clients = self.clients.read().await;
            for (&client_id, sender) in clients.iter() {
                if sender.send(payload.clone()).is_err() {
                    stale_clients.push(client_id);
                }
            }
            info!(
                client_count = clients.len(),
                "WebSocket broadcasted event to clients"
            );
        }

        if !stale_clients.is_empty() {
            let mut clients = self.clients.write().await;
            for client_id in stale_clients {
                if clients.remove(&client_id).is_some() {
                    debug!(client_id, "Removed stale WebSocket client");
                }
            }
        }

        Ok(())
    }

    pub async fn add_client(&self) -> (ClientId, UnboundedReceiver<String>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let client_id = self.next_client_id.fetch_add(1, Ordering::Relaxed);

        {
            let mut clients = self.clients.write().await;
            clients.insert(client_id, tx);
        }

        info!(client_id, "Registered WebSocket client");
        (client_id, rx)
    }

    pub async fn remove_client(&self, client_id: ClientId) {
        let removed = {
            let mut clients = self.clients.write().await;
            clients.remove(&client_id)
        };

        if removed.is_some() {
            info!(client_id, "Removed WebSocket client");
        }
    }
}
