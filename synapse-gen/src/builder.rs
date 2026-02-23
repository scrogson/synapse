use std::collections::HashMap;
use std::io::{self, Read, Write};

use prost::Message;
use prost_types::compiler::CodeGeneratorResponse;

use crate::generator::{CodeGenerator, GeneratedFile, GeneratorContext, GeneratorError};
use crate::parser::ParsedSchema;

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

    /// Full protoc plugin lifecycle: read stdin, generate, write stdout.
    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let mut input = Vec::new();
        io::stdin().read_to_end(&mut input)?;

        let response = self.generate(&input)?;

        let mut output = Vec::new();
        response.encode(&mut output)?;
        io::stdout().write_all(&output)?;
        Ok(())
    }

    /// Generate from raw bytes (testable without stdin/stdout).
    pub fn generate(&self, bytes: &[u8]) -> Result<CodeGeneratorResponse, GeneratorError> {
        let parsed = ParsedSchema::parse(bytes)?;
        let schema = parsed.schema();

        let mut all_files: Vec<(String, GeneratedFile)> = Vec::new();

        for package in &schema.packages {
            let ctx = GeneratorContext {
                schema: &schema,
                package,
            };

            for generator in &self.generators {
                let gen_name = generator.name().to_string();

                for entity in &package.entities {
                    for file in generator.generate_entity(&ctx, entity)? {
                        all_files.push((gen_name.clone(), file));
                    }
                }

                for service in &package.services {
                    for file in generator.generate_service(&ctx, service)? {
                        all_files.push((gen_name.clone(), file));
                    }
                }

                for enum_ in &package.enums {
                    for file in generator.generate_enum(&ctx, enum_)? {
                        all_files.push((gen_name.clone(), file));
                    }
                }

                for message in &package.messages {
                    for file in generator.generate_message(&ctx, message)? {
                        all_files.push((gen_name.clone(), file));
                    }
                }

                for file in generator.finalize_package(&ctx)? {
                    all_files.push((gen_name.clone(), file));
                }
            }
        }

        for generator in &self.generators {
            let gen_name = generator.name().to_string();
            for file in generator.finalize(&schema)? {
                all_files.push((gen_name.clone(), file));
            }
        }

        // Detect file path collisions
        let mut seen: HashMap<String, String> = HashMap::new();
        for (gen_name, file) in &all_files {
            if let Some(existing_gen) = seen.get(&file.path) {
                return Err(GeneratorError::FileCollision {
                    path: file.path.clone(),
                    first: existing_gen.clone(),
                    second: gen_name.clone(),
                });
            }
            seen.insert(file.path.clone(), gen_name.clone());
        }

        // Build CodeGeneratorResponse
        let mut response = CodeGeneratorResponse::default();
        for (_, file) in all_files {
            response.file.push(
                prost_types::compiler::code_generator_response::File {
                    name: Some(file.path),
                    content: Some(file.content),
                    ..Default::default()
                },
            );
        }

        Ok(response)
    }
}

impl Default for SynapseGenerator {
    fn default() -> Self {
        Self::new()
    }
}
