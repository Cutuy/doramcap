//! Bounded, user-triggered workloads for the supported memory paths.
use crate::{Result, now_ns, packet, record};
use prost::Message as _;
use serde_json::{Value, json};
use storage_persistence::{
    mcap::{McapWriter, Message},
    observability::Observer,
    slabs::{SealedSlab, SlabPool},
};

const MIB: usize = 1024 * 1024;
#[derive(Default)]
pub struct Exercises {
    pool: Option<SlabPool>,
    readers: Vec<SealedSlab>,
    receipts: std::collections::VecDeque<Value>,
    next: u64,
}
impl Exercises {
    pub fn run(
        &mut self,
        action: &str,
        size_mib: usize,
        hold: bool,
        writer: &mut McapWriter,
        observer: Observer,
        sequence: &mut u32,
    ) {
        self.next += 1;
        let before = writer.snapshot().writer.registration_updates;
        let result = self.perform(action, size_mib, hold, writer, observer, sequence);
        let receipt = match result {
            Ok(mut row) => {
                row["status"] = json!("accepted");
                row
            }
            Err(error) => json!({"status":"rejected","error":error.to_string()}),
        };
        let mut row = receipt;
        row["id"] = json!(self.next.to_string());
        row["action"] = json!(action);
        row["registrationUpdates"] = json!(writer.snapshot().writer.registration_updates - before);
        if self.receipts.len() == 16 {
            self.receipts.pop_front();
        }
        self.receipts.push_back(row);
    }
    fn perform(
        &mut self,
        action: &str,
        size_mib: usize,
        hold: bool,
        writer: &mut McapWriter,
        observer: Observer,
        sequence: &mut u32,
    ) -> Result<Value> {
        if action == "release_readers" {
            self.readers.clear();
            return Ok(json!({"messages":0}));
        }
        writer.flush_written()?;
        if action == "retire_slabs" {
            let retired = if let Some(pool) = &self.pool {
                let before = writer.snapshot().writer.registered_regions;
                writer.retire_slabs(pool)?;
                before - writer.snapshot().writer.registered_regions
            } else {
                0
            };
            return Ok(json!({"messages":0,"retired":retired}));
        }
        let first = *sequence;
        let (messages, bytes, range) = if action == "managed_large" {
            let p = payload(*sequence, "managed:", size_mib * MIB);
            let bytes = record(writer, &p, 4, *sequence)?;
            *sequence = sequence.wrapping_add(1);
            (1, bytes, None)
        } else {
            if self.pool.is_none() {
                self.pool = Some(SlabPool::with_observer(4, MIB, 4096, observer)?);
            }
            let pool = self.pool.as_ref().unwrap();
            writer.prepare_slabs(pool)?;
            let mut builder = writer.prepare_records(pool.acquire()?);
            let mut bytes = 0;
            for i in 0..2 {
                let seq = sequence.wrapping_add(i);
                let p = payload(seq, "slab:", 256 * 1024);
                let len = p.encoded_len();
                let now = now_ns();
                p.encode(&mut builder.reserve(
                    Message {
                        channel: 5,
                        sequence: seq,
                        log_time: now,
                        publish_time: now,
                    },
                    len,
                )?)?;
                bytes += len;
            }
            let block = builder.seal().map_err(|r| r.error)?;
            let reader = hold.then(|| block.slab().clone());
            let range = writer.append_prepared_records(block).map_err(|r| r.error)?;
            if let Some(reader) = reader {
                self.readers.push(reader);
            }
            *sequence = sequence.wrapping_add(2);
            (2, bytes, Some(range))
        };
        let active = writer.active_writes().max_by_key(|w| w.len);
        Ok(
            json!({"messages":messages,"bytes":bytes,"first":first,"last":sequence.wrapping_sub(1),"write":active.as_ref().map(|w|w.ids.write.to_string()),"region":active.as_ref().map(|w|w.ids.region.to_string()),"slabUse":active.as_ref().map(|w|w.ids.slab_use.to_string()),"offset":range.map(|r|r.offset),"length":active.map(|w|w.len)}),
        )
    }
    pub fn release(&mut self) {
        self.readers.clear();
    }
    pub fn json(&self) -> Value {
        let pool=self.pool.as_ref().map(|p| { let s=p.snapshot();
            json!({"id":s.pool_id.to_string(),"available":s.available,"capacity":s.slabs.len(),"slabs":s.slabs.iter().map(|r|json!({"region":r.region_id.to_string(),"use":r.slab_use_id.to_string(),"phase":format!("{:?}",r.phase),"readers":r.readers,"ioLeases":r.io_leases,"retainedAfterIo":r.retained_after_io(),"ageUs":r.age_us})).collect::<Vec<_>>()}) });
        json!({"receipts":self.receipts,"heldReaders":self.readers.len(),"pool":pool,"maxWriteBytes":256*1024})
    }
}
fn payload(seq: u32, prefix: &str, size: usize) -> telemetry_protocol::pb::Packet {
    let mut p = packet(seq, false);
    if let Some(telemetry_protocol::pb::packet::Body::Snapshot(f)) = &mut p.body {
        f.source = prefix.to_owned() + &"x".repeat(size);
    }
    p
}
