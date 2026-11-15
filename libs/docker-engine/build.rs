fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::Config::new()
        .extern_path(
            ".doramcap.filesystem.v1",
            "::host_filesystem::filesystem_proto",
        )
        .field_attribute(
            ".doramcap.docker.v1.OwnedContainer.app_scope",
            "#[serde(default)]",
        )
        .type_attribute(
            ".doramcap.docker.v1",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .compile_protos(
            &["proto/docker.proto"],
            &["proto", "../host-filesystem/proto"],
        )?;
    println!("cargo:rerun-if-changed=proto/docker.proto");
    println!("cargo:rerun-if-changed=../host-filesystem/proto/filesystem.proto");
    Ok(())
}
