use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use thiserror::Error;
use tokio::sync::{mpsc, watch};

use crate::config::ConfigurationSnapshot;
use crate::messages::{DashboardSnapshot, RunSnapshot, ServerEvent, ServerMessage};

const EVENT_REPLAY_CAPACITY: usize = 128;
const OUTBOUND_QUEUE_CAPACITY: usize = 64;
const TOKEN_BYTES: usize = 32;
const TOKEN_TTL: Duration = Duration::from_secs(30);

/// Issues short-lived bootstrap credentials and consumes each credential once.
pub struct BootstrapTokenStore {
    tokens: Mutex<HashMap<String, Instant>>,
    ttl: Duration,
}

impl BootstrapTokenStore {
    pub fn new() -> Self {
        Self::with_ttl(TOKEN_TTL)
    }

    pub fn consume(&self, token: &str) -> Result<bool, ConnectionError> {
        let now = Instant::now();
        let mut tokens = self.tokens.lock().map_err(|_| ConnectionError::Lock)?;
        tokens.retain(|_, expires_at| *expires_at > now);
        Ok(tokens
            .remove(token)
            .is_some_and(|expires_at| expires_at > now))
    }

    pub fn issue(&self) -> Result<String, ConnectionError> {
        let mut bytes = [0_u8; TOKEN_BYTES];
        getrandom::fill(&mut bytes).map_err(|error| ConnectionError::Random(error.to_string()))?;
        let token = bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let now = Instant::now();
        let mut tokens = self.tokens.lock().map_err(|_| ConnectionError::Lock)?;
        tokens.retain(|_, expires_at| *expires_at > now);
        tokens.insert(token.clone(), now + self.ttl);
        Ok(token)
    }

    fn with_ttl(ttl: Duration) -> Self {
        Self {
            tokens: Mutex::new(HashMap::new()),
            ttl,
        }
    }
}

impl Default for BootstrapTokenStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Owns bounded per-tab queues and a bounded replay window for shared events.
pub struct ConnectionHub {
    connections: Mutex<HashMap<u64, mpsc::Sender<ServerMessage>>>,
    events: Mutex<VecDeque<ServerMessage>>,
    next_connection_id: AtomicU64,
    next_event_sequence: AtomicU64,
}

impl ConnectionHub {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            connections: Mutex::new(HashMap::new()),
            events: Mutex::new(VecDeque::with_capacity(EVENT_REPLAY_CAPACITY)),
            next_connection_id: AtomicU64::new(1),
            next_event_sequence: AtomicU64::new(1),
        })
    }

    pub fn current_event_sequence(&self) -> u64 {
        self.next_event_sequence.load(Ordering::Acquire) - 1
    }

    pub fn publish_configuration(
        &self,
        snapshot: ConfigurationSnapshot,
    ) -> Result<(), ConnectionError> {
        self.publish_event(ServerEvent::ConfigurationReloaded {
            configuration: snapshot.transport(),
        })
    }

    pub fn publish_dashboard(&self, snapshot: DashboardSnapshot) -> Result<(), ConnectionError> {
        self.publish_event(ServerEvent::DashboardUpdated {
            dashboard: snapshot,
        })
    }

    pub fn publish_run(&self, connection_id: u64, run: RunSnapshot) -> Result<(), ConnectionError> {
        let sequence = self.next_event_sequence.fetch_add(1, Ordering::AcqRel);
        let message = ServerMessage::Event {
            event: ServerEvent::RunUpdated { run },
            event_id: format!("event-{sequence}"),
            sequence,
        };
        let mut connections = self.connections.lock().map_err(|_| ConnectionError::Lock)?;
        let should_remove = connections
            .get(&connection_id)
            .is_none_or(|sender| sender.try_send(message).is_err());
        if should_remove {
            connections.remove(&connection_id);
        }
        Ok(())
    }

    fn publish_event(&self, event: ServerEvent) -> Result<(), ConnectionError> {
        let sequence = self.next_event_sequence.fetch_add(1, Ordering::AcqRel);
        let message = ServerMessage::Event {
            event,
            event_id: format!("event-{sequence}"),
            sequence,
        };

        let mut events = self.events.lock().map_err(|_| ConnectionError::Lock)?;
        if events.len() == EVENT_REPLAY_CAPACITY {
            events.pop_front();
        }
        events.push_back(message.clone());
        drop(events);

        let mut connections = self.connections.lock().map_err(|_| ConnectionError::Lock)?;
        connections.retain(|_, sender| sender.try_send(message.clone()).is_ok());
        Ok(())
    }

    pub fn register(&self) -> Result<RegisteredConnection, ConnectionError> {
        let connection_id = self.next_connection_id.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
        self.connections
            .lock()
            .map_err(|_| ConnectionError::Lock)?
            .insert(connection_id, sender);
        Ok(RegisteredConnection {
            id: connection_id,
            receiver,
        })
    }

    pub fn replay_after(
        &self,
        sequence: Option<u64>,
    ) -> Result<Vec<ServerMessage>, ConnectionError> {
        let Some(sequence) = sequence else {
            return Ok(Vec::new());
        };
        let events = self.events.lock().map_err(|_| ConnectionError::Lock)?;
        let Some(first_sequence) = events.front().and_then(event_sequence) else {
            return Ok(Vec::new());
        };
        if sequence.saturating_add(1) < first_sequence {
            return Ok(Vec::new());
        }
        Ok(events
            .iter()
            .filter(|event| event_sequence(event).is_some_and(|current| current > sequence))
            .cloned()
            .collect())
    }

    pub fn send(&self, connection_id: u64, message: ServerMessage) {
        let Ok(mut connections) = self.connections.lock() else {
            return;
        };
        let should_remove = connections
            .get(&connection_id)
            .is_none_or(|sender| sender.try_send(message).is_err());
        if should_remove {
            connections.remove(&connection_id);
        }
    }

    pub fn unregister(&self, connection_id: u64) {
        if let Ok(mut connections) = self.connections.lock() {
            connections.remove(&connection_id);
        }
    }

    pub async fn publish_config_events(
        self: Arc<Self>,
        mut snapshots: watch::Receiver<Option<ConfigurationSnapshot>>,
    ) {
        while snapshots.changed().await.is_ok() {
            let snapshot = snapshots.borrow_and_update().clone();
            if let Some(snapshot) = snapshot
                && let Err(error) = self.publish_configuration(snapshot)
            {
                tracing::error!(%error, "could not publish configuration event");
            }
        }
    }

    pub async fn publish_dashboard_events(
        self: Arc<Self>,
        mut snapshots: watch::Receiver<Option<DashboardSnapshot>>,
    ) {
        while snapshots.changed().await.is_ok() {
            let snapshot = snapshots.borrow_and_update().clone();
            if let Some(snapshot) = snapshot
                && let Err(error) = self.publish_dashboard(snapshot)
            {
                tracing::error!(%error, "could not publish dashboard event");
            }
        }
    }
}

pub struct RegisteredConnection {
    pub id: u64,
    pub receiver: mpsc::Receiver<ServerMessage>,
}

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("connection state lock was poisoned")]
    Lock,
    #[error("could not generate a bootstrap token: {0}")]
    Random(String),
}

fn event_sequence(message: &ServerMessage) -> Option<u64> {
    match message {
        ServerMessage::Event { sequence, .. } => Some(*sequence),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::BootstrapTokenStore;

    #[test]
    fn bootstrap_tokens_are_single_use_and_expire() {
        let tokens = BootstrapTokenStore::new();
        let token = tokens.issue().unwrap();

        assert_eq!(token.len(), 64);
        assert!(tokens.consume(&token).unwrap());
        assert!(!tokens.consume(&token).unwrap());

        let expired_tokens = BootstrapTokenStore::with_ttl(Duration::ZERO);
        let expired = expired_tokens.issue().unwrap();
        assert!(!expired_tokens.consume(&expired).unwrap());
    }
}
