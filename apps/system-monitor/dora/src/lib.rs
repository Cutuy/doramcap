use dora_node_api::{DoraNode, Event, EventStream, MetadataParameters, Parameter};
use eyre::{Result, eyre};
use futures::{FutureExt, channel::oneshot};
use prost::Message;
use std::thread::JoinHandle;

/// Owns Dora exclusively on the publication thread. Calls may allocate/block.
pub struct Publisher {
    node: Option<DoraNode>,
    cancel: Option<oneshot::Sender<()>>,
    receiver: Option<JoinHandle<()>>,
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
        mut events: EventStream,
        notify: impl FnOnce(std::result::Result<(), String>) + Send + 'static,
    ) -> Result<Self> {
        let (cancel, cancel_rx) = oneshot::channel();
        let receiver = std::thread::Builder::new().name("monitor-dora-events".into()).spawn(move || {
            let result = futures::executor::block_on(async {
                let mut cancel = cancel_rx.fuse();
                loop {
                    futures::select_biased! {
                        _ = cancel => return None,
                        event = events.recv_async().fuse() => match event {
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
    fn testing_publisher(
        delay: f64,
        notify: impl FnOnce(std::result::Result<(), String>) + Send + 'static,
    ) -> Publisher {
        use dora_node_api::integration_testing::integration_testing_format::{
            IncomingEvent, TimedIncomingEvent,
        };
        use dora_node_api::integration_testing::{
            IntegrationTestInput, TestingInput, TestingOutput, output_channel,
        };
        let input = IntegrationTestInput::new(
            "monitor-test".parse().unwrap(),
            vec![TimedIncomingEvent {
                time_offset_secs: delay,
                event: IncomingEvent::Stop,
            }],
        );
        let (output, _outputs) = output_channel();
        let (node, events) = DoraNode::init_testing(
            TestingInput::Input(input),
            TestingOutput::ToChannel(output),
            Default::default(),
        )
        .unwrap();
        Publisher::with_events(node, events, notify).unwrap()
    }

    #[test]
    fn dora_stop_notifies_the_publisher() {
        // Arrange
        let (notify, events) = std::sync::mpsc::channel();
        let mut publisher = testing_publisher(0.0, move |event| {
            notify.send(event).unwrap();
        });

        // Act
        let event = events
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        publisher.finish().unwrap();

        // Assert
        assert!(event.is_ok());
        assert!(publisher.receiver.is_none());
    }

    #[test]
    fn finish_cancels_a_receiver_waiting_for_a_future_event() {
        // Arrange
        let (done, result) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let mut publisher =
                testing_publisher(30.0, |_| panic!("canceled receiver must not notify stop"));

            // Act
            publisher.finish().unwrap();
            done.send(()).unwrap();
        });

        // Assert: completion occurs before the scheduled Dora event.
        result
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
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
