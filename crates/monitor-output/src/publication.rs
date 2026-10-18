use dora_node_api::{DoraNode, Event, EventStream, MetadataParameters, Parameter, TryRecvError};
use eyre::{bail, Result};

/// Owns Dora exclusively on the publication thread. Calls may allocate/block.
pub struct Publisher {
    node: DoraNode,
    events: EventStream,
}
impl Publisher {
    pub fn new_from_env() -> Result<Self> {
        let (node, events) = DoraNode::init_from_env_force()?;
        Ok(Self { node, events })
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
            .send_output_bytes("snapshots".into(), parameters, payload.len(), payload)?;
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
        self.node.close_outputs(vec!["snapshots".into()])?;
        Ok(())
    }
}
