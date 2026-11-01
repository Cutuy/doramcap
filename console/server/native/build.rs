fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::Config::new()
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .file_descriptor_set_path(
            std::path::PathBuf::from(std::env::var("OUT_DIR")?).join("console.bin"),
        )
        .extern_path(".doramcap.compose.v1", "::workload_docker::compose")
        .extern_path(".doramcap.dora.v1", "::workload_docker::workflows::dora")
        .extern_path(".doramcap.workflow.v1", "::workflow_protocol")
        .compile_protos(
            &[
                "../../proto/console-config.proto",
                "../../proto/execution.proto",
            ],
            &[
                "../../proto",
                "../../../libs/workload-docker/proto",
                "../../../libs/telemetry/workflow-protocol/proto",
                "../../../libs/telemetry/protocol/proto",
            ],
        )?;
    println!("cargo:rerun-if-changed=../../proto/console-config.proto");
    println!("cargo:rerun-if-changed=../../proto/execution.proto");
    println!("cargo:rerun-if-changed=../../../libs/workload-docker/proto/compose.proto");
    println!(
        "cargo:rerun-if-changed=../../../libs/telemetry/workflow-protocol/proto/workflow.proto"
    );
    println!("cargo:rerun-if-changed=../../../libs/telemetry/protocol/proto/telemetry.proto");
    Ok(())
}
