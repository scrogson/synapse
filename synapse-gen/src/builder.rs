// Builder for composing and running Synapse code generators.

use crate::generator::{CodeGenerator, GeneratedFile, GeneratorContext, GeneratorError};

/// Builder for composing multiple code generators and running them together.
#[derive(Default)]
pub struct SynapseGenerator {
    generators: Vec<Box<dyn CodeGenerator>>,
}

impl SynapseGenerator {
    /// Create a new empty `SynapseGenerator`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a code generator to the pipeline.
    pub fn add_generator(mut self, generator: impl CodeGenerator + 'static) -> Self {
        self.generators.push(Box::new(generator));
        self
    }

    /// Run all registered generators and collect results.
    pub fn generate(&self, ctx: &GeneratorContext) -> Result<Vec<GeneratedFile>, GeneratorError> {
        let mut files = Vec::new();
        for generator in &self.generators {
            files.extend(generator.generate(ctx)?);
        }
        Ok(files)
    }
}
