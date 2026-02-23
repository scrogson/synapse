use crate::generator::CodeGenerator;

/// Builder for composing multiple code generators into a protoc plugin.
pub struct SynapseGenerator {
    generators: Vec<Box<dyn CodeGenerator>>,
}

impl SynapseGenerator {
    pub fn new() -> Self {
        Self { generators: vec![] }
    }

    /// Register a generator.
    pub fn add<G: CodeGenerator + 'static>(mut self, generator: G) -> Self {
        self.generators.push(Box::new(generator));
        self
    }
}

impl Default for SynapseGenerator {
    fn default() -> Self {
        Self::new()
    }
}
