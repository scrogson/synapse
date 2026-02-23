# synapse-gen Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create the `synapse-gen` crate — a framework for third-party code generators that parse Synapse-annotated proto files into a high-level IR and invoke pluggable generators.

**Architecture:** New crate `synapse-gen` owns the IR types, CodeGenerator trait, parser (CodeGeneratorRequest → Schema IR via prost-reflect), and a builder/runner that composes generators into a protoc plugin binary. `protoc-gen-synapse` becomes a consumer.

**Tech Stack:** Rust, prost 0.13, prost-types 0.13, prost-reflect 0.15, prost-build 0.13, thiserror 2.0

---

### Task 1: Scaffold the synapse-gen crate

**Files:**
- Create: `synapse-gen/Cargo.toml`
- Create: `synapse-gen/src/lib.rs`
- Modify: `Cargo.toml` (workspace root)

**Step 1: Create Cargo.toml**

```toml
[package]
name = "synapse-gen"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "A framework for building code generators from Synapse-annotated protobuf definitions"
keywords = ["protobuf", "codegen", "generator", "framework"]
categories = ["development-tools"]

[dependencies]
prost.workspace = true
prost-types.workspace = true
prost-reflect.workspace = true
thiserror.workspace = true
once_cell.workspace = true
heck.workspace = true

[build-dependencies]
prost-build = "0.13"
prost-reflect-build = "0.15"
```

**Step 2: Create lib.rs with module stubs**

```rust
pub mod ir;
mod parser;

mod generator;
pub use generator::{CodeGenerator, GeneratedFile, GeneratorContext, GeneratorError};

mod builder;
pub use builder::SynapseGenerator;
```

**Step 3: Add to workspace**

In root `Cargo.toml`, add `"synapse-gen"` to the `members` array.

**Step 4: Create stub modules**

Create empty files for `synapse-gen/src/ir/mod.rs`, `synapse-gen/src/generator.rs`, `synapse-gen/src/builder.rs`, `synapse-gen/src/parser.rs` with enough to compile (empty modules, placeholder types).

**Step 5: Verify it compiles**

Run: `cargo check -p synapse-gen`
Expected: PASS (may have warnings about unused, that's fine)

**Step 6: Commit**

```bash
git add synapse-gen/ Cargo.toml
git commit -m "scaffold synapse-gen crate with module stubs"
```

---

### Task 2: Define IR types

**Files:**
- Create: `synapse-gen/src/ir/mod.rs`
- Create: `synapse-gen/src/ir/schema.rs`
- Create: `synapse-gen/src/ir/entity.rs`
- Create: `synapse-gen/src/ir/field.rs`
- Create: `synapse-gen/src/ir/service.rs`
- Create: `synapse-gen/src/ir/enum_.rs`
- Create: `synapse-gen/src/ir/message.rs`
- Create: `synapse-gen/src/ir/relation.rs`
- Create: `synapse-gen/src/ir/options.rs`

The IR types are plain data structs. Every type carries a `raw` escape hatch to the underlying prost descriptor. Options are modeled as synapse-gen's own types (not re-exports of the prost-generated option types).

**Step 1: Define schema.rs and entity.rs**

`synapse-gen/src/ir/schema.rs`:
```rust
use prost_types::FileDescriptorProto;
use super::{Entity, Service, Enum, Message};

/// The complete parsed proto world with all Synapse annotations resolved.
#[derive(Debug, Clone)]
pub struct Schema<'a> {
    pub packages: Vec<Package<'a>>,
}

/// A proto package (e.g., "iam", "blog") with all its artifacts.
#[derive(Debug, Clone)]
pub struct Package<'a> {
    pub name: String,
    pub entities: Vec<Entity<'a>>,
    pub services: Vec<Service<'a>>,
    pub enums: Vec<Enum<'a>>,
    pub messages: Vec<Message<'a>>,
    pub raw_files: Vec<&'a FileDescriptorProto>,
}
```

`synapse-gen/src/ir/entity.rs`:
```rust
use prost_types::{DescriptorProto, FileDescriptorProto};
use super::{Field, Relation};
use super::options::{GraphQLTypeOptions, GraphQLResolverOptions};

#[derive(Debug, Clone)]
pub struct Entity<'a> {
    pub name: String,
    pub table_name: String,
    pub skip: bool,
    pub fields: Vec<Field<'a>>,
    pub relations: Vec<Relation>,
    pub graphql: Option<GraphQLTypeOptions>,
    pub graphql_resolver: Option<GraphQLResolverOptions>,
    pub raw: &'a DescriptorProto,
    pub raw_file: &'a FileDescriptorProto,
}
```

**Step 2: Define field.rs with FieldType**

`synapse-gen/src/ir/field.rs`:
```rust
use prost_types::FieldDescriptorProto;
use super::options::{
    ColumnOptions, ValidationRules, GraphQLFieldOptions, GraphQLFieldResolverOptions,
};

#[derive(Debug, Clone)]
pub struct Field<'a> {
    pub name: String,
    pub field_type: FieldType,
    pub number: i32,
    pub nullable: bool,
    pub repeated: bool,
    pub column: Option<ColumnOptions>,
    pub validation: Option<ValidationFieldOptions>,
    pub graphql: Option<GraphQLFieldOptions>,
    pub graphql_resolver: Option<GraphQLFieldResolverOptions>,
    pub raw: &'a FieldDescriptorProto,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    Int32,
    Int64,
    UInt32,
    UInt64,
    SInt32,
    SInt64,
    Fixed32,
    Fixed64,
    SFixed32,
    SFixed64,
    Float,
    Double,
    Bool,
    String,
    Bytes,
    Timestamp,
    Duration,
    Struct,
    Enum(String),
    Message(String),
}

#[derive(Debug, Clone)]
pub struct ValidationFieldOptions {
    pub skip: bool,
    pub rename: String,
    pub field_type: String,
    pub rules: Option<ValidationRules>,
}
```

**Step 3: Define service.rs**

```rust
use prost_types::{FileDescriptorProto, MethodDescriptorProto, ServiceDescriptorProto};
use super::options::*;

#[derive(Debug, Clone)]
pub struct Service<'a> {
    pub name: String,
    pub methods: Vec<Method<'a>>,
    pub storage: Option<StorageServiceOptions>,
    pub graphql: Option<GraphQLServiceOptions>,
    pub grpc: Option<GrpcServiceOptions>,
    pub raw: &'a ServiceDescriptorProto,
    pub raw_file: &'a FileDescriptorProto,
}

#[derive(Debug, Clone)]
pub struct Method<'a> {
    pub name: String,
    pub input_type: String,
    pub output_type: String,
    pub client_streaming: bool,
    pub server_streaming: bool,
    pub storage: Option<StorageMethodOptions>,
    pub graphql: Option<GraphQLMethodOptions>,
    pub grpc: Option<GrpcMethodOptions>,
    pub raw: &'a MethodDescriptorProto,
}
```

**Step 4: Define enum_.rs, message.rs, relation.rs**

`synapse-gen/src/ir/enum_.rs`:
```rust
use prost_types::{EnumDescriptorProto, FileDescriptorProto};
use super::options::EnumStorageOptions;

#[derive(Debug, Clone)]
pub struct Enum<'a> {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub storage: Option<EnumStorageOptions>,
    pub raw: &'a EnumDescriptorProto,
    pub raw_file: &'a FileDescriptorProto,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub number: i32,
    pub string_value: String,
    pub int_value: i32,
    pub is_default: bool,
    pub skip: bool,
}
```

`synapse-gen/src/ir/message.rs`:
```rust
use prost_types::{DescriptorProto, FileDescriptorProto};
use super::Field;
use super::options::*;

/// A non-entity message (request/response types).
#[derive(Debug, Clone)]
pub struct Message<'a> {
    pub name: String,
    pub fields: Vec<Field<'a>>,
    pub validate: Option<ValidateMessageOptions>,
    pub graphql: Option<GraphQLTypeOptions>,
    pub graphql_resolver: Option<GraphQLResolverOptions>,
    pub grpc_response: Option<GrpcResponseOptions>,
    pub raw: &'a DescriptorProto,
    pub raw_file: &'a FileDescriptorProto,
}
```

`synapse-gen/src/ir/relation.rs`:
```rust
#[derive(Debug, Clone)]
pub struct Relation {
    pub name: String,
    pub relation_type: RelationType,
    pub related: String,
    pub foreign_key: String,
    pub references: String,
    pub through: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelationType {
    HasOne,
    HasMany,
    BelongsTo,
    ManyToMany,
}
```

**Step 5: Define options.rs — all Synapse option types as IR structs**

This is the key file. These are synapse-gen's own clean types, NOT the prost-generated ones. The parser will convert from prost-generated types to these.

`synapse-gen/src/ir/options.rs`:
```rust
// --- Storage options ---

#[derive(Debug, Clone, Default)]
pub struct ColumnOptions {
    pub primary_key: bool,
    pub auto_increment: bool,
    pub unique: bool,
    pub column_name: String,
    pub default_value: String,
    pub embed: bool,
    pub column_type: String,
    pub default_expr: String,
}

#[derive(Debug, Clone, Default)]
pub struct StorageServiceOptions {
    pub generate_storage: bool,
    pub generate_implementation: bool,
    pub trait_name: String,
    pub skip: bool,
}

#[derive(Debug, Clone, Default)]
pub struct StorageMethodOptions {
    pub skip: bool,
    pub method_name: String,
    pub entity_name: String,
    pub operation: String,
}

#[derive(Debug, Clone, Default)]
pub struct EnumStorageOptions {
    pub storage_type: EnumStorageType,
    pub skip: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum EnumStorageType {
    #[default]
    Unspecified,
    String,
    Integer,
}

// --- Validation options ---

#[derive(Debug, Clone, Default)]
pub struct ValidateMessageOptions {
    pub skip: bool,
    pub name: String,
    pub generate_conversion: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ValidationRules {
    pub required: bool,
    pub email: bool,
    pub url: bool,
    pub uuid: bool,
    pub ascii: bool,
    pub alphanumeric: bool,
    pub ip: bool,
    pub ipv4: bool,
    pub ipv6: bool,
    pub credit_card: bool,
    pub phone: bool,
    pub pattern: String,
    pub length: Option<LengthConstraint>,
    pub range: Option<RangeConstraint>,
    pub unique_items: bool,
    pub dive: bool,
    pub custom: String,
    pub message: String,
    pub required_if: String,
    pub required_unless: String,
}

#[derive(Debug, Clone, Default)]
pub struct LengthConstraint {
    pub min: Option<u64>,
    pub max: Option<u64>,
    pub equal: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct RangeConstraint {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub greater_than: Option<f64>,
    pub less_than: Option<f64>,
    pub exclusive_min: bool,
    pub exclusive_max: bool,
}

// --- gRPC options ---

#[derive(Debug, Clone, Default)]
pub struct GrpcServiceOptions {
    pub skip: bool,
    pub struct_name: String,
    pub storage_trait: String,
}

#[derive(Debug, Clone, Default)]
pub struct GrpcMethodOptions {
    pub skip: bool,
    pub method_name: String,
    pub input_type: String,
}

#[derive(Debug, Clone, Default)]
pub struct GrpcResponseOptions {
    pub rich_errors: bool,
}

// --- GraphQL options ---

#[derive(Debug, Clone, Default)]
pub struct GraphQLTypeOptions {
    pub skip: bool,
    pub name: String,
    pub input: bool,
    pub node: bool,
}

#[derive(Debug, Clone, Default)]
pub struct GraphQLFieldOptions {
    pub skip: bool,
    pub name: String,
    pub deprecated: Option<String>,  // deprecation reason
    pub from_context: Option<ContextSource>,
}

#[derive(Debug, Clone, Default)]
pub struct ContextSource {
    pub path: String,
    pub required: bool,
    pub error_message: String,
}

#[derive(Debug, Clone, Default)]
pub struct GraphQLServiceOptions {
    pub skip: bool,
}

#[derive(Debug, Clone, Default)]
pub struct GraphQLMethodOptions {
    pub kind: GraphQLMethodKind,
    pub skip: bool,
    pub name: String,
    pub input_type: String,
    pub output_type: String,
    pub output_field: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum GraphQLMethodKind {
    #[default]
    Query,
    Mutation,
    Subscription,
}

// --- GraphQL resolver options ---

#[derive(Debug, Clone, Default)]
pub struct GraphQLResolverOptions {
    pub fields: Vec<VirtualField>,
    pub deno: Option<DenoConfig>,
}

#[derive(Debug, Clone, Default)]
pub struct GraphQLFieldResolverOptions {
    pub deno: Option<DenoConfig>,
}

#[derive(Debug, Clone, Default)]
pub struct GraphQLMethodResolverOptions {
    pub deno: Option<DenoConfig>,
}

#[derive(Debug, Clone, Default)]
pub struct VirtualField {
    pub name: String,
    pub field_type: String,
    pub description: Option<String>,
    pub arguments: Vec<FieldArgument>,
    pub deno: Option<DenoConfig>,
}

#[derive(Debug, Clone, Default)]
pub struct FieldArgument {
    pub name: String,
    pub field_type: String,
    pub default_value: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DenoConfig {
    pub module: String,
    pub function: Option<String>,
    pub timeout_ms: Option<u32>,
    pub permissions: Option<DenoPermissions>,
}

#[derive(Debug, Clone, Default)]
pub struct DenoPermissions {
    pub net: Vec<String>,
    pub read: Vec<String>,
    pub env: Vec<String>,
}
```

**Step 6: Wire up ir/mod.rs**

```rust
mod schema;
mod entity;
mod field;
mod service;
mod enum_;
mod message;
mod relation;
pub mod options;

pub use schema::{Schema, Package};
pub use entity::Entity;
pub use field::{Field, FieldType, ValidationFieldOptions};
pub use service::{Service, Method};
pub use enum_::{Enum, EnumVariant};
pub use message::Message;
pub use relation::{Relation, RelationType};
```

**Step 7: Verify it compiles**

Run: `cargo check -p synapse-gen`
Expected: PASS

**Step 8: Commit**

```bash
git add synapse-gen/src/ir/
git commit -m "define IR types for synapse-gen framework"
```

---

### Task 3: Define CodeGenerator trait and output types

**Files:**
- Create: `synapse-gen/src/generator.rs`

**Step 1: Write generator.rs**

```rust
use crate::ir::*;

/// A generated file with a path and content string.
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
}

/// Context passed to every generator callback.
pub struct GeneratorContext<'a> {
    pub schema: &'a Schema<'a>,
    pub package: &'a Package<'a>,
}

/// Errors that generators can produce.
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    #[error("missing required option '{option}' on {entity}")]
    MissingOption { entity: String, option: String },

    #[error("invalid option '{option}' on {entity}: {message}")]
    InvalidOption {
        entity: String,
        option: String,
        message: String,
    },

    #[error("file path collision: '{path}' produced by generators '{first}' and '{second}'")]
    FileCollision {
        path: String,
        first: String,
        second: String,
    },

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// The trait third-party developers implement to generate code
/// from Synapse-annotated protobuf definitions.
///
/// All methods default to no-op (empty Vec), so generators only
/// implement the callbacks they care about.
pub trait CodeGenerator: Send + Sync {
    /// Human-readable name, used in error messages and logging.
    fn name(&self) -> &str;

    /// Called once per entity (message with synapse.storage.entity).
    fn generate_entity(
        &self,
        _ctx: &GeneratorContext,
        _entity: &Entity,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        Ok(vec![])
    }

    /// Called once per service.
    fn generate_service(
        &self,
        _ctx: &GeneratorContext,
        _service: &Service,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        Ok(vec![])
    }

    /// Called once per enum.
    fn generate_enum(
        &self,
        _ctx: &GeneratorContext,
        _enum: &Enum,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        Ok(vec![])
    }

    /// Called once per non-entity message (request/response types).
    fn generate_message(
        &self,
        _ctx: &GeneratorContext,
        _message: &Message,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        Ok(vec![])
    }

    /// Called once per package after all entities/services/enums/messages.
    /// Use for module files, package-level rollups, index files.
    fn finalize_package(
        &self,
        _ctx: &GeneratorContext,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        Ok(vec![])
    }

    /// Called once after all packages. Use for top-level files.
    fn finalize(
        &self,
        _schema: &Schema,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        Ok(vec![])
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo check -p synapse-gen`
Expected: PASS

**Step 3: Commit**

```bash
git add synapse-gen/src/generator.rs
git commit -m "define CodeGenerator trait and output types"
```

---

### Task 4: Build system — file descriptor set and option types

The parser needs the same build-time artifacts as protoc-gen-synapse: compiled Rust types for synapse options, and a FileDescriptorSet for prost-reflect extension parsing.

**Files:**
- Create: `synapse-gen/build.rs`
- Create: `synapse-gen/src/options.rs` (internal, re-includes prost-generated types)

**Step 1: Create build.rs**

Adapt from `protoc-gen-synapse/build.rs`. The key difference: the proto path is relative to the synapse-gen crate, not protoc-gen-synapse.

```rust
use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    println!("cargo:rerun-if-changed=../proto/synapse/storage/options.proto");
    println!("cargo:rerun-if-changed=../proto/synapse/validate/options.proto");
    println!("cargo:rerun-if-changed=../proto/synapse/grpc/options.proto");
    println!("cargo:rerun-if-changed=../proto/synapse/graphql/options.proto");
    println!("cargo:rerun-if-changed=../proto/synapse/graphql/resolver.proto");
    println!("cargo:rerun-if-changed=../proto/synapse/graphql/context.proto");

    prost_build::Config::new()
        .out_dir(&out_dir)
        .compile_protos(
            &[
                "../proto/synapse/storage/options.proto",
                "../proto/synapse/validate/options.proto",
                "../proto/synapse/grpc/options.proto",
                "../proto/synapse/graphql/options.proto",
                "../proto/synapse/graphql/resolver.proto",
                "../proto/synapse/graphql/context.proto",
            ],
            &["../proto/"],
        )?;

    let fds_path = out_dir.join("file_descriptor_set.bin");
    let protobuf_include = find_protobuf_include();

    let status = Command::new("protoc")
        .args([
            "--descriptor_set_out",
            fds_path.to_str().unwrap(),
            "--include_imports",
            "--include_source_info",
            "-I../proto",
            &format!("-I{}", protobuf_include),
            "synapse/storage/options.proto",
            "synapse/validate/options.proto",
            "synapse/grpc/options.proto",
            "synapse/graphql/options.proto",
            "synapse/graphql/resolver.proto",
            "synapse/graphql/context.proto",
            "google/protobuf/compiler/plugin.proto",
        ])
        .status()?;

    if !status.success() {
        return Err("protoc failed to generate file descriptor set".into());
    }

    Ok(())
}

fn find_protobuf_include() -> String {
    if let Some(path) = Command::new("brew")
        .args(["--prefix", "protobuf"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| format!("{}/include", s.trim()))
    {
        if std::path::Path::new(&path).exists() {
            return path;
        }
    }

    for path in ["/usr/include", "/usr/local/include", "/opt/homebrew/include"] {
        let test_file = format!("{}/google/protobuf/descriptor.proto", path);
        if std::path::Path::new(&test_file).exists() {
            return path.to_string();
        }
    }

    "/usr/include".to_string()
}
```

**Step 2: Create src/options.rs**

Internal module that includes the prost-generated option types. NOT part of the public API.

```rust
//! Internal prost-generated option types.
//! Used by the parser to decode extensions; NOT part of public API.

pub mod synapse {
    pub mod storage {
        include!(concat!(env!("OUT_DIR"), "/synapse.storage.rs"));
    }
    pub mod validate {
        include!(concat!(env!("OUT_DIR"), "/synapse.validate.rs"));
    }
    pub mod grpc {
        include!(concat!(env!("OUT_DIR"), "/synapse.grpc.rs"));
    }
    pub mod graphql {
        include!(concat!(env!("OUT_DIR"), "/synapse.graphql.rs"));
    }
}
```

**Step 3: Add `mod options;` to lib.rs**

Add `mod options;` (private) to `synapse-gen/src/lib.rs`.

**Step 4: Verify it compiles**

Run: `cargo check -p synapse-gen`
Expected: PASS (build.rs generates types, options.rs includes them)

**Step 5: Commit**

```bash
git add synapse-gen/build.rs synapse-gen/src/options.rs synapse-gen/src/lib.rs
git commit -m "add build system for synapse option types and file descriptor set"
```

---

### Task 5: Parser — CodeGeneratorRequest to Schema IR

This is the core of synapse-gen. It takes raw `CodeGeneratorRequest` bytes, extracts extensions via prost-reflect, and builds the `Schema` IR.

The logic is adapted from `protoc-gen-synapse/src/storage/seaorm/options.rs` but instead of populating a global cache, it builds the IR directly.

**Files:**
- Create: `synapse-gen/src/parser.rs`
- Create: `synapse-gen/src/parser/extensions.rs`
- Create: `synapse-gen/src/parser/ir_builder.rs`
- Test: `synapse-gen/tests/parser_test.rs`

**Step 1: Write the failing test**

`synapse-gen/tests/parser_test.rs`:
```rust
use synapse_gen::parse_schema;

/// Minimal test: parse an empty CodeGeneratorRequest and get an empty Schema.
#[test]
fn test_parse_empty_request() {
    use prost::Message;
    use prost_types::compiler::CodeGeneratorRequest;

    let request = CodeGeneratorRequest::default();
    let mut bytes = Vec::new();
    request.encode(&mut bytes).unwrap();

    let schema = parse_schema(&bytes).unwrap();
    assert!(schema.packages.is_empty());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p synapse-gen test_parse_empty_request`
Expected: FAIL — `parse_schema` doesn't exist yet

**Step 3: Implement parser.rs (extension extraction)**

Restructure parser as a module directory:

`synapse-gen/src/parser/mod.rs`:
```rust
mod extensions;
mod ir_builder;

use crate::generator::GeneratorError;
use crate::ir::Schema;

/// Parse raw CodeGeneratorRequest bytes into a Schema IR.
///
/// This is the main entry point. It:
/// 1. Decodes extensions via prost-reflect (preserving synapse.* options)
/// 2. Decodes the request with standard prost
/// 3. Builds the Schema IR by merging descriptors with extracted options
pub fn parse_schema(bytes: &[u8]) -> Result<Schema<'_>, GeneratorError> {
    // Note: we need to return owned data, not borrowing from bytes.
    // The actual implementation will store the decoded request internally.
    todo!()
}
```

**Important design note:** The `Schema` has lifetime `'a` tied to `&'a DescriptorProto` etc. The parser must own the decoded `CodeGeneratorRequest` and the `Schema` borrows from it. This means we need an owning container:

```rust
/// Owns the parsed CodeGeneratorRequest and the Schema that borrows from it.
pub struct ParsedSchema {
    request: prost_types::compiler::CodeGeneratorRequest,
    // Schema built from request, borrowing from it
}

impl ParsedSchema {
    pub fn parse(bytes: &[u8]) -> Result<Self, GeneratorError> { ... }
    pub fn schema(&self) -> Schema<'_> { ... }
}
```

This avoids the self-referential struct problem by building the Schema on-demand from the owned request.

**Step 4: Implement extensions.rs**

Port the extension extraction from `protoc-gen-synapse/src/storage/seaorm/options.rs` (lines 57-706), but instead of populating a global `OptionsCache`, return an `ExtractedOptions` struct:

```rust
use std::collections::HashMap;
use crate::options::synapse::*;

pub struct ExtractedOptions {
    pub entity_options: HashMap<(String, String), storage::EntityOptions>,
    pub column_options: HashMap<(String, String, i32), storage::ColumnOptions>,
    pub enum_options: HashMap<(String, String), storage::EnumOptions>,
    pub enum_value_options: HashMap<(String, String, i32), storage::EnumValueOptions>,
    pub service_options: HashMap<(String, String), storage::ServiceOptions>,
    pub method_options: HashMap<(String, String, String), storage::MethodOptions>,
    pub grpc_service_options: HashMap<(String, String), grpc::ServiceOptions>,
    pub grpc_method_options: HashMap<(String, String, String), grpc::MethodOptions>,
    pub grpc_response_options: HashMap<(String, String), grpc::ResponseOptions>,
    pub validate_message_options: HashMap<(String, String), validate::MessageOptions>,
    pub validate_field_options: HashMap<(String, String, i32), validate::FieldOptions>,
    pub graphql_type_options: HashMap<(String, String), graphql::TypeOptions>,
    pub graphql_field_options: HashMap<(String, String, i32), graphql::FieldOptions>,
    pub graphql_service_options: HashMap<(String, String), graphql::ServiceOptions>,
    pub graphql_query_options: HashMap<(String, String, String), graphql::QueryOptions>,
    pub graphql_mutation_options: HashMap<(String, String, String), graphql::MutationOptions>,
    pub graphql_subscription_options: HashMap<(String, String, String), graphql::SubscriptionOptions>,
    pub graphql_resolver_options: HashMap<(String, String), graphql::MessageResolverOptions>,
    pub graphql_field_resolver_options: HashMap<(String, String, i32), graphql::FieldResolverOptions>,
    pub graphql_method_resolver_options: HashMap<(String, String, String), graphql::MethodResolverOptions>,
}

pub fn extract_options(bytes: &[u8]) -> Result<ExtractedOptions, String> {
    // Same logic as preprocess_request_bytes + extract_options_from_file
    // but returns ExtractedOptions instead of populating global cache
}
```

Port all `convert_to_*` functions and `extract_*_options` functions from `protoc-gen-synapse/src/storage/seaorm/options.rs`.

**Step 5: Implement ir_builder.rs**

Converts the decoded `CodeGeneratorRequest` + `ExtractedOptions` into a `Schema`:

```rust
use prost_types::compiler::CodeGeneratorRequest;
use super::extensions::ExtractedOptions;
use crate::ir::*;
use crate::ir::options::*;

pub fn build_schema<'a>(
    request: &'a CodeGeneratorRequest,
    options: &ExtractedOptions,
) -> Schema<'a> {
    let mut packages: HashMap<String, Package<'a>> = HashMap::new();

    for file in &request.proto_file {
        let pkg_name = file.package.clone().unwrap_or_default();
        let file_name = file.name.clone().unwrap_or_default();

        // Only process files that were requested for generation
        let should_generate = request.file_to_generate.contains(&file_name);
        if !should_generate {
            continue;
        }

        let package = packages.entry(pkg_name.clone()).or_insert_with(|| Package {
            name: pkg_name.clone(),
            entities: vec![],
            services: vec![],
            enums: vec![],
            messages: vec![],
            raw_files: vec![],
        });

        package.raw_files.push(file);

        // Build entities and messages from message_type
        for msg in &file.message_type {
            let msg_name = msg.name.clone().unwrap_or_default();
            let key = (file_name.clone(), msg_name.clone());

            if let Some(entity_opts) = options.entity_options.get(&key) {
                package.entities.push(build_entity(file, msg, &file_name, entity_opts, options));
            } else {
                package.messages.push(build_message(file, msg, &file_name, options));
            }
        }

        // Build enums
        for enum_desc in &file.enum_type {
            package.enums.push(build_enum(file, enum_desc, &file_name, options));
        }

        // Build services
        for svc in &file.service {
            package.services.push(build_service(file, svc, &file_name, options));
        }
    }

    Schema {
        packages: packages.into_values().collect(),
    }
}

fn build_entity<'a>(
    file: &'a FileDescriptorProto,
    msg: &'a DescriptorProto,
    file_name: &str,
    entity_opts: &storage::EntityOptions,
    options: &ExtractedOptions,
) -> Entity<'a> {
    // Convert entity_opts to IR options, build fields, relations
}

// ... similar build_* functions for each IR type
```

**Step 6: Wire parse_schema and export**

In `synapse-gen/src/parser/mod.rs`, implement `ParsedSchema::parse()` and `ParsedSchema::schema()`.

In `synapse-gen/src/lib.rs`, add:
```rust
pub use parser::ParsedSchema;
```

**Step 7: Run the test**

Run: `cargo test -p synapse-gen test_parse_empty_request`
Expected: PASS

**Step 8: Write a more substantive test**

Add a test that creates a CodeGeneratorRequest with a message that has entity options (using uninterpreted options), parses it, and verifies the Schema contains an Entity with the expected table name and fields.

**Step 9: Run all tests**

Run: `cargo test -p synapse-gen`
Expected: PASS

**Step 10: Commit**

```bash
git add synapse-gen/src/parser/ synapse-gen/tests/
git commit -m "implement parser: CodeGeneratorRequest to Schema IR"
```

---

### Task 6: Builder and Runner

**Files:**
- Create: `synapse-gen/src/builder.rs`
- Test: `synapse-gen/tests/builder_test.rs`

**Step 1: Write the failing test**

```rust
use synapse_gen::{CodeGenerator, GeneratedFile, GeneratorContext, GeneratorError, SynapseGenerator};
use synapse_gen::ir::Entity;

struct TestEntityGen;

impl CodeGenerator for TestEntityGen {
    fn name(&self) -> &str { "test-entity-gen" }

    fn generate_entity(
        &self,
        _ctx: &GeneratorContext,
        entity: &Entity,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        Ok(vec![GeneratedFile {
            path: format!("{}.txt", entity.name.to_lowercase()),
            content: format!("entity: {}", entity.name),
        }])
    }
}

#[test]
fn test_builder_runs_generator() {
    // Build a minimal CodeGeneratorRequest with one entity
    // Run SynapseGenerator with TestEntityGen
    // Verify output contains the expected file
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p synapse-gen test_builder_runs_generator`
Expected: FAIL

**Step 3: Implement builder.rs**

```rust
use std::collections::HashMap;
use std::io::{self, Read, Write};
use prost::Message;
use prost_types::compiler::CodeGeneratorResponse;

use crate::generator::{CodeGenerator, GeneratedFile, GeneratorContext, GeneratorError};
use crate::parser::ParsedSchema;

pub struct SynapseGenerator {
    generators: Vec<Box<dyn CodeGenerator>>,
}

impl SynapseGenerator {
    pub fn new() -> Self {
        Self { generators: vec![] }
    }

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

        let mut all_files: Vec<(String, GeneratedFile)> = Vec::new(); // (generator_name, file)

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

        // finalize() for each generator
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
            response.file.push(prost_types::compiler::code_generator_response::File {
                name: Some(file.path),
                content: Some(file.content),
                ..Default::default()
            });
        }

        Ok(response)
    }
}
```

**Step 4: Run tests**

Run: `cargo test -p synapse-gen`
Expected: PASS

**Step 5: Write file collision test**

```rust
#[test]
fn test_file_collision_detected() {
    // Two generators both produce "user.txt" for the same entity
    // Verify GeneratorError::FileCollision is returned
}
```

**Step 6: Run tests**

Run: `cargo test -p synapse-gen`
Expected: PASS

**Step 7: Commit**

```bash
git add synapse-gen/src/builder.rs synapse-gen/tests/
git commit -m "implement SynapseGenerator builder and runner"
```

---

### Task 7: Integration test with fixture protos

Verify the full pipeline works end-to-end using the existing test fixtures in `protoc-gen-synapse/tests/fixtures/`.

**Files:**
- Create: `synapse-gen/tests/integration_test.rs`

**Step 1: Write an integration test**

Build a CodeGeneratorRequest from the test fixture protos, run through SynapseGenerator with a simple generator that counts entities/services/enums, and verify the counts match.

```rust
use synapse_gen::{CodeGenerator, GeneratedFile, GeneratorContext, GeneratorError, SynapseGenerator};
use synapse_gen::ir::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct CountingGenerator {
    entity_count: Arc<AtomicUsize>,
    service_count: Arc<AtomicUsize>,
}

impl CodeGenerator for CountingGenerator {
    fn name(&self) -> &str { "counting" }

    fn generate_entity(&self, _ctx: &GeneratorContext, _entity: &Entity)
        -> Result<Vec<GeneratedFile>, GeneratorError>
    {
        self.entity_count.fetch_add(1, Ordering::SeqCst);
        Ok(vec![GeneratedFile {
            path: format!("entity_{}.txt", self.entity_count.load(Ordering::SeqCst)),
            content: String::new(),
        }])
    }

    fn generate_service(&self, _ctx: &GeneratorContext, _service: &Service)
        -> Result<Vec<GeneratedFile>, GeneratorError>
    {
        self.service_count.fetch_add(1, Ordering::SeqCst);
        Ok(vec![])
    }
}
```

**Step 2: Run test**

Run: `cargo test -p synapse-gen integration`
Expected: PASS

**Step 3: Commit**

```bash
git add synapse-gen/tests/
git commit -m "add integration test for synapse-gen pipeline"
```

---

### Task 8: Verify existing protoc-gen-synapse still builds

Ensure the existing crate is unaffected. No changes to protoc-gen-synapse are needed yet — that's a separate migration task.

**Step 1: Build and test existing crate**

Run: `cargo test -p protoc-gen-synapse`
Expected: PASS (no changes to this crate)

**Step 2: Build entire workspace**

Run: `cargo check --workspace`
Expected: PASS

**Step 3: Commit (if any workspace-level fixes needed)**

```bash
git commit -m "verify workspace builds with synapse-gen added"
```

---

## Future Work (not in this plan)

These are follow-up tasks after synapse-gen is established:

1. **Migrate built-in generators** — Refactor each existing generator in protoc-gen-synapse to implement `CodeGenerator` and use the IR instead of raw descriptors + options cache.
2. **Update protoc-gen-synapse main.rs** — Replace the orchestrator with `SynapseGenerator::new().add(SeaOrmEntityGenerator).add(...)`.
3. **Remove global OPTIONS_CACHE** — Once all generators use the IR, the global mutable cache can be eliminated.
4. **Publish synapse-gen** — Publish to crates.io with documentation and examples.
5. **Example third-party generator** — Write a simple example (e.g., JSON schema generator or Ecto generator) to validate the API ergonomics.
