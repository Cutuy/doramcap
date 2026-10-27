fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::Config::new()
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .extern_path(".doramcap.telemetry.v1", "::telemetry_protocol::pb")
        .compile_protos(
            &["proto/delivery.proto"],
            &["proto", "../../../libs/telemetry/protocol/proto"],
        )?;
    println!("cargo:rerun-if-changed=proto/delivery.proto");
    println!("cargo:rerun-if-changed=../../../libs/telemetry/protocol/proto/telemetry.proto");
    Ok(())
}
