pub mod ir;
mod parser;

mod generator;
pub use generator::{CodeGenerator, GeneratedFile, GeneratorContext, GeneratorError};

mod builder;
pub use builder::SynapseGenerator;
