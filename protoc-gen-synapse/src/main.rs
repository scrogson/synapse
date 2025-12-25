//! protoc-gen-synapse
//!
//! A protoc plugin for generating type-safe ORM code across multiple backends.
//!
//! Usage:
//!   protoc --synapse_out=backend=seaorm:./gen proto/*.proto
//!   protoc --synapse_out=backend=ecto:./gen proto/*.proto

#![deny(warnings)]
#![deny(missing_docs)]

use std::io::{self, Read, Write};

use prost::Message;
use prost_types::compiler::{CodeGeneratorRequest, CodeGeneratorResponse};

mod error;
mod grpc;
pub mod options;
mod storage;

pub use error::GeneratorError;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read the CodeGeneratorRequest from stdin
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input)?;

    // Parse the parameter to get the backend
    let param_str = {
        // Peek at the request to get parameter before consuming
        let request = CodeGeneratorRequest::decode(&input[..])?;
        request.parameter.clone().unwrap_or_default()
    };
    let backend_name = parse_backend_param(&param_str);

    // Generate code using the appropriate backend
    let response = match backend_name.as_str() {
        "seaorm" => {
            // Use the SeaORM backend's generate_from_bytes which handles extension caching
            storage::seaorm::generator::generate_from_bytes(&input)?
        }
        _ => {
            // Unknown backend
            CodeGeneratorResponse {
                error: Some(format!("Unknown backend: {}", backend_name)),
                supported_features: Some(1),
                ..Default::default()
            }
        }
    };

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
    // Default to seaorm
    "seaorm".to_string()
}
