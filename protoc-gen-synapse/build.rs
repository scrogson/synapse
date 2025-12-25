//! Build script for protoc-gen-synapse
//!
//! This compiles synapse proto options to generate Rust types for parsing
//! protobuf extensions. It also generates a file descriptor set for use
//! with prost-reflect to parse extension fields.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    println!("cargo:rerun-if-changed=../proto/synapse/storage/options.proto");
    println!("cargo:rerun-if-changed=../proto/synapse/validate/options.proto");
    println!("cargo:rerun-if-changed=../proto/synapse/grpc/options.proto");
    println!("cargo:rerun-if-changed=../proto/synapse/graphql/options.proto");

    // Compile options protos to Rust types
    prost_build::Config::new()
        .out_dir(&out_dir)
        .compile_protos(
            &[
                "../proto/synapse/storage/options.proto",
                "../proto/synapse/validate/options.proto",
                "../proto/synapse/grpc/options.proto",
                "../proto/synapse/graphql/options.proto",
            ],
            &["../proto/"],
        )?;

    // Generate a FileDescriptorSet for prost-reflect
    // Must include synapse options AND google.protobuf.compiler types
    let fds_path = out_dir.join("file_descriptor_set.bin");

    // Find the protobuf include path
    let protobuf_include = find_protobuf_include();

    let status = Command::new("protoc")
        .args([
            "--descriptor_set_out",
            fds_path.to_str().unwrap(),
            "--include_imports",
            "--include_source_info",
            "-I../proto",
            &format!("-I{}", protobuf_include),
            // Include synapse options
            "synapse/storage/options.proto",
            "synapse/validate/options.proto",
            "synapse/grpc/options.proto",
            "synapse/graphql/options.proto",
            // Include compiler types for CodeGeneratorRequest parsing
            "google/protobuf/compiler/plugin.proto",
        ])
        .status()?;

    if !status.success() {
        return Err("protoc failed to generate file descriptor set".into());
    }

    Ok(())
}

/// Find the protobuf include directory containing well-known types
fn find_protobuf_include() -> String {
    // Try brew (macOS)
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

    // Common Linux paths
    for path in [
        "/usr/include",
        "/usr/local/include",
        "/opt/homebrew/include",
    ] {
        let test_file = format!("{}/google/protobuf/descriptor.proto", path);
        if std::path::Path::new(&test_file).exists() {
            return path.to_string();
        }
    }

    // Fallback
    "/usr/include".to_string()
}
