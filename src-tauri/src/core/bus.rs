//! Interface Bus -- all inter-module communication routes through here.
//! Modules must never call each other directly.

use std::sync::Arc;
use tokio::sync::broadcast;

pub const BUS_CAPACITY: usize = 256;

#[derive(Debug, Clone)]
pub struct BusMessage {
    pub event_type: String,
    pub source: String,
    pub payload: String,
}

pub struct Bus {
    sender: broadcast::Sender<BusMessage>,
}

impl Bus {
    pub fn new() -> Arc<Self> {
        let (sender, _) = broadcast::channel(BUS_CAPACITY);
        Arc::new(Self { sender })
    }

    pub fn publish(&self, msg: BusMessage) -> Result<(), broadcast::error::SendError<BusMessage>> {
        self.sender.send(msg).map(|_| ())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<BusMessage> {
        self.sender.subscribe()
    }
}

impl Default for Bus {
    fn default() -> Self {
        let (sender, _) = broadcast::channel(BUS_CAPACITY);
        Self { sender }
    }
}