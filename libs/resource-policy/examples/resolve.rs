//! cargo run -p resource-policy --example resolve -- PATH APP_ID...
use resource_policy::{Goal, config, targets};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        return Err("usage: resolve PATH APP_ID...".into());
    }
    let registry = args[1..].iter().map(String::as_str).collect::<Vec<_>>();
    let resolved = config::resolve(std::path::Path::new(&args[0]), &registry)?;
    for id in registry {
        println!(
            "# {id}\n[Service]\n{}",
            targets::systemd_service_directives(&Goal::from_proto(&resolved.config(id)?)?)
        );
    }
    Ok(())
}
