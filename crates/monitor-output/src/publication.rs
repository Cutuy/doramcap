use dora_node_api::{DoraNode, Event, EventStream, MetadataParameters, Parameter, TryRecvError};
use eyre::{bail, eyre, Result};

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

    pub fn publish_bytes(
        &mut self,
        sequence: u64,
        capture_unix_ns: u64,
        payload: &[u8],
    ) -> Result<()> {
        let mut parameters = MetadataParameters::default();
        parameters.insert("sequence".into(), Parameter::String(sequence.to_string()));
        parameters.insert(
            "capture_unix_ns".into(),
            Parameter::String(capture_unix_ns.to_string()),
        );
        self.node
            .as_mut()
            .ok_or_else(|| eyre!("publisher already finished"))?
            .send_output_bytes("snapshots".into(), parameters, payload.len(), payload)?;
        Ok(())
    }

    pub fn publish_dictionary(&mut self, payload: &[u8]) -> Result<()> {
        self.node
            .as_mut()
            .ok_or_else(|| eyre!("publisher already finished"))?
            .send_output_bytes(
                "dictionary".into(),
                MetadataParameters::default(),
                payload.len(),
                payload,
            )?;
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
