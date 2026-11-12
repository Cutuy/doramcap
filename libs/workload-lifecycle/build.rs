fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::Config::new()
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .type_attribute(
            ".doramcap.launch.v1",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .field_attribute(
            ".doramcap.launch.v1.RunPlan.private_records",
            "#[serde(default)]",
        )
        .compile_protos(&["proto/launch.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto/launch.proto");
    Ok(())
}
