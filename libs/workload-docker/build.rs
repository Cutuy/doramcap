fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    let mut config = prost_build::Config::new();
    for field in [
        "ComposeProject.env_files",
        "ComposeProject.environment",
        "ComposeProject.profiles",
        "ComposeProject.services",
        "ComposeProject.completed_services",
        "ComposeProject.remove_volumes",
        "StartInput.services",
        "StopInput.services",
        "StopInput.preserve_volumes",
    ] {
        config.field_attribute(format!(".doramcap.compose.v1.{field}"), "#[serde(default)]");
    }
    config
        .btree_map([".doramcap.compose.v1"])
        .type_attribute(".doramcap.compose.v1", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute(".doramcap.compose.v1.ComposeProject", "#[serde(deny_unknown_fields)]")
        .file_descriptor_set_path(out.join("workflow_config.bin"))
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .type_attribute(
            ".doramcap.recovery.v1.RecoveryProject",
            "#[derive(serde::Serialize, serde::Deserialize)]\n#[serde(deny_unknown_fields)]",
        )
        .type_attribute(".doramcap.system_apps.v1", "#[derive(serde::Serialize, serde::Deserialize)]\n#[serde(default, deny_unknown_fields)]")
        .type_attribute(".doramcap.dora.v1", "#[derive(serde::Serialize, serde::Deserialize)]\n#[serde(default, deny_unknown_fields)]")
        .compile_protos(
            &["proto/filesystem.proto", "proto/recovery.proto", "proto/dora.proto", "proto/system_apps.proto", "proto/compose.proto"],
            &["proto"],
        )?;
    println!("cargo:rerun-if-changed=proto/compose.proto");
    println!("cargo:rerun-if-changed=proto/filesystem.proto");
    println!("cargo:rerun-if-changed=proto/recovery.proto");
    println!("cargo:rerun-if-changed=proto/dora.proto");
    println!("cargo:rerun-if-changed=proto/system_apps.proto");
    Ok(())
}
