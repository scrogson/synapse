# synapse-gen: Code Generator Framework

## Problem

Synapse's code generation logic is tightly coupled to `protoc-gen-synapse`. Third-party developers cannot write generators for new languages or backends without forking the entire plugin. The generation pipeline mixes proto parsing, extension extraction, and code generation into one monolithic binary.

## Solution

A new crate `synapse-gen` that provides:

1. A high-level **Intermediate Representation (IR)** that pre-merges proto descriptors with Synapse annotations
2. A **`CodeGenerator` trait** with fine-grained callbacks that third-party developers implement
3. A **builder API** to compose generators into a protoc plugin binary
4. An internal **parser** that converts `CodeGeneratorRequest` + prost-reflect extension extraction into the IR

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Audience | Third-party developers | External devs write generators as separate crates |
| Generator language | Rust only | Generators implement a Rust trait, distributed as crates |
| Abstraction level | High-level IR with raw escape hatch | Clean domain types for 99% of cases, `raw` field for edge cases |
| Trait granularity | Fine-grained callbacks | `generate_entity()`, `generate_service()`, etc. with default no-ops |
| Output format | `Vec<GeneratedFile>` (path + content string) | Language-agnostic, works for Rust/TS/Elixir/Go |
| Registration | Compile-time builder API | `SynapseGenerator::new().add(MyGen).run()` |
| Generator dependencies | Independent, no inter-generator visibility | Each generator works solely from the IR |

## Intermediate Representation

The IR pre-merges proto descriptors with their Synapse options. Every type carries a `raw` escape hatch back to the underlying prost descriptor.

### Schema & Package

```rust
/// The complete parsed proto world with all Synapse annotations resolved.
pub struct Schema<'a> {
    pub packages: Vec<Package<'a>>,
}

/// A proto package (e.g., "iam", "blog") with all its artifacts.
pub struct Package<'a> {
    pub name: String,
    pub entities: Vec<Entity<'a>>,
    pub services: Vec<Service<'a>>,
    pub enums: Vec<Enum<'a>>,
    pub messages: Vec<Message<'a>>,      // non-entity messages
    pub raw_files: Vec<&'a FileDescriptorProto>,
}
```

### Entity & Field

```rust
/// A message annotated with synapse.storage.entity.
pub struct Entity<'a> {
    pub name: String,                    // "User"
    pub table_name: String,              // "users"
    pub fields: Vec<Field<'a>>,
    pub relations: Vec<Relation>,
    pub graphql: Option<GraphQLTypeOptions>,
    pub raw: &'a DescriptorProto,
    pub raw_file: &'a FileDescriptorProto,
}

/// A field within an entity or message.
pub struct Field<'a> {
    pub name: String,                    // "email"
    pub field_type: FieldType,
    pub nullable: bool,
    pub repeated: bool,
    pub column: Option<ColumnOptions>,
    pub validation: Option<ValidationRules>,
    pub graphql: Option<GraphQLFieldOptions>,
    pub raw: &'a FieldDescriptorProto,
}

pub enum FieldType {
    Int32, Int64, Float, Double, Bool, String, Bytes,
    Timestamp, Duration, Struct,
    Enum(String),
    Message(String),
}
```

### Service & Method

```rust
/// An RPC service with Synapse annotations.
pub struct Service<'a> {
    pub name: String,                    // "UserService"
    pub methods: Vec<Method<'a>>,
    pub storage: Option<StorageServiceOptions>,
    pub graphql: Option<GraphQLServiceOptions>,
    pub grpc: Option<GrpcServiceOptions>,
    pub raw: &'a ServiceDescriptorProto,
    pub raw_file: &'a FileDescriptorProto,
}

/// An RPC method.
pub struct Method<'a> {
    pub name: String,                    // "CreateUser"
    pub input_type: String,
    pub output_type: String,
    pub graphql: Option<GraphQLMethodOptions>,
    pub grpc: Option<GrpcMethodOptions>,
    pub raw: &'a MethodDescriptorProto,
}
```

### Enum

```rust
pub struct Enum<'a> {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub raw: &'a EnumDescriptorProto,
    pub raw_file: &'a FileDescriptorProto,
}

pub struct EnumVariant {
    pub name: String,
    pub number: i32,
}
```

## CodeGenerator Trait

```rust
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
}

pub struct GeneratorContext<'a> {
    pub schema: &'a Schema<'a>,
    pub package: &'a Package<'a>,
}

pub trait CodeGenerator: Send + Sync {
    fn name(&self) -> &str;

    fn generate_entity(&self, ctx: &GeneratorContext, entity: &Entity)
        -> Result<Vec<GeneratedFile>, GeneratorError> { Ok(vec![]) }

    fn generate_service(&self, ctx: &GeneratorContext, service: &Service)
        -> Result<Vec<GeneratedFile>, GeneratorError> { Ok(vec![]) }

    fn generate_enum(&self, ctx: &GeneratorContext, enum_: &Enum)
        -> Result<Vec<GeneratedFile>, GeneratorError> { Ok(vec![]) }

    fn generate_message(&self, ctx: &GeneratorContext, message: &Message)
        -> Result<Vec<GeneratedFile>, GeneratorError> { Ok(vec![]) }

    fn finalize_package(&self, ctx: &GeneratorContext)
        -> Result<Vec<GeneratedFile>, GeneratorError> { Ok(vec![]) }

    fn finalize(&self, schema: &Schema)
        -> Result<Vec<GeneratedFile>, GeneratorError> { Ok(vec![]) }
}
```

## Error Handling

```rust
pub enum GeneratorError {
    MissingOption { entity: String, option: String },
    InvalidOption { entity: String, option: String, message: String },
    Other(Box<dyn std::error::Error + Send + Sync>),
}
```

File path collisions between generators are reported as errors by the runner.

## Builder & Runner

```rust
pub struct SynapseGenerator {
    generators: Vec<Box<dyn CodeGenerator>>,
}

impl SynapseGenerator {
    pub fn new() -> Self;
    pub fn add<G: CodeGenerator + 'static>(self, generator: G) -> Self;

    /// Full protoc plugin lifecycle:
    /// 1. Read CodeGeneratorRequest from stdin
    /// 2. Extract extensions via prost-reflect, build Schema IR
    /// 3. For each package, call generators for each entity/service/enum/message
    /// 4. Call finalize_package(), then finalize()
    /// 5. Detect file path collisions
    /// 6. Write CodeGeneratorResponse to stdout
    pub fn run(self) -> Result<(), Box<dyn std::error::Error>>;
}
```

### Third-Party Usage

```rust
// my-protoc-plugin/src/main.rs
use synapse_gen::SynapseGenerator;
use my_ecto_gen::EctoGenerator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    SynapseGenerator::new()
        .add(EctoGenerator::new())
        .run()
}
```

### protoc-gen-synapse Becomes a Thin Binary

```rust
// protoc-gen-synapse/src/main.rs
use synapse_gen::SynapseGenerator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    SynapseGenerator::new()
        .add(SeaOrmEntityGenerator)
        .add(StorageTraitGenerator)
        .add(GraphQLGenerator)
        .add(GrpcGenerator)
        .add(ValidateGenerator)
        .add(TypeScriptGenerator)
        .run()
}
```

## Crate Structure

```
synapse-gen/
├── Cargo.toml
└── src/
    ├── lib.rs              # pub use everything
    ├── ir/
    │   ├── mod.rs
    │   ├── schema.rs       # Schema, Package
    │   ├── entity.rs       # Entity, Field, FieldType, ColumnOptions
    │   ├── service.rs      # Service, Method
    │   ├── enum_.rs        # Enum, EnumVariant
    │   ├── message.rs      # Message (non-entity)
    │   ├── relation.rs     # Relation, RelationType
    │   ├── graphql.rs      # GraphQL*Options types
    │   ├── grpc.rs         # Grpc*Options types
    │   └── validate.rs     # ValidationRules, FieldRules
    ├── generator.rs        # CodeGenerator trait, GeneratedFile, GeneratorError
    ├── builder.rs          # SynapseGenerator builder + run()
    └── parser.rs           # CodeGeneratorRequest -> Schema (extension extraction)
```

**Public API:** `ir::*`, `CodeGenerator`, `GeneratedFile`, `GeneratorError`, `SynapseGenerator`, `GeneratorContext`.

**Dependencies:** `prost`, `prost-types`, `prost-reflect` (already in workspace).

## Migration Path

Existing built-in generators refactored to implement `CodeGenerator`:

| Before | After |
|--------|-------|
| `fn generate(file: &FileDescriptorProto, msg: &DescriptorProto)` | `fn generate_entity(&self, ctx: &GeneratorContext, entity: &Entity)` |
| Query global `OPTIONS_CACHE` | Read fields directly from IR (`entity.table_name`, etc.) |
| Return `Option<File>` (prost response file) | Return `Vec<GeneratedFile>` |

The global `OPTIONS_CACHE` is eliminated — replaced by the IR. Extension extraction via prost-reflect moves into `parser.rs` where it builds the `Schema` once upfront.
