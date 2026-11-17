use orchestrator::Step;
use workload_holoscan::{DiscoverApplications, DiscoverRequest};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: catalog /absolute/holoscan /absolute/project".into());
    }
    let catalog = DiscoverApplications::default()
        .run(DiscoverRequest {
            cli: args[1].clone(),
            project_root: args[2].clone(),
        })
        .await?;
    println!("{}", serde_json::to_string(&catalog)?);
    Ok(())
}
