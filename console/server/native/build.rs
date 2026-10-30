fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::Config::new()
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .file_descriptor_set_path(
            std::path::PathBuf::from(std::env::var("OUT_DIR")?).join("console.bin"),
        )
        .extern_path(
            ".doramcap.system_apps.v1",
            "::workload_docker::workflows::system_apps",
        )
        .extern_path(".doramcap.workflow.v1", "::workflow_protocol")
        .compile_protos(
            &["../../proto/console.proto"],
            &[
                "../../proto",
                "../../../libs/workload-docker/proto",
                "../../../libs/telemetry/workflow-protocol/proto",
            ],
        )?;
    println!("cargo:rerun-if-changed=../../proto/console.proto");
    println!("cargo:rerun-if-changed=../../../libs/workload-docker/proto/system_apps.proto");
    println!(
        "cargo:rerun-if-changed=../../../libs/telemetry/workflow-protocol/proto/workflow.proto"
    );
    Ok(())
}
