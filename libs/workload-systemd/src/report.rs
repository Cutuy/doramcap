//! Native failure evidence, generated from the backend-owned protobuf schema.
include!(concat!(env!("OUT_DIR"), "/doramcap.systemd_report.v1.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;

    #[test]
    fn failure_wire_preserves_native_zero_and_service_text() {
        // Arrange
        let failure = UnitFailure {
            unit: "camera.service".into(),
            cause: "camera failed to start".into(),
            service: Some(ServiceStatus {
                result: "exit-code".into(),
                exec_main_code: 1,
                exec_main_status: 0,
                status_text: "camera disconnected".into(),
            }),
        };
        // Act
        let restored = UnitFailure::decode(failure.encode_to_vec().as_slice()).unwrap();
        // Assert
        assert_eq!(restored, failure);
        assert_eq!(restored.service.unwrap().exec_main_status, 0);
    }

    #[test]
    fn failure_wire_retains_cause_when_service_status_is_unavailable() {
        // Arrange
        let failure = UnitFailure {
            unit: "camera.service".into(),
            cause: "camera failed to start".into(),
            service: None,
        };
        // Act
        let restored = UnitFailure::decode(failure.encode_to_vec().as_slice()).unwrap();
        // Assert
        assert_eq!(restored, failure);
        assert!(restored.service.is_none());
    }
}
