use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    println!("cargo:rerun-if-changed=../proto/synapse/storage/options.proto");
    println!("cargo:rerun-if-changed=../proto/synapse/validate/options.proto");
    println!("cargo:rerun-if-changed=../proto/synapse/grpc/options.proto");
    println!("cargo:rerun-if-changed=../proto/synapse/graphql/options.proto");
    println!("cargo:rerun-if-changed=../proto/synapse/graphql/resolver.proto");
    println!("cargo:rerun-if-changed=../proto/synapse/graphql/context.proto");

    prost_build::Config::new()
        .out_dir(&out_dir)
        .compile_protos(
            &[
                "../proto/synapse/storage/options.proto",
                "../proto/synapse/validate/options.proto",
                "../proto/synapse/grpc/options.proto",
                "../proto/synapse/graphql/options.proto",
                "../proto/synapse/graphql/resolver.proto",
                "../proto/synapse/graphql/context.proto",
            ],
            &["../proto/"],
        )?;

    let fds_path = out_dir.join("file_descriptor_set.bin");
    let protobuf_include = find_protobuf_include();

    let status = Command::new("protoc")
        .args([
            "--descriptor_set_out",
            fds_path.to_str().unwrap(),
            "--include_imports",
            "--include_source_info",
            "-I../proto",
            &format!("-I{}", protobuf_include),
            "synapse/storage/options.proto",
            "synapse/validate/options.proto",
            "synapse/grpc/options.proto",
            "synapse/graphql/options.proto",
            "synapse/graphql/resolver.proto",
            "synapse/graphql/context.proto",
            "google/protobuf/compiler/plugin.proto",
        ])
        .status()?;

    if !status.success() {
        return Err("protoc failed to generate file descriptor set".into());
    }

    Ok(())
}

fn find_protobuf_include() -> String {
    if let Some(path) = Command::new("brew")
        .args(["--prefix", "protobuf"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| format!("{}/include", s.trim()))
    {
        if std::path::Path::new(&path).exists() {
            return path;
        }
    }

    for path in ["/usr/include", "/usr/local/include", "/opt/homebrew/include"] {
        let test_file = format!("{}/google/protobuf/descriptor.proto", path);
        if std::path::Path::new(&test_file).exists() {
            return path.to_string();
        }
    }

    "/usr/include".to_string()
}
