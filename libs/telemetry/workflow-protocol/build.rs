fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    prost_build::Config::new()
        .type_attribute(
            ".doramcap.workflow.v1.StepDescriptor",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .file_descriptor_set_path(out.join("workflow_descriptor.bin"))
        .compile_protos(&["proto/workflow.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto/workflow.proto");
    Ok(())
}
