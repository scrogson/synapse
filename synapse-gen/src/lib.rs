pub mod ir;
mod parser;
mod options;

pub use parser::ParsedSchema;

mod generator;
pub use generator::{CodeGenerator, GeneratedFile, GeneratorContext, GeneratorError};

mod builder;
pub use builder::SynapseGenerator;
