use dora_node_api::{DoraNode, Event, EventStream, MetadataParameters, Parameter, TryRecvError};
use eyre::{bail, eyre, Result};
use prost::Message;

/// Owns Dora exclusively on the publication thread. Calls may allocate/block.
pub struct Publisher {
    node: Option<DoraNode>,
    events: EventStream,
}
impl Publisher {
    pub fn new_from_env() -> Result<Self> {
        let (node, events) = DoraNode::init_from_env_force()?;
        Ok(Self {
            node: Some(node),
            events,
        })
    }

    pub fn publish_packet(
        &mut self,
        sequence: u64,
        capture_unix_ns: u64,
        packet: &monitor_wire::pb::Packet,
    ) -> Result<()> {
        let mut parameters = MetadataParameters::default();
        parameters.insert("sequence".into(), Parameter::String(sequence.to_string()));
        parameters.insert(
            "capture_unix_ns".into(),
            Parameter::String(capture_unix_ns.to_string()),
        );
        self.publish("snapshots", parameters, packet)
    }

    pub fn publish_dictionary(&mut self, packet: &monitor_wire::pb::Packet) -> Result<()> {
        self.publish("dictionary", MetadataParameters::default(), packet)
    }

    fn publish(
        &mut self,
        topic: &str,
        parameters: MetadataParameters,
        packet: &monitor_wire::pb::Packet,
    ) -> Result<()> {
        let len = packet.encoded_len();
        eyre::ensure!(
            len <= monitor_wire::MAX_FRAME_BYTES,
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

    /// Poll stop/error events without delaying the host's publication loop.
    pub fn stop_requested(&mut self) -> Result<bool> {
        loop {
            match self.events.try_recv() {
                Ok(Event::Stop(_)) | Err(TryRecvError::Closed) => return Ok(true),
                Ok(Event::Error(error)) => bail!("Dora event error: {error}"),
                Err(TryRecvError::Empty) => return Ok(false),
                _ => {}
            }
        }
    }

    pub fn finish(&mut self) -> Result<()> {
        // close_outputs() only sends a control-plane notification. It can overtake
        // the final direct Zenoh/SHM payload. DoraNode::drop tears down the data
        // transport BEFORE notifying the daemon that outputs are closed.
        drop(self.node.take());
        Ok(())
    }
}

fn encode_into(packet: &monitor_wire::pb::Packet, buffer: &mut [u8]) {
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
    #[test]
    fn direct_destination_preserves_wire_bytes_and_bounds() {
        let packet = monitor_wire::dictionary_packet(&monitor_wire::DictionaryFrame {
            version: monitor_wire::VERSION,
            session_id: "test".into(),
            revision: u64::MAX,
            overflow: 0,
            capture_unix_ns: u64::MAX,
            entries: vec![monitor_wire::DictionaryEntry {
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
            monitor_wire::decode_dictionary(&guarded[1..=len])
                .unwrap()
                .revision,
            u64::MAX
        );
    }
}
