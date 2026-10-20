fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    prost_build::Config::new()
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .type_attribute("GenerationRef", "#[derive(PartialOrd, Ord)]")
        .type_attribute("Step", "#[derive(PartialOrd, Ord)]")
        .file_descriptor_set_path(out.join("provenance.bin"))
        .compile_protos(&["proto/provenance.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto/provenance.proto");
    Ok(())
}
