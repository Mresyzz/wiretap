use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::model::TrafficEntry;

#[derive(Clone)]
pub struct TrafficStore {
    max_capacity: usize,
    entries: Arc<RwLock<VecDeque<TrafficEntry>>>,
    sender: broadcast::Sender<TrafficEntry>,
}

impl TrafficStore {
    pub fn new(max_capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(500);
        Self {
            max_capacity,
            entries: Arc::new(RwLock::new(VecDeque::with_capacity(max_capacity))),
            sender,
        }
    }

    pub async fn add(&self, entry: TrafficEntry) {
        {
            let mut w = self.entries.write().await;
            if w.len() >= self.max_capacity {
                w.pop_front();
            }
            w.push_back(entry.clone());
        }
        let _ = self.sender.send(entry);
    }

    pub async fn list(&self) -> Vec<TrafficEntry> {
        let r = self.entries.read().await;
        r.iter().cloned().collect()
    }

    pub async fn get(&self, id: &str) -> Option<TrafficEntry> {
        let r = self.entries.read().await;
        r.iter().find(|e| e.id == id).cloned()
    }

    pub async fn clear(&self) {
        let mut w = self.entries.write().await;
        w.clear();
    }

    pub fn subscribe(&self) -> broadcast::Receiver<TrafficEntry> {
        self.sender.subscribe()
    }
}
