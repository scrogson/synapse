# Validate Generator Migration Design

## Goal

Migrate the `validate` domain-type generator from the legacy `protoc-gen-synapse` orchestrator to use the `synapse-gen` framework's `CodeGenerator` trait and IR types. This establishes the pattern for migrating all remaining generators.

## Strategy

**Incremental migration** — migrate one generator at a time, starting with `validate` (the simplest, most isolated generator). The orchestrator runs both legacy and new generators, merging results into a single `CodeGeneratorResponse`.

## Changes

### 1. IR Enhancement: Add validate options to Entity

A proto message can have both `synapse.storage.entity` AND `synapse.validate.message` annotations. The Entity IR type needs to carry validate options so the validate generator can handle entities too.

**File:** `synapse-gen/src/ir/entity.rs`

Add `validate: Option<ValidateMessageOptions>` to `Entity<'a>`.

**File:** `synapse-gen/src/parser/ir_builder.rs`

Wire validate option lookup in `build_entity()`, same pattern as existing `graphql` option.

### 2. ValidateGenerator

**File:** `protoc-gen-synapse/src/validate/mod.rs` (rewrite)

A `ValidateGenerator` struct implementing `synapse_gen::CodeGenerator`:

- `name()` returns `"validate"`
- `generate_message()` checks `message.validate` options. If `generate_conversion` is true and `name` is non-empty, generates domain type with validation.
- `generate_entity()` checks `entity.validate` options. Same generation logic.
- Both delegate to a shared internal function.

**Input (IR types used):**
- `Message.validate: Option<ValidateMessageOptions>` — skip, name, generate_conversion
- `Field.validation: Option<ValidationFieldOptions>` — skip, rename, field_type, rules
- `Field.field_type: FieldType` — replaces raw proto type integer inspection
- `Field.nullable: bool` — replaces `proto3_optional` check
- `Field.repeated: bool` — replaces label check
- `Package.name` — for output file path

**Output:** `Vec<GeneratedFile>` with path `{package}/{domain_name_snake}.rs`

**Code generation:** Uses `proc-macro2/quote/syn/prettyplease` (same tooling as current).

**Generated code structure (unchanged):**
- `{DomainName}FieldError` struct (code, message, field)
- `{DomainName}` domain type struct
- `{DomainName}ValidationError` struct with `into_errors()`, `errors()`
- `TryFrom<ProtoMessage>` impl with field validation

### 3. Orchestrator Integration

**File:** `protoc-gen-synapse/src/storage/seaorm/generator.rs`

Update `generate_from_bytes()`:

1. Parse with `synapse_gen::ParsedSchema::parse(input)` (new path)
2. Still run `preprocess_request_bytes(input)` for legacy generators (OPTIONS_CACHE)
3. Run `ValidateGenerator` over all messages and entities from the IR schema
4. Remove `validate::generate()` calls from the legacy orchestration loop
5. Merge results from both paths into the response

### 4. Dependencies

**File:** `protoc-gen-synapse/Cargo.toml`

Add `synapse-gen = { path = "../synapse-gen" }` as a dependency.

## Output Compatibility

Functionally equivalent output — same code structure and behavior, minor formatting differences acceptable.

## Future

Once all generators are migrated to `CodeGenerator` impls, the orchestrator is replaced by:
```rust
SynapseGenerator::new()
    .add(ValidateGenerator)
    .add(EntityGenerator)
    .add(GraphQLGenerator)
    .add(GrpcGenerator)
    .add(TypeScriptGenerator)
    .run()
```

And the global `OPTIONS_CACHE` is removed entirely.
