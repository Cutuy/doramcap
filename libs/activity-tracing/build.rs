fn main() {
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    prost_build::Config::new()
        .protoc_executable(protoc_bin_vendored::protoc_bin_path().unwrap())
        .file_descriptor_set_path(out.join("activity.bin"))
        .compile_protos(&["proto/activity.proto"], &["proto"])
        .unwrap();
    println!("cargo:rerun-if-changed=proto/activity.proto");
}
