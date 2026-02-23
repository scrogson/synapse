use std::sync::{Arc, Mutex};

use prost::Message;
use prost_types::compiler::CodeGeneratorRequest;
use prost_types::{DescriptorProto, FieldDescriptorProto, FileDescriptorProto};

use synapse_gen::ir::{Entity, Message as IrMessage};
use synapse_gen::{
    CodeGenerator, GeneratedFile, GeneratorContext, GeneratorError, ParsedSchema, SynapseGenerator,
};

// ---------------------------------------------------------------------------
// Helper: build a CodeGeneratorRequest with a single message
// ---------------------------------------------------------------------------

fn make_request_with_message(pkg: &str, file_name: &str, msg_name: &str) -> Vec<u8> {
    let msg = DescriptorProto {
        name: Some(msg_name.to_string()),
        field: vec![
            FieldDescriptorProto {
                name: Some("id".to_string()),
                number: Some(1),
                r#type: Some(3), // TYPE_INT64
                ..Default::default()
            },
            FieldDescriptorProto {
                name: Some("name".to_string()),
                number: Some(2),
                r#type: Some(9), // TYPE_STRING
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let file = FileDescriptorProto {
        name: Some(file_name.to_string()),
        package: Some(pkg.to_string()),
        message_type: vec![msg],
        ..Default::default()
    };

    let request = CodeGeneratorRequest {
        file_to_generate: vec![file_name.to_string()],
        proto_file: vec![file],
        ..Default::default()
    };

    let mut bytes = Vec::new();
    request.encode(&mut bytes).unwrap();
    bytes
}

// ---------------------------------------------------------------------------
// Test generators
// ---------------------------------------------------------------------------

/// A generator that records every entity and message name it receives.
struct CollectingGenerator {
    entities: Arc<Mutex<Vec<String>>>,
    messages: Arc<Mutex<Vec<String>>>,
}

impl CodeGenerator for CollectingGenerator {
    fn name(&self) -> &str {
        "collecting"
    }

    fn generate_entity(
        &self,
        _ctx: &GeneratorContext,
        entity: &Entity,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        self.entities.lock().unwrap().push(entity.name.clone());
        Ok(vec![])
    }

    fn generate_message(
        &self,
        _ctx: &GeneratorContext,
        message: &IrMessage,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        self.messages.lock().unwrap().push(message.name.clone());
        Ok(vec![])
    }
}

/// A generator that produces one file per message.
struct FileProducingGenerator;

impl CodeGenerator for FileProducingGenerator {
    fn name(&self) -> &str {
        "file-producer"
    }

    fn generate_message(
        &self,
        _ctx: &GeneratorContext,
        message: &IrMessage,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        Ok(vec![GeneratedFile {
            path: format!("{}.txt", message.name.to_lowercase()),
            content: format!("// Generated from {}", message.name),
        }])
    }
}

/// A second generator that produces a file at the same path as FileProducingGenerator,
/// used to test collision detection.
struct CollidingGenerator;

impl CodeGenerator for CollidingGenerator {
    fn name(&self) -> &str {
        "colliding"
    }

    fn generate_message(
        &self,
        _ctx: &GeneratorContext,
        message: &IrMessage,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        Ok(vec![GeneratedFile {
            path: format!("{}.txt", message.name.to_lowercase()),
            content: format!("// Also generated from {}", message.name),
        }])
    }
}

/// A no-op generator used to verify builder chaining compiles and runs.
struct AnotherGenerator;

impl CodeGenerator for AnotherGenerator {
    fn name(&self) -> &str {
        "another"
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[test]
fn test_parse_empty_request() {
    let request = CodeGeneratorRequest::default();
    let mut bytes = Vec::new();
    request.encode(&mut bytes).unwrap();

    let parsed = ParsedSchema::parse(&bytes).unwrap();
    let schema = parsed.schema();
    assert!(schema.packages.is_empty());
}

#[test]
fn test_generator_receives_messages_not_entities() {
    let entities = Arc::new(Mutex::new(Vec::new()));
    let messages = Arc::new(Mutex::new(Vec::new()));

    let generator = CollectingGenerator {
        entities: Arc::clone(&entities),
        messages: Arc::clone(&messages),
    };

    let bytes = make_request_with_message("test", "test/entities.proto", "User");

    let synapse = SynapseGenerator::new().add(generator);
    let _response = synapse.generate(&bytes).unwrap();

    let collected_entities = entities.lock().unwrap();
    let collected_messages = messages.lock().unwrap();

    // Without entity options, "User" should appear as a message, not an entity.
    assert!(
        collected_entities.is_empty(),
        "expected no entities, got {:?}",
        *collected_entities
    );
    assert_eq!(
        *collected_messages,
        vec!["User".to_string()],
        "expected [\"User\"] in messages, got {:?}",
        *collected_messages
    );
}

#[test]
fn test_generator_produces_files() {
    let bytes = make_request_with_message("test", "test/entities.proto", "User");

    let synapse = SynapseGenerator::new().add(FileProducingGenerator);
    let response = synapse.generate(&bytes).unwrap();

    assert_eq!(response.file.len(), 1, "expected exactly one generated file");

    let file = &response.file[0];
    assert_eq!(file.name.as_deref(), Some("user.txt"));
    assert_eq!(
        file.content.as_deref(),
        Some("// Generated from User")
    );
}

#[test]
fn test_file_collision_detection() {
    let bytes = make_request_with_message("test", "test/entities.proto", "User");

    let synapse = SynapseGenerator::new()
        .add(FileProducingGenerator)
        .add(CollidingGenerator);

    let result = synapse.generate(&bytes);
    assert!(result.is_err(), "expected a collision error");

    let err = result.unwrap_err();
    match &err {
        GeneratorError::FileCollision {
            path,
            first,
            second,
        } => {
            assert_eq!(path, "user.txt");
            assert_eq!(first, "file-producer");
            assert_eq!(second, "colliding");
        }
        other => panic!("expected FileCollision, got: {other:?}"),
    }
}

#[test]
fn test_builder_method_chaining() {
    let generator = SynapseGenerator::new()
        .add(FileProducingGenerator)
        .add(AnotherGenerator);

    // Just verify it works by running an empty request through it.
    let request = CodeGeneratorRequest::default();
    let mut bytes = Vec::new();
    request.encode(&mut bytes).unwrap();

    let response = generator.generate(&bytes).unwrap();
    assert!(
        response.file.is_empty(),
        "empty request should produce no files"
    );
}
