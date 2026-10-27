use workflow_protocol::{Event, FILE_DESCRIPTOR_SET, decode, encode, encoded_len, event};

#[test]
fn descriptor_is_identical_to_the_pre_extraction_contract() {
    // Arrange
    let legacy = include_bytes!("fixtures/legacy-descriptor.bin");
    // Act / Assert
    assert_eq!(FILE_DESCRIPTOR_SET, legacy);
}

#[test]
fn event_encoding_preserves_the_v1_wire_layout() {
    // Arrange
    let legacy = b"\x08\x01\x10\x02\x1a\x01s\x20\x03\x2a\x01X\x30\x01\x3a\x01e\x40\x04\x4a\x02\x05\x06\x50\x02\x58\x07\x60\x08\x6a\x02{}\x72\x00";
    let event = Event {
        version: 1,
        sequence: 2,
        session: "s".into(),
        invocation: 3,
        step: "X".into(),
        step_version: 1,
        executor: "e".into(),
        parent: Some(4),
        dependencies: vec![5, 6],
        state: event::State::Succeeded.into(),
        elapsed_ns: 7,
        duration_ns: 8,
        payload_json: Some("{}".into()),
        error: Some(String::new()),
    };
    // Act
    let bytes = encode(&event);
    let decoded = decode(legacy).unwrap();
    // Assert
    assert_eq!(bytes, legacy);
    assert_eq!(encoded_len(&event), legacy.len());
    assert_eq!(decoded, event);
}
