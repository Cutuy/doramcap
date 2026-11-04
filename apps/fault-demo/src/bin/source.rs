//! Demo-only producer: high-level operations report outcomes; the host drains IPC.
use dora_node_api::{Event, futures::StreamExt};
use eyre::Result;
use fault_reporter::{Definition, Mode, Text};
use std::time::Duration;
#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let name = args.next().unwrap_or_else(|| "a".into());
    let mut health = fault_dora::Health::new()?;
    let reporter = health.reporter();
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
        health.tick_to(&mut node, "reports")?;
    }
    health.finish_to(&mut node, "reports")?;
    Ok(())
}
