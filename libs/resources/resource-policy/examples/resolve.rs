//! cargo run -p resource-policy --example resolve -- PATH APP_ID...
use resource_policy::{Goal, config, targets};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        return Err("usage: resolve PATH APP_ID...".into());
    }
    let deployment = config::load(std::path::Path::new(&args[0]))?;
    for id in &args[1..] {
        let policy = deployment
            .app
            .iter()
            .find(|app| &app.id == id)
            .and_then(|app| app.policy.as_ref())
            .ok_or("unknown app ID")?;
        println!(
            "# {id}\n[Service]\n{}",
            targets::systemd_service_directives(&Goal::from_proto(policy)?)
        );
    }
    Ok(())
}
