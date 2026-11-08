fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    // launch.proto joins the descriptor set so its RunPlan text decodes; its Rust types
    // stay in workload-lifecycle.
    prost_build::Config::new()
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .file_descriptor_set_path(out.join("systemd_plan.bin"))
        .extern_path(".doramcap.launch.v1", "::workload_lifecycle::launch")
        .type_attribute(
            ".doramcap.systemd_plan.v1",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .compile_protos(
            &[
                "proto/systemd_plan.proto",
                "../workload-lifecycle/proto/launch.proto",
            ],
            &["proto", "../workload-lifecycle/proto"],
        )?;
    println!("cargo:rerun-if-changed=proto/systemd_plan.proto");
    println!("cargo:rerun-if-changed=../workload-lifecycle/proto/launch.proto");
    Ok(())
}
