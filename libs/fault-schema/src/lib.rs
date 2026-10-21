//! Generated fault wire contracts. No dependency on runtime cores.
/// Protobuf FileDescriptorSet including all three schema files, suitable for MCAP.
pub const FILE_DESCRIPTOR_SET: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/fault_descriptor.bin"));
#[allow(clippy::large_enum_variant)]
pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/doramcap.fault.v1.rs"));
}
