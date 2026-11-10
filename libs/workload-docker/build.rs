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
        "ComposeProject.manifest",
        "StartInput.services",
        "StopInput.services",
        "StopInput.allow_volume_deletion",
    ] {
        config.field_attribute(format!(".doramcap.compose.v1.{field}"), "#[serde(default)]");
    }
    config
        .btree_map([".doramcap.compose.v1"])
        .type_attribute(
            ".doramcap.compose.v1",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute(
            ".doramcap.compose.v1.ComposeProject",
            "#[serde(deny_unknown_fields)]",
        )
        .extern_path(".doramcap.launch.v1", "::workload_lifecycle::launch")
        .file_descriptor_set_path(out.join("workflow_config.bin"))
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .compile_protos(
            &[
                "proto/filesystem.proto",
                "proto/compose.proto",
                "../workload-lifecycle/proto/launch.proto",
            ],
            &["proto", "../workload-lifecycle/proto"],
        )?;
    // The Compose plan reuses the generated Compose input types.
    prost_build::Config::new()
        .type_attribute(
            ".doramcap.compose_plan.v1",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .extern_path(".doramcap.compose.v1", "crate::compose")
        .compile_protos(&["proto/compose_plan.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=../workload-lifecycle/proto/launch.proto");
    println!("cargo:rerun-if-changed=proto/compose.proto");
    println!("cargo:rerun-if-changed=proto/filesystem.proto");
    println!("cargo:rerun-if-changed=proto/compose_plan.proto");
    Ok(())
}
