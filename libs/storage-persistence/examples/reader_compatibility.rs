//! Generate real-writer fixtures and verify Rust protobuf readback for the TypeScript suite.
use prost::Message as _;
use std::{error::Error, path::Path};
use storage_persistence::{
    mapped::MmapRegion,
    mcap::{ALIGNMENT, Channel, McapConfig, McapWriter, Message, Schema},
    observability::Observer,
    slabs::SlabPool,
};

#[path = "../tests/support/mod.rs"]
mod support;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const MESSAGE_HEADER_BYTES: usize = 31;
const BUFFER_BYTES: usize = ALIGNMENT * 2;
const SLAB_BYTES: usize = ALIGNMENT * 16;
const BASE_TIME: u64 = 1_800_000_000_000_000_001;
const LARGE_COUNTER: u64 = 9_007_199_254_740_993;
const SNAPSHOT_SCHEMA_VERSION: u32 = 1;
const ROTATIONS: usize = 2;
const PATHS: [&str; 5] = [
    "combined",
    "reserved",
    "slab",
    "external-records",
    "external-payload",
];

fn cpu_payload(size: usize) -> Result<Vec<u8>> {
    let mut frame: telemetry_protocol::Frame = telemetry_protocol::Frame {
        version: telemetry_protocol::VERSION,
        snapshot: telemetry_protocol::pb::Snapshot {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            sequence: LARGE_COUNTER,
            capture_unix_ns: BASE_TIME,
            records: vec![telemetry_protocol::pb::CpuRecord {
                cgroup_id: LARGE_COUNTER,
                cpu: 3,
                runtime_ns: LARGE_COUNTER,
                ..Default::default()
            }],
            ..Default::default()
        }
        .into(),
        source: "x".repeat(size),
        session_id: "reader-roundtrip".into(),
        task_collection_enabled: false,
        task_capacity: 0,
        dictionary_revision: LARGE_COUNTER,
        dictionary_full: true,
        dictionary_overflow: 0,
        dictionary: vec![telemetry_protocol::DictionaryEntry {
            id: LARGE_COUNTER,
            parent_id: 1,
            path: Some("/roundtrip/測試".into()),
        }],
    };
    let mut bytes = Vec::new();
    loop {
        telemetry_protocol::encode(&frame, &mut bytes)?;
        if bytes.len() == size {
            return Ok(bytes);
        }
        let excess = bytes
            .len()
            .checked_sub(size)
            .ok_or("payload sizing undershot")?;
        frame.source.truncate(
            frame
                .source
                .len()
                .checked_sub(excess)
                .ok_or("payload target too small")?,
        );
    }
}

fn fault_payload() -> Result<Vec<u8>> {
    // Use the production checked serializer, including its semantic validation.
    let packet = fault_protocol::pb::AggregationPublication {
        version: fault_protocol::VERSION,
        published_unix_ns: Some(BASE_TIME),
        emitted_at_ns: LARGE_COUNTER,
        payload: Some(
            fault_protocol::pb::aggregation_publication::Payload::Snapshot(
                fault_protocol::pb::AggregationSnapshot {
                    epoch: 1_u128.to_be_bytes().to_vec(),
                    revision: LARGE_COUNTER,
                    stats: Some(fault_protocol::pb::Stats {
                        dropped_updates: LARGE_COUNTER,
                        rejected_inputs: 2,
                        too_old: 3,
                    }),
                    sources: vec![],
                },
            ),
        ),
    };
    let frame = fault_protocol::decode_aggregation(&packet.encode_to_vec(), Default::default())?;
    Ok(fault_protocol::encode_aggregation(
        &frame,
        Default::default(),
    )?)
}

fn verify(path: &Path, expected: &[Vec<u8>], cpu: bool) -> Result<()> {
    // Act
    let bytes = std::fs::read(path)?;
    let messages = mcap::MessageStream::new(&bytes)?.collect::<std::result::Result<Vec<_>, _>>()?;
    // Assert
    assert_eq!(messages.len(), expected.len());
    for (sequence, (message, payload)) in messages.iter().zip(expected).enumerate() {
        assert_eq!(message.sequence, sequence as u32);
        assert_eq!(message.log_time, BASE_TIME + sequence as u64);
        assert_eq!(message.publish_time, BASE_TIME + sequence as u64 + 1);
        assert_eq!(
            message.channel.topic,
            if sequence % 2 == 0 {
                "primary"
            } else {
                "secondary"
            }
        );
        assert_eq!(message.channel.message_encoding, "protobuf");
        assert_eq!(message.data.as_ref(), payload);
        let schema = message.channel.schema.as_ref().ok_or("missing schema")?;
        assert_eq!(schema.encoding, "protobuf");
        if cpu {
            assert_eq!(schema.name, telemetry_protocol::MCAP_SCHEMA_NAME);
            assert_eq!(
                schema.data.as_ref(),
                telemetry_protocol::FILE_DESCRIPTOR_SET
            );
            telemetry_protocol::decode(&message.data)?;
            assert_eq!(
                telemetry_protocol::decode_packet(&message.data)?,
                telemetry_protocol::decode_packet(payload)?
            );
        } else {
            assert_eq!(schema.name, fault_protocol::AGGREGATION_SCHEMA_NAME);
            assert_eq!(schema.data.as_ref(), fault_protocol::FILE_DESCRIPTOR_SET);
            fault_protocol::decode_aggregation(&message.data, Default::default())?;
            assert_eq!(
                fault_protocol::pb::AggregationPublication::decode(message.data.as_ref())?,
                fault_protocol::pb::AggregationPublication::decode(payload.as_slice())?
            );
        }
    }
    Ok(())
}

fn capture(directory: &Path, path: &str, cpu: bool) -> Result<()> {
    // Arrange
    let kind = if cpu { "cpu" } else { "fault" };
    let lengths = match path {
        "reserved" => vec![BUFFER_BYTES, BUFFER_BYTES + ALIGNMENT],
        "external-payload" => vec![ALIGNMENT, BUFFER_BYTES],
        _ => vec![
            256,
            256,
            256,
            ALIGNMENT - MESSAGE_HEADER_BYTES - 1,
            ALIGNMENT - MESSAGE_HEADER_BYTES,
            ALIGNMENT - MESSAGE_HEADER_BYTES + 1,
        ],
    };
    let payloads = lengths
        .into_iter()
        .map(|len| {
            if cpu {
                cpu_payload(len)
            } else {
                fault_payload()
            }
        })
        .collect::<Result<Vec<_>>>()?;
    for (sequence, payload) in payloads.iter().enumerate() {
        std::fs::write(
            directory.join(format!("{kind}-{path}-{sequence}.bin")),
            payload,
        )?;
    }
    let file = |rotation| directory.join(format!("{kind}-{path}-{rotation}.mcap"));
    let mut writer = McapWriter::create(
        file(0),
        Schema {
            name: if cpu {
                telemetry_protocol::MCAP_SCHEMA_NAME
            } else {
                fault_protocol::AGGREGATION_SCHEMA_NAME
            },
            encoding: "protobuf",
            data: if cpu {
                telemetry_protocol::FILE_DESCRIPTOR_SET
            } else {
                fault_protocol::FILE_DESCRIPTOR_SET
            },
        },
        &[
            Channel {
                id: 1,
                topic: "primary",
                encoding: "protobuf",
            },
            Channel {
                id: 2,
                topic: "secondary",
                encoding: "protobuf",
            },
        ],
        McapConfig {
            buffers: 2,
            buffer_size: BUFFER_BYTES,
            ..Default::default()
        },
    )?;
    let slabs = SlabPool::new(1, SLAB_BYTES, ALIGNMENT)?;
    for rotation in 0..ROTATIONS {
        if rotation > 0 {
            writer.rotate(file(rotation))?;
        }
        let mut prepared = writer.prepare_records(slabs.acquire()?);
        for (sequence, payload) in payloads.iter().enumerate() {
            let message = Message {
                channel: if sequence % 2 == 0 { 1 } else { 2 },
                sequence: sequence as u32,
                log_time: BASE_TIME + sequence as u64,
                publish_time: BASE_TIME + sequence as u64 + 1,
            };
            // Act
            match path {
                "combined" => writer.write(message, payload)?,
                "reserved" => {
                    let mut reservation = writer.reserve(message, payload.len())?;
                    reservation.payload_mut().copy_from_slice(payload);
                    reservation.commit()?;
                    writer.flush_written()?;
                }
                "slab" => prepared
                    .reserve(message, payload.len())?
                    .copy_from_slice(payload),
                "external-records" | "external-payload" => {
                    let region = MmapRegion::new(
                        memmap2::MmapMut::map_anon(SLAB_BYTES)?,
                        Observer::default(),
                    );
                    let mut range = region.acquire(0..SLAB_BYTES)?;
                    let len = if path == "external-records" {
                        support::stamp(range.bytes_mut(), message, payload)
                    } else {
                        range.bytes_mut()[..payload.len()].copy_from_slice(payload);
                        payload.len()
                    };
                    let lease = range.seal(len).map_err(|e| e.error)?;
                    if path == "external-records" {
                        writer.append_external_records(lease).map_err(|e| e.error)?;
                    } else {
                        writer
                            .append_external_payload(message, lease)
                            .map_err(|e| e.error)?;
                    }
                    writer.flush_written()?;
                }
                _ => unreachable!(),
            }
        }
        if path == "slab" {
            writer
                .append_prepared_records(prepared.seal().map_err(|e| e.error)?)
                .map_err(|e| e.error)?;
        }
        writer.flush_written()?;
    }
    writer.finish()?;
    for rotation in 0..ROTATIONS {
        verify(&file(rotation), &payloads, cpu)?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let directory = std::env::args_os()
        .nth(1)
        .ok_or("usage: reader_compatibility OUTPUT_DIRECTORY")?;
    let directory = Path::new(&directory);
    std::fs::create_dir(directory)?;
    for path in PATHS {
        capture(directory, path, true)?;
        // External payload DMA requires an aligned payload length; CPU fixtures cover it.
        if path != "external-payload" {
            capture(directory, path, false)?;
        }
    }
    println!(
        "Rust reader and protobuf round trips passed: {}",
        directory.display()
    );
    Ok(())
}
