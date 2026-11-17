fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::Config::new()
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .type_attribute(
            ".doramcap.holoscan.v1",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .field_attribute(
            ".doramcap.holoscan.v1.HoloscanPlan.system_projects",
            "#[serde(default)]",
        )
        .extern_path(".doramcap.launch.v1", "::workload_lifecycle::launch")
        .extern_path(".doramcap.compose_plan.v1", "::compose_apps::cgroup")
        .extern_path(".doramcap.compose.v1", "::compose_apps::compose")
        .extern_path(
            ".doramcap.filesystem.v1",
            "::host_filesystem::filesystem_proto",
        )
        .compile_protos(
            &["proto/holoscan.proto"],
            &[
                "proto",
                "../workload-lifecycle/proto",
                "../host-filesystem/proto",
                "../compose-apps/proto",
            ],
        )?;
    println!("cargo:rerun-if-changed=proto/holoscan.proto");
    println!("cargo:rerun-if-changed=../workload-lifecycle/proto/launch.proto");
    println!("cargo:rerun-if-changed=../host-filesystem/proto/filesystem.proto");
    println!("cargo:rerun-if-changed=../compose-apps/proto/cgroup.proto");
    println!("cargo:rerun-if-changed=../compose-apps/proto/compose.proto");
    Ok(())
}
