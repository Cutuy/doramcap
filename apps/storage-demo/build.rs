fn main() {
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    prost_build::Config::new()
        .protoc_executable(protoc_bin_vendored::protoc_bin_path().unwrap())
        .file_descriptor_set_path(out.join("snapshot.bin"))
        .compile_protos(&["proto/snapshot.proto"], &["proto"])
        .unwrap();
    println!("cargo:rerun-if-changed=proto/snapshot.proto");
}
