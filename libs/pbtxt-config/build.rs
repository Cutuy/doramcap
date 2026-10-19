fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    prost_build::Config::new()
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .file_descriptor_set_path(out.join("config.bin"))
        .compile_protos(
            &["proto/config.proto"],
            &[
                std::path::PathBuf::from("proto"),
                protoc_bin_vendored::include_path()?,
            ],
        )?;
    println!("cargo:rerun-if-changed=proto/config.proto");
    Ok(())
}
