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
        .field_attribute(
            ".doramcap.launch.v1.RunPlan.write_policy",
            "#[serde(default)]",
        )
        .extern_path(
            ".doramcap.filesystem.v1",
            "::host_filesystem::filesystem_proto",
        )
        .type_attribute(
            ".doramcap.journal.v1",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .compile_protos(
            &["proto/launch.proto", "proto/journal.proto"],
            &["proto", "../host-filesystem/proto"],
        )?;
    println!("cargo:rerun-if-changed=../host-filesystem/proto/filesystem.proto");
    println!("cargo:rerun-if-changed=proto/launch.proto");
    println!("cargo:rerun-if-changed=proto/journal.proto");
    Ok(())
}
