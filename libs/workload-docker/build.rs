fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::Config::new()
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .compile_protos(&["proto/filesystem.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto/filesystem.proto");
    Ok(())
}
