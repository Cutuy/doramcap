//! A simulated lifecycle with explicit injection and cleanup.
use orchestrator::{LocalExecutor, Result, Step};
use std::rc::Rc;

struct Start {
    project: Rc<String>,
}
impl Step for Start {
    type Input = String;
    type Output = String;
    async fn run(&mut self, label: String) -> Result<String> {
        Ok(format!("{label}-{}", self.project))
    }
}
struct Observe;
impl Step for Observe {
    type Input = String;
    type Output = ();
    async fn run(&mut self, resource: String) -> Result<()> {
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        println!("observed {resource}");
        Ok(())
    }
}
struct Stop;
impl Step for Stop {
    type Input = String;
    type Output = ();
    async fn run(&mut self, resource: String) -> Result<()> {
        println!("stopped {resource}");
        Ok(())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let project = Rc::new("robot".to_owned());
    let mut executor = LocalExecutor::default();
    executor.register::<Start>(move || Start {
        project: project.clone(),
    });
    executor.register::<Observe>(|| Observe);
    executor.register::<Stop>(|| Stop);
    let resource = executor.run::<Start>("attempt-1".to_owned()).await?;
    let outcome = executor.run::<Observe>(resource.clone()).await;
    // Cleanup is awaited even when Observe fails.
    let cleanup = executor.run::<Stop>(resource).await;
    if let Err(error) = &outcome {
        eprintln!("experiment failed: {error}");
    }
    cleanup?;
    outcome?;
    Ok(())
}
