fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    prost_build::Config::new()
        .file_descriptor_set_path(out.join("workflow_config.bin"))
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .type_attribute(
            ".doramcap.recovery.v1.RecoveryProject",
            "#[derive(serde::Serialize, serde::Deserialize)]\n#[serde(deny_unknown_fields)]",
        )
        .type_attribute(".doramcap.system_apps.v1", "#[derive(serde::Serialize, serde::Deserialize)]\n#[serde(default, deny_unknown_fields)]")
        .type_attribute(".doramcap.dora.v1", "#[derive(serde::Serialize, serde::Deserialize)]\n#[serde(default, deny_unknown_fields)]")
        .compile_protos(
            &["proto/filesystem.proto", "proto/recovery.proto", "proto/dora.proto", "proto/system_apps.proto"],
            &["proto"],
        )?;
    println!("cargo:rerun-if-changed=proto/filesystem.proto");
    println!("cargo:rerun-if-changed=proto/recovery.proto");
    println!("cargo:rerun-if-changed=proto/dora.proto");
    println!("cargo:rerun-if-changed=proto/system_apps.proto");
    Ok(())
}
