fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::Config::new()
        .type_attribute(
            ".doramcap.compose_plan.v1",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .file_descriptor_set_path(
            std::path::PathBuf::from(std::env::var("OUT_DIR")?).join("workflow_config.bin"),
        )
        .extern_path(".doramcap.compose.v1", "::compose_apps::compose")
        .extern_path(".doramcap.launch.v1", "::workload_lifecycle::launch")
        .extern_path(
            ".doramcap.filesystem.v1",
            "::host_filesystem::filesystem_proto",
        )
        .compile_protos(
            &[
                "proto/compose_plan.proto",
                "../workload-lifecycle/proto/launch.proto",
            ],
            &[
                "proto",
                "../compose-apps/proto",
                "../workload-lifecycle/proto",
                "../host-filesystem/proto",
            ],
        )?;
    println!("cargo:rerun-if-changed=../workload-lifecycle/proto/launch.proto");
    println!("cargo:rerun-if-changed=../host-filesystem/proto/filesystem.proto");
    println!("cargo:rerun-if-changed=../compose-apps/proto/compose.proto");
    println!("cargo:rerun-if-changed=proto/compose_plan.proto");
    Ok(())
}
