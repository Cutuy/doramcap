//! Versioned workflow events with transport-supplied framing.
//!
//! The host bounds frame size before decoding and validates [`Event::version`]
//! against [`VERSION`]. Codecs perform no I/O and retain no state. Observer sequence
//! numbers, session identities and elapsed-time epochs belong to the publisher.
include!(concat!(env!("OUT_DIR"), "/doramcap.workflow.v1.rs"));
pub const VERSION: u32 = 1;
pub const FILE_DESCRIPTOR_SET: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/workflow_descriptor.bin"));
/// Encode one event. Transports supply framing.
pub fn encode(event: &Event) -> Vec<u8> {
    prost::Message::encode_to_vec(event)
}
/// Decode one event. The consumer enforces frame limits before calling.
pub fn decode(bytes: &[u8]) -> Result<Event, prost::DecodeError> {
    <Event as prost::Message>::decode(bytes)
}
/// Encoded size, for admission before allocation.
pub fn encoded_len(event: &Event) -> usize {
    prost::Message::encoded_len(event)
}
