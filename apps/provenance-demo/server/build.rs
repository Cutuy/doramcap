fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::Config::new()
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .extern_path(".doramcap.provenance.v1", "::step_provenance::pb")
        .extern_path(".doramcap.activity.v1", "::activity_tracing::wire::proto")
        .compile_protos(
            &["proto/demo.proto"],
            &[
                "proto",
                "../../../libs/step-provenance/proto",
                "../../../libs/activity-tracing/proto",
            ],
        )?;
    println!("cargo:rerun-if-changed=proto/demo.proto");
    println!("cargo:rerun-if-changed=../../../libs/step-provenance/proto/provenance.proto");
    println!("cargo:rerun-if-changed=../../../libs/activity-tracing/proto/activity.proto");
    Ok(())
}
