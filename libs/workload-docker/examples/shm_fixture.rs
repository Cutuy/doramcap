//! Native filesystem payload fixture: `shm_fixture write|read|absent|remove NAME`.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let path = std::path::Path::new("/dev/shm").join(&args[2]);
    let payload = b"producer-exited-data";
    match args[1].as_str() {
        "write" => {
            use std::io::Write;
            std::fs::File::create_new(path)?.write_all(payload)?;
        }
        "read" => assert_eq!(std::fs::read(path)?, payload),
        "absent" => assert!(!path.exists()),
        "remove" => std::fs::remove_file(path)?,
        _ => return Err("expected write, read, absent, or remove".into()),
    }
    Ok(())
}
