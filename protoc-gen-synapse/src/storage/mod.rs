//! Storage layer generation

pub mod defaults;
pub mod seaorm;
mod traits;

pub use defaults::StorageDefaultsGenerator;
pub use traits::StorageTraitGenerator;
