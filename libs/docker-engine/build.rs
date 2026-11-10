fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::Config::new()
        .type_attribute(
            ".doramcap.docker.v1",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .compile_protos(
            &["proto/docker.proto", "proto/filesystem.proto"],
            &["proto"],
        )?;
    println!("cargo:rerun-if-changed=proto/docker.proto");
    println!("cargo:rerun-if-changed=proto/filesystem.proto");
    Ok(())
}
