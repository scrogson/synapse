//! synapse-proto-gen: Generate auxiliary proto types from entity definitions
//!
//! This tool reads proto files with synapse.storage.entity annotations and generates:
//! - Primitive filter types (StringFilter, IntFilter, BoolFilter)
//! - OrderDirection enum
//! - PageInfo message
//! - Entity-specific Filter, OrderBy, Edge, Connection types
//! - Request/Response messages for CRUD operations

use clap::Parser;
use std::path::PathBuf;

mod generator;
mod parser;

#[derive(Parser, Debug)]
#[command(name = "synapse-proto-gen")]
#[command(about = "Generate auxiliary proto types from entity definitions")]
struct Args {
    /// Input proto file(s) containing entity definitions
    #[arg(required = true)]
    input: Vec<PathBuf>,

    /// Output proto file path
    #[arg(short, long, default_value = "generated.synapse.proto")]
    output: PathBuf,

    /// Proto include paths for imports
    #[arg(short = 'I', long = "proto-path")]
    proto_paths: Vec<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Parse input proto files
    let entities = parser::parse_proto_files(&args.input, &args.proto_paths)?;

    if entities.is_empty() {
        eprintln!("No entities found with synapse.storage.entity annotation");
        return Ok(());
    }

    // Generate output proto
    let output = generator::generate_proto(&entities)?;

    // Write to file
    std::fs::write(&args.output, output)?;

    eprintln!(
        "Generated {} with types for {} entities",
        args.output.display(),
        entities.len()
    );

    Ok(())
}
