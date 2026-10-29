fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::Config::new()
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .type_attribute(
            ".doramcap.recovery.v1.RecoveryProject",
            "#[derive(serde::Serialize, serde::Deserialize)]\n#[serde(deny_unknown_fields)]",
        )
        .type_attribute(".doramcap.system_apps.v1", "#[derive(serde::Serialize, serde::Deserialize)]\n#[serde(default, deny_unknown_fields)]")
        .compile_protos(
            &["proto/filesystem.proto", "proto/recovery.proto", "proto/system_apps.proto"],
            &["proto"],
        )?;
    println!("cargo:rerun-if-changed=proto/filesystem.proto");
    println!("cargo:rerun-if-changed=proto/recovery.proto");
    println!("cargo:rerun-if-changed=proto/system_apps.proto");
    Ok(())
}
