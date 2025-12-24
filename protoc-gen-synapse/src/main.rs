//! protoc-gen-synapse
//!
//! A protoc plugin for generating type-safe ORM code across multiple backends.
//!
//! Usage:
//!   protoc --synapse_out=backend=seaorm:./gen proto/*.proto
//!   protoc --synapse_out=backend=ecto:./gen proto/*.proto

use std::io::{self, Read, Write};

use prost::Message;
use prost_types::compiler::{CodeGeneratorRequest, CodeGeneratorResponse};

mod backends;
mod ir;
mod options;

use backends::Backend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read the CodeGeneratorRequest from stdin
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input)?;

    let request = CodeGeneratorRequest::decode(&input[..])?;

    // Parse the parameter to get the backend
    let backend_name = parse_backend_param(request.parameter.as_deref().unwrap_or(""));
    let backend = backends::get_backend(&backend_name)?;

    // Generate code
    let response = generate(&request, backend.as_ref())?;

    // Write the response to stdout
    let mut output = Vec::new();
    response.encode(&mut output)?;
    io::stdout().write_all(&output)?;

    Ok(())
}

/// Parse the backend parameter from the protoc plugin parameter string
fn parse_backend_param(param: &str) -> String {
    for part in param.split(',') {
        if let Some(value) = part.strip_prefix("backend=") {
            return value.to_string();
        }
    }
    // Default to seaorm for backwards compatibility
    "seaorm".to_string()
}

/// Generate code using the specified backend
fn generate(
    request: &CodeGeneratorRequest,
    backend: &dyn Backend,
) -> Result<CodeGeneratorResponse, Box<dyn std::error::Error>> {
    let mut response = CodeGeneratorResponse::default();

    // TODO: Parse proto files into IR
    // TODO: Generate code using backend

    // For now, just set the supported features
    response.supported_features = Some(1); // FEATURE_PROTO3_OPTIONAL

    eprintln!(
        "protoc-gen-synapse: Using {} backend (not yet implemented)",
        backend.name()
    );

    Ok(response)
}
