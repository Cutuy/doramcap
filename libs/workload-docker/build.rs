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
        .compile_protos(
            &["proto/compose_plan.proto"],
            &[
                "proto",
                "../compose-apps/proto",
                "../workload-lifecycle/proto",
            ],
        )?;
    println!("cargo:rerun-if-changed=../workload-lifecycle/proto/launch.proto");
    println!("cargo:rerun-if-changed=../compose-apps/proto/compose.proto");
    println!("cargo:rerun-if-changed=proto/compose_plan.proto");
    Ok(())
}
