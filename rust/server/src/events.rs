use serde::Serialize;
use serde_json::Value;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize)]
pub struct AppEvent {
    pub event_type: String,
    pub payload: Value,
    pub timestamp: String,
}

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<AppEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn publish<T: Serialize>(
        &self,
        event_type: &str,
        payload: &T,
    ) {
        let payload = serde_json::to_value(payload).unwrap_or_else(|_| serde_json::json!({}));
        let _ = self.tx.send(AppEvent {
            event_type: event_type.to_string(),
            payload,
            timestamp: crate::db::now_rfc3339(),
        });
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.tx.subscribe()
    }
}
