fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    prost_build::Config::new()
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .file_descriptor_set_path(out.join("telemetry_descriptor.bin"))
        .compile_protos(&["proto/telemetry.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto/telemetry.proto");
    Ok(())
}
