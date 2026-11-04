//! Direct BridgeConfig protobuf TextFormat loading.
use crate::bridge::pb::BridgeConfig;
use eyre::{Result, ensure};
use prost_reflect::{DescriptorPool, DynamicMessage};

/// Parses complete launch settings; rejects invalid ports and empty payload limits.
pub fn parse(text: &str) -> Result<BridgeConfig> {
    let descriptor =
        DescriptorPool::decode(include_bytes!(concat!(env!("OUT_DIR"), "/bridge.bin")).as_slice())?
            .get_message_by_name("doramcap.console.live.v1.BridgeConfig")
            .expect("compiled configuration");
    let config: BridgeConfig =
        DynamicMessage::parse_text_format(descriptor, text)?.transcode_to()?;
    ensure!(
        (1..=65535).contains(&config.port),
        "port must be in 1..65535"
    );
    ensure!(
        config.max_payload_bytes > 0,
        "max_payload_bytes must be positive"
    );
    Ok(config)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn standard_textformat_requires_explicit_valid_settings() {
        // Arrange / Act
        let config = parse("port: 43190 max_payload_bytes: 1048576").unwrap();
        // Assert
        assert_eq!(config.port, 43190);
        assert_eq!(config.max_payload_bytes, 1048576);
        for text in [
            "",
            "port: 65536 max_payload_bytes: 1",
            "port: 43190",
            "include: 'base.pbtxt'",
        ] {
            assert!(parse(text).is_err());
        }
    }
}
