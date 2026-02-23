pub mod ir;
mod parser;
mod options;

mod generator;
pub use generator::{CodeGenerator, GeneratedFile, GeneratorContext, GeneratorError};

mod builder;
pub use builder::SynapseGenerator;
