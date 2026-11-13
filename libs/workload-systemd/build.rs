fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    // The descriptor set serves the example and tests that read protobuf text. launch.proto
    // joins it so RunPlan and AppManifest text decode; its Rust types stay in workload-lifecycle.
    prost_build::Config::new()
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .file_descriptor_set_path(out.join("systemd_plan.bin"))
        .extern_path(".doramcap.launch.v1", "::workload_lifecycle::launch")
        .extern_path(
            ".doramcap.filesystem.v1",
            "::host_filesystem::filesystem_proto",
        )
        .type_attribute(
            ".doramcap.systemd_plan.v1",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .field_attribute(
            ".doramcap.systemd_plan.v1.PreparedRun.dataflow_sha256",
            "#[serde(default)]",
        )
        .compile_protos(
            &[
                "proto/systemd_plan.proto",
                "../workload-lifecycle/proto/launch.proto",
            ],
            &[
                "proto",
                "../workload-lifecycle/proto",
                "../host-filesystem/proto",
            ],
        )?;
    println!("cargo:rerun-if-changed=proto/systemd_plan.proto");
    println!("cargo:rerun-if-changed=../workload-lifecycle/proto/launch.proto");
    println!("cargo:rerun-if-changed=../host-filesystem/proto/filesystem.proto");
    Ok(())
}
