use std::path::Path;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args()
        .nth(1)
        .ok_or("usage: project_inputs /absolute/project")?;
    let inputs = workload_holoscan::project::read_project_inputs(Path::new(&root)).await?;
    println!("{}", serde_json::to_string(&inputs)?);
    Ok(())
}
