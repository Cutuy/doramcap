use dora_node_api::{DoraNode, Event, MetadataParameters, Parameter};
use eyre::{Result, eyre};
use futures::{FutureExt, Stream, StreamExt, channel::oneshot};
use prost::Message;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const SAMPLING_HEALTH_TIMEOUT: Duration = Duration::from_secs(1);
const HEALTH_INTERVAL: Duration = Duration::from_millis(100);

/// Owns Dora exclusively on the publication thread. Calls may allocate/block.
pub struct Publisher {
    node: Option<DoraNode>,
    cancel: Option<oneshot::Sender<()>>,
    receiver: Option<JoinHandle<()>>,
    health: fault_dora::Health,
    sampling: fault_dora::Check,
    recording: fault_dora::Check,
}
impl Publisher {
    /// Attach using the app environment; notify once on Dora stop, disconnect or error.
    /// Finish and drop cancel and join the event receiver.
    pub fn new(
        notify: impl FnOnce(std::result::Result<(), String>) + Send + 'static,
    ) -> Result<Self> {
        let (node, events) = dora_app::environment::connect()?;
        Self::with_events(node, events, notify)
    }

    fn with_events(
        node: DoraNode,
        mut events: impl Stream<Item = Event> + Unpin + Send + 'static,
        notify: impl FnOnce(std::result::Result<(), String>) + Send + 'static,
    ) -> Result<Self> {
        let health = fault_dora::Health::new()?;
        let sampling =
            health.register("system-monitor", "sampling", Some(SAMPLING_HEALTH_TIMEOUT))?;
        let recording = health.register("system-monitor", "publication", None)?;
        sampling.success()?;
        recording.success()?;
        let (cancel, cancel_rx) = oneshot::channel();
        let receiver = std::thread::Builder::new().name("monitor-dora-events".into()).spawn(move || {
            let result = futures::executor::block_on(async {
                let mut cancel = cancel_rx.fuse();
                loop {
                    futures::select_biased! {
                        _ = cancel => return None,
                        event = events.next().fuse() => match event {
                            Some(Event::Stop(_)) | None => return Some(Ok(())),
                            Some(Event::Error(error)) => return Some(Err(format!("Dora event error: {error}"))),
                            _ => {},
                        },
                    }
                }
            });
            if let Some(result) = result { notify(result); }
        })?;
        Ok(Self {
            health,
            sampling,
            recording,
            node: Some(node),
            cancel: Some(cancel),
            receiver: Some(receiver),
        })
    }

    pub fn publish_packet(
        &mut self,
        sequence: u64,
        capture_unix_ns: u64,
        packet: &telemetry_protocol::pb::Packet,
    ) -> Result<()> {
        let mut parameters = MetadataParameters::default();
        parameters.insert("sequence".into(), Parameter::String(sequence.to_string()));
        parameters.insert(
            "capture_unix_ns".into(),
            Parameter::String(capture_unix_ns.to_string()),
        );
        self.publish("snapshots", parameters, packet)
    }

    pub fn publish_dictionary(&mut self, packet: &telemetry_protocol::pb::Packet) -> Result<()> {
        self.publish("dictionary", MetadataParameters::default(), packet)
    }

    fn publish(
        &mut self,
        topic: &str,
        parameters: MetadataParameters,
        packet: &telemetry_protocol::pb::Packet,
    ) -> Result<()> {
        let len = packet.encoded_len();
        eyre::ensure!(
            len <= telemetry_protocol::MAX_FRAME_BYTES,
            "frame exceeds size limit"
        );
        self.node
            .as_mut()
            .ok_or_else(|| eyre!("publisher already finished"))?
            .send_output_raw(topic.into(), parameters, len, |buffer| {
                // The immutable packet is sized before allocation. Any invariant failure
                // panics before send_output_raw can publish a partially encoded buffer.
                encode_into(packet, buffer);
            })?;
        Ok(())
    }

    /// Copy an already encoded storage payload into Dora-owned publication memory.
    pub fn publish_serialized(
        &mut self,
        dictionary: bool,
        sequence: u64,
        time: u64,
        len: usize,
        fill: impl FnOnce(&mut [u8]),
    ) -> Result<()> {
        eyre::ensure!(
            len <= telemetry_protocol::MAX_FRAME_BYTES,
            "frame exceeds size limit"
        );
        let mut parameters = MetadataParameters::default();
        if !dictionary {
            parameters.insert("sequence".into(), Parameter::String(sequence.to_string()));
            parameters.insert(
                "capture_unix_ns".into(),
                Parameter::String(time.to_string()),
            );
        }
        self.node
            .as_mut()
            .ok_or_else(|| eyre!("publisher already finished"))?
            .send_output_raw(
                if dictionary {
                    "dictionary"
                } else {
                    "snapshots"
                }
                .into(),
                parameters,
                len,
                fill,
            )?;
        Ok(())
    }

    pub fn sampling_check(&self) -> fault_dora::Check {
        self.sampling.clone()
    }
    /// Reports and publishes a failure on the `publication` check.
    pub fn report_failure(&mut self, error: &str) -> Result<()> {
        fault_dora::Health::failure(&self.recording, error)?;
        self.tick_health().map(|_| ())
    }
    /// Reports and publishes a collector failure on the `sampling` check.
    pub fn report_sampling_failure(&mut self, error: &str) -> Result<()> {
        fault_dora::Health::failure(&self.sampling, error)?;
        self.tick_health().map(|_| ())
    }
    /// Publishes due health and returns when the host's publication loop must call again.
    pub fn tick_health(&mut self) -> Result<Instant> {
        self.health.tick(
            self.node
                .as_mut()
                .ok_or_else(|| eyre!("publisher already finished"))?,
        )?;
        Ok(Instant::now() + HEALTH_INTERVAL)
    }

    fn stop_events(&mut self) -> Result<()> {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
        if let Some(receiver) = self.receiver.take() {
            receiver
                .join()
                .map_err(|_| eyre!("Dora event receiver panicked"))?;
        }
        Ok(())
    }

    pub fn finish(&mut self) -> Result<()> {
        self.stop_events()?;
        // close_outputs() only sends a control-plane notification. It can overtake
        // the final direct Zenoh/SHM payload. DoraNode::drop tears down the data
        // transport BEFORE notifying the daemon that outputs are closed.
        drop(self.node.take());
        Ok(())
    }
}

impl Drop for Publisher {
    fn drop(&mut self) {
        let _ = self.stop_events();
    }
}

fn encode_into(packet: &telemetry_protocol::pb::Packet, buffer: &mut [u8]) {
    assert_eq!(
        buffer.len(),
        packet.encoded_len(),
        "Dora destination size mismatch"
    );
    let mut destination = buffer;
    packet
        .encode(&mut destination)
        .expect("immutable Protobuf packet sized with encoded_len");
    assert!(
        destination.is_empty(),
        "Protobuf did not fill the destination"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    const TEST_DEADLINE: Duration = Duration::from_secs(2);

    fn testing_node(
        events: Vec<
            dora_node_api::integration_testing::integration_testing_format::TimedIncomingEvent,
        >,
    ) -> (DoraNode, dora_node_api::EventStream) {
        use dora_node_api::integration_testing::{
            IntegrationTestInput, TestingInput, TestingOutput, output_channel,
        };
        let input = IntegrationTestInput::new("monitor-test".parse().unwrap(), events);
        let (output, _outputs) = output_channel();
        DoraNode::init_testing(
            TestingInput::Input(input),
            TestingOutput::ToChannel(output),
            Default::default(),
        )
        .unwrap()
    }

    #[test]
    fn dora_stop_notifies_the_publisher() {
        use dora_node_api::integration_testing::integration_testing_format::{
            IncomingEvent, TimedIncomingEvent,
        };
        // Arrange
        let (node, stream) = testing_node(vec![TimedIncomingEvent {
            time_offset_secs: 0.0,
            event: IncomingEvent::Stop,
        }]);
        let (notify, events) = std::sync::mpsc::channel();
        let mut publisher = Publisher::with_events(node, stream, move |event| {
            notify.send(event).unwrap();
        })
        .unwrap();

        // Act
        let event = events.recv_timeout(TEST_DEADLINE).unwrap();
        publisher.finish().unwrap();

        // Assert
        assert!(event.is_ok());
        assert!(publisher.receiver.is_none());
    }

    #[test]
    fn finish_cancels_a_receiver_waiting_for_an_event() {
        // Arrange
        let (done, result) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let (node, _stream) = testing_node(Vec::new());
            let mut publisher = Publisher::with_events(node, futures::stream::pending(), |_| {
                panic!("canceled receiver must not notify")
            })
            .unwrap();

            // Act
            publisher.finish().unwrap();
            done.send(()).unwrap();
        });

        // Assert: only cancellation can end a wait on a stream that never yields.
        result.recv_timeout(TEST_DEADLINE).unwrap();
        worker.join().unwrap();
    }

    #[test]
    fn direct_destination_preserves_wire_bytes_and_bounds() {
        let packet = telemetry_protocol::dictionary_packet(&telemetry_protocol::DictionaryFrame {
            version: telemetry_protocol::VERSION,
            session_id: "test".into(),
            revision: u64::MAX,
            overflow: 0,
            capture_unix_ns: u64::MAX,
            entries: vec![telemetry_protocol::DictionaryEntry {
                id: u64::MAX,
                parent_id: 0,
                path: Some("/test".into()),
            }],
        });
        let len = packet.encoded_len();
        let mut guarded = vec![0xa5; len + 2];
        encode_into(&packet, &mut guarded[1..=len]);
        assert_eq!(&guarded[1..=len], packet.encode_to_vec());
        assert_eq!(guarded[0], 0xa5);
        assert_eq!(guarded[len + 1], 0xa5);
        assert_eq!(
            telemetry_protocol::decode_dictionary(&guarded[1..=len])
                .unwrap()
                .revision,
            u64::MAX
        );
    }
}
