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
        .type_attribute(
            ".doramcap.compose_plan.v1",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
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
                "proto/compose.proto",
                "proto/cgroup.proto",
                "../workload-lifecycle/proto/launch.proto",
            ],
            &["proto", "../workload-lifecycle/proto"],
        )?;
    println!("cargo:rerun-if-changed=../workload-lifecycle/proto/launch.proto");
    println!("cargo:rerun-if-changed=proto/compose.proto");
    println!("cargo:rerun-if-changed=proto/cgroup.proto");
    Ok(())
}
