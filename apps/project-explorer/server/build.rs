fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    prost_build::Config::new()
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .file_descriptor_set_path(out.join("registration.bin"))
        .extern_path(".doramcap.dora.v1", "::workload_docker::workflows::dora")
        // Serde supports private artifact indexes.
        .type_attribute(".doramcap.hosting.v1.ArtifactCollection", "#[derive(serde::Serialize, serde::Deserialize)]\n#[serde(deny_unknown_fields)]")
        .type_attribute(".doramcap.hosting.v1.ResultFormat", "#[derive(serde::Serialize, serde::Deserialize)]\n#[serde(rename_all = \"snake_case\")]")
        .field_attribute(".doramcap.hosting.v1.ArtifactCollection.format", "#[serde(with = \"super::result_format_serde\")]")
        .compile_protos(
            &["proto/registration.proto"],
            &["proto", "../../../libs/workload-docker/proto"],
        )?;
    println!("cargo:rerun-if-changed=proto/registration.proto");
    println!("cargo:rerun-if-changed=../../../libs/workload-docker/proto/dora.proto");
    Ok(())
}
