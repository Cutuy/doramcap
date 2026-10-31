//! Demo-only producer: high-level operations report outcomes; the host drains IPC.
use dora_node_api::{DoraNode, Event, futures::StreamExt};
use eyre::Result;
use fault_protocol::{ReporterFrame, ReporterPayload};
use fault_reporter::{Definition, Mode, Reporter, Text};
use std::time::{Duration, Instant};
#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let name = args.next().unwrap_or_else(|| "a".into());
    let (reporter, mut publisher) =
        Reporter::new(fault_watchdog::incarnation()?, Default::default())?;
    let mut definition = Definition::new("demo", &name, "operation")?;
    definition.policy.deadline = Some(Duration::from_millis(500));
    definition.policy.success_interval = Some(Duration::from_millis(100));
    let check = reporter.register(definition)?;
    let mut definition = Definition::new("demo", &name, "event")?;
    definition.policy.mode = Mode::EventOnly;
    let event = reporter.register(definition)?;
    let (mut node, mut events) = dora_app::environment::connect()?;
    if args.next().is_some() {
        eyre::bail!("unexpected argument");
    }
    dora_app::environment::ready()?;
    let mut mode = "healthy".to_string();
    let mut ticks = 0u64;
    let mut snapshot = Instant::now();
    let mut timer = tokio::time::interval(Duration::from_millis(20));
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    // Keep the subscription alive while processing batches between selects.
    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            input=events.next()=>match input{
                Some(Event::Input{data,..})=>{if let Ok(bytes)=<&[u8]>::try_from(&data)&& let Ok(command)=serde_json::from_slice::<serde_json::Value>(bytes)&& (command["source"]==name || command["source"]=="all")&& let Some(value)=command["mode"].as_str(){match value{"event"=>{event.failure(Text::new("demo event")?)?;},"healthy"|"fail"|"flap"|"pause"|"flood"=>mode=value.into(),_=>{}}}},
                Some(Event::InputClosed{..})|Some(Event::Stop(dora_node_api::StopCause::AllInputsClosed))=>{},
                Some(Event::Stop(_))|None=>break,_=>{},
            },
            _=timer.tick()=>{ticks+=1;match mode.as_str(){"pause"=>{},"fail"=>{check.failure(Text::new("simulated I/O error")?)?;},"flap" if ticks%30<15=>{check.failure(Text::new("intermittent operation")?)?;},"flood"=>{for _ in 0..2000{check.failure(Text::new("burst")?)?;}},_=>{check.success()?;}}},
            _=&mut shutdown=>break,
        }
        if mode == "pause" {
            continue;
        }
        // Registration/reconciliation precedes batches, and repeats after restart.
        if let Some(s) = publisher.take_reconciliation() {
            send(&mut node, ReporterPayload::Snapshot(s))?;
            snapshot = Instant::now();
        }
        publisher.tick();
        while publisher.has_records() {
            let batch = publisher.batch(512)?;
            if batch.observations.is_empty() {
                break;
            }
            send(&mut node, ReporterPayload::Batch(batch))?;
        }
        if snapshot.elapsed() >= Duration::from_millis(500) {
            send(&mut node, ReporterPayload::Snapshot(publisher.snapshot()))?;
            snapshot = Instant::now();
        }
    }
    reporter.close();
    send(
        &mut node,
        ReporterPayload::Snapshot(publisher.final_snapshot()?),
    )?;
    Ok(())
}
fn send(node: &mut DoraNode, payload: ReporterPayload) -> Result<()> {
    let bytes = fault_protocol::encode_reporter(
        &ReporterFrame {
            published_unix_ns: fault_watchdog::unix_ns(),
            payload,
        },
        Default::default(),
    )?;
    node.send_output_raw("reports".into(), Default::default(), bytes.len(), |b| {
        b.copy_from_slice(&bytes)
    })?;
    Ok(())
}
