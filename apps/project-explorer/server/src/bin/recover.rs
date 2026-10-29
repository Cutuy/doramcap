//! Explicitly recover a gallery generation after a manager crash.
use project_explorer_server::{lease::ManagerLease, registry::Registry, workflow};
use std::{path::PathBuf, rc::Rc};
use workload_docker::compose::DockerAdapter;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 5 {
        return Err("usage: recover WORKSPACE DATA APP GENERATION DOCKER_SOCKET".into());
    }
    let workspace = std::fs::canonicalize(PathBuf::from(&args[0]))?;
    let data = std::fs::canonicalize(PathBuf::from(&args[1]))?;
    let app_id = &args[2];
    let token = &args[3];
    if token.len() != 32
        || !token
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("invalid generation".into());
    }
    let generation = u128::from_str_radix(token, 16)?;
    let _lease = ManagerLease::acquire(&data)?;
    let registry = Registry::load_all(&workspace)?;
    let app = registry.get(app_id).ok_or("unknown app")?;
    let adapter = Rc::new(DockerAdapter::connect(&args[4]).await?);
    let report = workflow::recover(adapter, app, generation, &data).await?;
    for error in &report.errors {
        eprintln!("{error}");
    }
    if !report.cleanup_verified {
        return Err("cleanup could not be verified; resources and evidence retained".into());
    }
    if !report.errors.is_empty() {
        return Err("cleanup verified; finalization or metadata errors retained".into());
    }
    println!("Generation {token}: cleanup verified; results retained");
    Ok(())
}
