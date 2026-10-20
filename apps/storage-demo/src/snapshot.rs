// Typed projection from the dashboard state; no JSON text serialization.
use serde_json::Value;
pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/doramcap.storage.v1.rs"));
}
fn unsigned(v: &Value) -> u64 {
    v.as_u64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        .unwrap_or_default()
}
pub fn snapshot(v: &Value) -> pb::Snapshot {
    pb::Snapshot {
        version: 1,
        status: v["status"].as_str().unwrap_or_default().to_owned(),
        session: v["session"].as_str().unwrap_or_default().to_owned(),
        error: v["error"].as_str().unwrap_or_default().to_owned(),
        backend: v["backend"].as_str().unwrap_or_default().to_owned(),
        direct: v["direct"].as_bool().unwrap_or_default(),
        elapsed_ms: unsigned(&v["elapsedMs"]),
        cpu: unsigned(&v["cpu"]),
        heartbeats: unsigned(&v["heartbeats"]),
        probes: unsigned(&v["probes"]),
        input_drops: unsigned(&v["inputDrops"]),
        last_cpu: (!v["lastCpu"].is_null()).then(|| last_cpu(&v["lastCpu"])),
        combining: (!v["combining"].is_null()).then(|| combining(&v["combining"])),
        pool: (!v["pool"].is_null()).then(|| pool(&v["pool"])),
        ring: (!v["ring"].is_null()).then(|| ring(&v["ring"])),
        exercises: (!v["exercises"].is_null()).then(|| exercises(&v["exercises"])),
        observation: (!v["observation"].is_null()).then(|| observation(&v["observation"])),
        active_writes: v["activeWrites"]
            .as_array()
            .into_iter()
            .flatten()
            .map(active_write)
            .collect(),
        timing: (!v["timing"].is_null()).then(|| timing(&v["timing"])),
        latency: (!v["latency"].is_null()).then(|| stats(&v["latency"])),
        lost_timing_events: unsigned(&v["lostTimingEvents"]),
        burst_remaining: unsigned(&v["burstRemaining"]),
        receipts: v["receipts"]
            .as_array()
            .into_iter()
            .flatten()
            .map(receipt)
            .collect(),
        messages: v["messages"]
            .as_array()
            .into_iter()
            .flatten()
            .map(input)
            .collect(),
        events: v["events"]
            .as_array()
            .into_iter()
            .flatten()
            .map(event)
            .collect(),
        file: (!v["file"].is_null()).then(|| file(&v["file"])),
        last_sealed: (!v["lastSealed"].is_null()).then(|| file(&v["lastSealed"])),
    }
}
pub fn last_cpu(v: &Value) -> pb::LastCpu {
    pb::LastCpu {
        sequence: unsigned(&v["sequence"]),
        rows: unsigned(&v["rows"]),
        dropped: unsigned(&v["dropped"]),
        source: v["source"].as_str().unwrap_or_default().to_owned(),
    }
}
pub fn combining(v: &Value) -> pb::Combining {
    pb::Combining {
        used: unsigned(&v["used"]),
        messages: unsigned(&v["messages"]),
        capacity: unsigned(&v["capacity"]),
        age_ms: (!v["ageMs"].is_null()).then(|| v["ageMs"].as_f64().unwrap_or_default()),
        deadline_ms: unsigned(&v["deadlineMs"]),
    }
}
pub fn pool(v: &Value) -> pb::Pool {
    pb::Pool {
        free: unsigned(&v["free"]),
        total: unsigned(&v["total"]),
        allocated_bytes: unsigned(&v["allocatedBytes"]),
        free_bytes: unsigned(&v["freeBytes"]),
        in_flight_bytes: unsigned(&v["inFlightBytes"]),
        registered_bytes: unsigned(&v["registeredBytes"]),
    }
}
pub fn ring(v: &Value) -> pb::Ring {
    pb::Ring {
        writer: unsigned(&v["writer"]),
        file: unsigned(&v["file"]),
        pending_capacity: unsigned(&v["pendingCapacity"]),
        outstanding: unsigned(&v["outstanding"]),
        submitted: unsigned(&v["submitted"]),
        completions: unsigned(&v["completions"]),
        accepted: unsigned(&v["accepted"]),
        completed: unsigned(&v["completed"]),
        synced: unsigned(&v["synced"]),
        awaiting_submission: unsigned(&v["awaitingSubmission"]),
        awaiting_cqe: unsigned(&v["awaitingCqe"]),
        pending_bytes: unsigned(&v["pendingBytes"]),
        oldest_us: (!v["oldestUs"].is_null()).then(|| unsigned(&v["oldestUs"])),
        registered_regions: unsigned(&v["registeredRegions"]),
        registration_updates: unsigned(&v["registrationUpdates"]),
        accepted_writes: unsigned(&v["acceptedWrites"]),
        written: unsigned(&v["written"]),
        failed: unsigned(&v["failed"]),
        cancelled: unsigned(&v["cancelled"]),
        rejected_capacity: unsigned(&v["rejectedCapacity"]),
        rejected_invalid: unsigned(&v["rejectedInvalid"]),
        rejected_closed: unsigned(&v["rejectedClosed"]),
        rejected_other: unsigned(&v["rejectedOther"]),
        retries: unsigned(&v["retries"]),
        registration_hits: unsigned(&v["registrationHits"]),
        registration_misses: unsigned(&v["registrationMisses"]),
        registration_failures: unsigned(&v["registrationFailures"]),
        sync_failures: unsigned(&v["syncFailures"]),
        submit_failures: unsigned(&v["submitFailures"]),
    }
}
pub fn observation(v: &Value) -> pb::Observation {
    pb::Observation {
        mode: v["mode"].as_str().unwrap_or_default().to_owned(),
        cursor: unsigned(&v["cursor"]),
        missed: unsigned(&v["missed"]),
        contention_drops: unsigned(&v["contentionDrops"]),
        overwritten: unsigned(&v["overwritten"]),
        timeline_evicted: unsigned(&v["timelineEvicted"]),
        capacity: unsigned(&v["capacity"]),
    }
}
pub fn active_write(v: &Value) -> pb::ActiveWrite {
    pb::ActiveWrite {
        write: unsigned(&v["write"]),
        operation: unsigned(&v["operation"]),
        region: unsigned(&v["region"]),
        slab_use: unsigned(&v["slabUse"]),
        state: v["state"].as_str().unwrap_or_default().to_owned(),
        offset: unsigned(&v["offset"]),
        len: unsigned(&v["len"]),
        completed: unsigned(&v["completed"]),
        age_us: (!v["ageUs"].is_null()).then(|| unsigned(&v["ageUs"])),
        attempts: unsigned(&v["attempts"]),
        retries: unsigned(&v["retries"]),
    }
}
pub fn timing(v: &Value) -> pb::Timing {
    pb::Timing {
        sample: (!v["sample"].is_null()).then(|| stats(&v["sample"])),
        arrival: (!v["arrival"].is_null()).then(|| stats(&v["arrival"])),
        gaps: unsigned(&v["gaps"]),
        interval_ms: (!v["intervalMs"].is_null())
            .then(|| v["intervalMs"].as_f64().unwrap_or_default()),
    }
}
pub fn stats(v: &Value) -> pb::Stats {
    pb::Stats {
        count: unsigned(&v["count"]),
        last_ms: (!v["lastMs"].is_null()).then(|| v["lastMs"].as_f64().unwrap_or_default()),
        p99_ms: (!v["p99Ms"].is_null()).then(|| v["p99Ms"].as_f64().unwrap_or_default()),
        max_ms: (!v["maxMs"].is_null()).then(|| v["maxMs"].as_f64().unwrap_or_default()),
    }
}
pub fn receipt(v: &Value) -> pb::Receipt {
    pb::Receipt {
        kind: v["kind"].as_str().unwrap_or_default().to_owned(),
        count: unsigned(&v["count"]),
        bytes: unsigned(&v["bytes"]),
        first: (!v["first"].is_null()).then(|| unsigned(&v["first"])),
        last: (!v["last"].is_null()).then(|| unsigned(&v["last"])),
        encode_ms: v["encodeMs"].as_f64().unwrap_or_default(),
    }
}
pub fn input(v: &Value) -> pb::Input {
    pb::Input {
        sequence: unsigned(&v["sequence"]),
        kind: v["kind"].as_str().unwrap_or_default().to_owned(),
        bytes: unsigned(&v["bytes"]),
        time_ms: unsigned(&v["timeMs"]),
    }
}
pub fn event(v: &Value) -> pb::Event {
    pb::Event {
        id: unsigned(&v["id"]),
        kind: v["kind"].as_str().unwrap_or_default().to_owned(),
        us: unsigned(&v["us"]),
        writer: unsigned(&v["writer"]),
        file: unsigned(&v["file"]),
        write: unsigned(&v["write"]),
        operation: unsigned(&v["operation"]),
        region: unsigned(&v["region"]),
        slab_use: unsigned(&v["slabUse"]),
        pool: unsigned(&v["pool"]),
        offset: unsigned(&v["offset"]),
        len: unsigned(&v["len"]),
        result: (!v["result"].is_null()).then(|| v["result"].as_i64().unwrap_or_default()),
        latency_us: (!v["latencyUs"].is_null()).then(|| unsigned(&v["latencyUs"])),
    }
}
pub fn file(v: &Value) -> pb::File {
    pb::File {
        index: unsigned(&v["index"]),
        name: v["name"].as_str().unwrap_or_default().to_owned(),
        sealed: v["sealed"].as_bool().unwrap_or_default(),
        size: unsigned(&v["size"]),
        parsed_through: unsigned(&v["parsedThrough"]),
        records: v["records"]
            .as_array()
            .into_iter()
            .flatten()
            .map(record)
            .collect(),
        tail_offset: unsigned(&v["tailOffset"]),
        tail: v["tail"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|b| unsigned(b) as u8)
            .collect(),
    }
}
pub fn record(v: &Value) -> pb::Record {
    pb::Record {
        offset: unsigned(&v["offset"]),
        len: unsigned(&v["len"]),
        kind: v["kind"].as_str().unwrap_or_default().to_owned(),
        channel: (!v["channel"].is_null()).then(|| unsigned(&v["channel"]) as u32),
        sequence: (!v["sequence"].is_null()).then(|| unsigned(&v["sequence"]) as u32),
        payload: (!v["payload"].is_null()).then(|| unsigned(&v["payload"])),
    }
}
pub fn exercises(v: &Value) -> pb::Exercises {
    pb::Exercises {
        receipts: v["receipts"]
            .as_array()
            .into_iter()
            .flatten()
            .map(path_receipt)
            .collect(),
        held_readers: unsigned(&v["heldReaders"]),
        pool: (!v["pool"].is_null()).then(|| slab_pool(&v["pool"])),
        max_write_bytes: unsigned(&v["maxWriteBytes"]),
    }
}
pub fn slab_pool(v: &Value) -> pb::SlabPool {
    pb::SlabPool {
        id: unsigned(&v["id"]),
        available: unsigned(&v["available"]),
        capacity: unsigned(&v["capacity"]),
        slabs: v["slabs"]
            .as_array()
            .into_iter()
            .flatten()
            .map(slab)
            .collect(),
    }
}
pub fn slab(v: &Value) -> pb::Slab {
    pb::Slab {
        region: unsigned(&v["region"]),
        r#use: unsigned(&v["use"]),
        phase: v["phase"].as_str().unwrap_or_default().to_owned(),
        readers: unsigned(&v["readers"]),
        io_leases: unsigned(&v["ioLeases"]),
        retained_after_io: v["retainedAfterIo"].as_bool().unwrap_or_default(),
        age_us: (!v["ageUs"].is_null()).then(|| unsigned(&v["ageUs"])),
    }
}
pub fn path_receipt(v: &Value) -> pb::PathReceipt {
    pb::PathReceipt {
        status: v["status"].as_str().unwrap_or_default().to_owned(),
        error: v["error"].as_str().unwrap_or_default().to_owned(),
        id: unsigned(&v["id"]),
        action: v["action"].as_str().unwrap_or_default().to_owned(),
        registration_updates: unsigned(&v["registrationUpdates"]),
        messages: unsigned(&v["messages"]),
        retired: unsigned(&v["retired"]),
        bytes: unsigned(&v["bytes"]),
        first: unsigned(&v["first"]) as u32,
        last: unsigned(&v["last"]) as u32,
        write: (!v["write"].is_null()).then(|| unsigned(&v["write"])),
        region: (!v["region"].is_null()).then(|| unsigned(&v["region"])),
        slab_use: (!v["slabUse"].is_null()).then(|| unsigned(&v["slabUse"])),
        offset: (!v["offset"].is_null()).then(|| unsigned(&v["offset"])),
        length: (!v["length"].is_null()).then(|| unsigned(&v["length"])),
    }
}

use axum::{
    body::Bytes,
    extract::{
        State,
        ws::{Message, WebSocketUpgrade},
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use prost::Message as _;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::watch;

/// Encode once per observed update; slow clients skip intermediate snapshots.
pub fn relay(mut source: watch::Receiver<Value>, stop: Arc<AtomicBool>) -> watch::Receiver<Bytes> {
    let initial = snapshot(&source.borrow_and_update()).encode_to_vec().into();
    let (tx, rx) = watch::channel(initial);
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_millis(100));
        loop {
            tokio::select! {
                changed = source.changed() => {
                    if changed.is_err() { break; }
                    let bytes = snapshot(&source.borrow_and_update()).encode_to_vec();
                    tx.send_replace(bytes.into());
                }
                _ = tick.tick() => if stop.load(Ordering::Relaxed) { break; },
            }
        }
    });
    rx
}
pub async fn stream(ws: WebSocketUpgrade, State(state): State<crate::Web>) -> Response {
    ws.max_message_size(1024).max_frame_size(1024).on_upgrade(move |socket| async move {
        let (mut outgoing, mut incoming) = socket.split();
        let mut rx = state.binary;
        let mut tick = tokio::time::interval(Duration::from_millis(100));
        loop {
            if state.stop.load(Ordering::Relaxed) { return; }
            let bytes = rx.borrow_and_update().clone();
            if !matches!(tokio::time::timeout(Duration::from_secs(2), outgoing.send(Message::Binary(bytes))).await, Ok(Ok(()))) { return; }
            loop {
                tokio::select! {
                    changed = rx.changed() => { if changed.is_err() { return; } break; }
                    input = incoming.next() => match input { None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return, _=>{} },
                    _ = tick.tick() => if state.stop.load(Ordering::Relaxed) { return; },
                }
            }
        }
    })
}
