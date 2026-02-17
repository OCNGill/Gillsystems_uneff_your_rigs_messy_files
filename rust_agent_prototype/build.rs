fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use vendored protoc so users do not need to install protobuf-compiler.
    let protoc = protoc_bin_vendored::protoc_bin_path()
        .expect("protoc-bin-vendored: could not locate bundled protoc binary");
    std::env::set_var("PROTOC", protoc);

    // Generate gRPC stubs into OUT_DIR (standard cargo convention).
    // Access from Rust via: tonic::include_proto!("gillsystems_uneff");
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(
            &["proto/agent_service.proto"],
            &["proto"],
        )?;

    println!("cargo:rerun-if-changed=proto/agent_service.proto");
    println!("cargo:rerun-if-changed=proto");
    Ok(())
}
