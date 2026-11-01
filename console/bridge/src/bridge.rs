//! Per-stream sequencing, latest-frame replay and lossy fan-out.
use prost::Message;
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast::{self, error::RecvError};

pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/doramcap.console.live.v1.rs"));
}

/// Frames buffered per client before it observes a sequence gap.
const CLIENT_BACKLOG: usize = 64;

/// One encoded `Frame` with its routing fields.
#[derive(Clone, Debug)]
pub struct Encoded {
    pub stream: Arc<str>,
    pub sequence: u64,
    pub bytes: Arc<Vec<u8>>,
}

/// Shared frame source. Publishing never blocks on clients.
pub struct Bridge {
    max_payload_bytes: usize,
    state: Mutex<State>,
    sender: broadcast::Sender<Encoded>,
}
#[derive(Default)]
struct State {
    sequences: HashMap<Arc<str>, u64>,
    latest: BTreeMap<Arc<str>, Encoded>,
}

impl Bridge {
    /// Relay payloads up to `max_payload_bytes`; Console supplies its transport limit.
    pub fn new(max_payload_bytes: usize) -> Self {
        Self {
            max_payload_bytes,
            state: Mutex::default(),
            sender: broadcast::channel(CLIENT_BACKLOG).0,
        }
    }

    /// Assign the stream's next sequence, retain the frame as latest and fan it out.
    /// Returns false when the payload exceeds the size limit and was skipped.
    pub fn publish(&self, stream: &str, payload: &[u8], received_unix_ns: u64) -> bool {
        let mut state = self.state.lock().unwrap();
        let key: Arc<str> = state
            .sequences
            .get_key_value(stream)
            .map_or_else(|| stream.into(), |(key, _)| key.clone());
        let sequence = state.sequences.entry(key.clone()).or_default();
        *sequence += 1;
        let sequence = *sequence;
        if payload.len() > self.max_payload_bytes {
            return false;
        }
        let frame = pb::Frame {
            stream: stream.into(),
            sequence,
            received_unix_ns,
            payload: payload.to_vec(),
        };
        let encoded = Encoded {
            stream: key.clone(),
            sequence,
            bytes: Arc::new(frame.encode_to_vec()),
        };
        state.latest.insert(key, encoded.clone());
        // Sent under the lock so subscriptions never miss a frame between replay and live.
        let _ = self.sender.send(encoded);
        true
    }

    /// Start a client view: latest frame of every stream, then live frames.
    pub fn subscribe(&self) -> Subscription {
        let state = self.state.lock().unwrap();
        Subscription {
            replay: state.latest.values().cloned().collect(),
            live: self.sender.subscribe(),
            sent: HashMap::new(),
        }
    }
}

/// Ordered client view; a lagging client skips frames and sees sequence gaps.
pub struct Subscription {
    replay: std::collections::VecDeque<Encoded>,
    live: broadcast::Receiver<Encoded>,
    sent: HashMap<Arc<str>, u64>,
}

impl Subscription {
    /// Next frame not yet delivered to this client; `None` after the bridge closes.
    pub async fn next(&mut self) -> Option<Arc<Vec<u8>>> {
        loop {
            let frame = match self.replay.pop_front() {
                Some(frame) => frame,
                None => match self.live.recv().await {
                    Ok(frame) => frame,
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => return None,
                },
            };
            let sent = self.sent.entry(frame.stream.clone()).or_default();
            if frame.sequence > *sent {
                *sent = frame.sequence;
                return Some(frame.bytes);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const LIMIT: usize = 16;

    fn decode(bytes: Arc<Vec<u8>>) -> pb::Frame {
        pb::Frame::decode(bytes.as_slice()).unwrap()
    }

    #[tokio::test]
    async fn new_clients_receive_latest_frame_per_stream_then_live_frames() {
        // Arrange
        let bridge = Bridge::new(LIMIT);
        bridge.publish("cpu", b"a", 1);
        bridge.publish("cpu", b"b", 2);
        bridge.publish("faults", b"c", 3);
        let mut client = bridge.subscribe();

        // Act
        bridge.publish("cpu", b"d", 4);
        let frames: Vec<_> = [
            client.next().await,
            client.next().await,
            client.next().await,
        ]
        .into_iter()
        .map(|frame| decode(frame.unwrap()))
        .collect();

        // Assert
        let seen: Vec<_> = frames
            .iter()
            .map(|f| (f.stream.as_str(), f.sequence, f.payload.as_slice()))
            .collect();
        assert_eq!(
            seen,
            [
                ("cpu", 2, &b"b"[..]),
                ("faults", 1, &b"c"[..]),
                ("cpu", 3, &b"d"[..])
            ]
        );
        assert_eq!(frames[0].received_unix_ns, 2);
    }

    #[tokio::test]
    async fn lagging_clients_skip_frames_and_observe_gaps() {
        // Arrange
        let bridge = Bridge::new(LIMIT);
        let mut client = bridge.subscribe();

        // Act
        for index in 0..(CLIENT_BACKLOG as u64 + 10) {
            bridge.publish("cpu", &index.to_le_bytes(), index);
        }
        let first = decode(client.next().await.unwrap());

        // Assert
        assert!(first.sequence > 1);
    }

    #[tokio::test]
    async fn oversized_payloads_are_skipped_with_a_sequence_gap() {
        // Arrange
        let bridge = Bridge::new(LIMIT);
        let mut client = bridge.subscribe();

        // Act
        let oversized = bridge.publish("cpu", &[0; LIMIT + 1], 1);
        bridge.publish("cpu", b"ok", 2);

        // Assert
        assert!(!oversized);
        assert_eq!(decode(client.next().await.unwrap()).sequence, 2);
    }
}
