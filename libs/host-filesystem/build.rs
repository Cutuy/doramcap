fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::Config::new()
        .field_attribute(
            ".doramcap.filesystem.v1.FileChange.app_scope",
            "#[serde(default)]",
        )
        .field_attribute(
            ".doramcap.filesystem.v1.FileChange.source_kind",
            "#[serde(default)]",
        )
        .field_attribute(
            ".doramcap.filesystem.v1.DirectoryMount.app_scope",
            "#[serde(default)]",
        )
        .type_attribute(
            ".doramcap.filesystem.v1",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .enum_attribute(
            ".doramcap.filesystem.v1.StorageCategory",
            "#[serde(rename_all = \"snake_case\")]",
        )
        .enum_attribute(
            ".doramcap.filesystem.v1.ChangeKind",
            "#[serde(rename_all = \"snake_case\")]",
        )
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .compile_protos(&["proto/filesystem.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto/filesystem.proto");
    Ok(())
}
