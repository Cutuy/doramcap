fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::Config::new()
        .type_attribute(
            ".doramcap.filesystem.v1",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .compile_protos(&["proto/filesystem.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto/filesystem.proto");
    Ok(())
}
