fn main() {
    println!("cargo:rerun-if-changed=src/bpf/tasks.bpf.c");
    #[cfg(feature = "bpf")]
    {
        let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        let mut builder = libbpf_cargo::SkeletonBuilder::new();
        builder
            .source("src/bpf/tasks.bpf.c")
            .obj(output.join("tasks.bpf.o"));
        if let Some(clang) = std::env::var_os("CLANG") {
            builder.clang(clang);
        }
        builder
            .build()
            .expect("task BPF compilation requires clang BPF and libelf/zlib development headers");
    }
}
