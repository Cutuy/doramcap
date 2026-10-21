fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    prost_build::Config::new()
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .file_descriptor_set_path(out.join("config.bin"))
        .compile_protos(&["proto/watchdog.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto/watchdog.proto");
    Ok(())
}
