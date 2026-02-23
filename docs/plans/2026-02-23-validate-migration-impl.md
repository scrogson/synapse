# Validate Generator Migration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Migrate the validate domain-type generator from legacy OPTIONS_CACHE + raw descriptors to use synapse-gen's CodeGenerator trait and IR types.

**Architecture:** Add `validate` option to Entity IR, create `ValidateGenerator` implementing `CodeGenerator`, update the orchestrator to run it alongside legacy generators via dual parsing. The validate generator uses IR types (Message, Entity, Field, FieldType, ValidationRules) and outputs `Vec<GeneratedFile>`.

**Tech Stack:** Rust, synapse-gen (CodeGenerator trait, IR types), proc-macro2/quote/syn/prettyplease, prost

---

### Task 1: Add validate option to Entity IR type

**Files:**
- Modify: `synapse-gen/src/ir/entity.rs:1-16`
- Modify: `synapse-gen/src/parser/ir_builder.rs:123-164`
- Modify: `synapse-gen/tests/integration_test.rs`

**Step 1: Write the failing test**

Add a test to `synapse-gen/tests/integration_test.rs` that creates an entity and verifies the `validate` field exists on it. Since we can't inject synapse.validate options without prost-reflect, verify the field is `None` when no options are present.

```rust
#[test]
fn test_entity_has_validate_field() {
    // Build a request with a message that has entity options
    // (We can't easily inject entity options via raw prost, so we use
    //  the IR builder directly via ParsedSchema and check the Entity type.)
    // For now, just verify Entity struct has the validate field by
    // checking that it's None when no validate options are present.
    let bytes = make_request_with_message("test", "test/entities.proto", "User");
    let parsed = ParsedSchema::parse(&bytes).unwrap();
    let schema = parsed.schema();

    // Without entity options, User is a Message not an Entity.
    // But we can verify Entity struct compiles with the validate field
    // by constructing one manually.
    let entity = synapse_gen::ir::Entity {
        name: "Test".to_string(),
        table_name: "tests".to_string(),
        skip: false,
        fields: vec![],
        relations: vec![],
        graphql: None,
        graphql_resolver: None,
        validate: None,
        raw: &prost_types::DescriptorProto::default(),
        raw_file: &prost_types::FileDescriptorProto::default(),
    };
    assert!(entity.validate.is_none());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p synapse-gen test_entity_has_validate_field`
Expected: FAIL — `Entity` struct has no `validate` field

**Step 3: Add validate field to Entity**

In `synapse-gen/src/ir/entity.rs`, add the import and field:

```rust
use prost_types::{DescriptorProto, FileDescriptorProto};
use super::{Field, Relation};
use super::options::{GraphQLTypeOptions, GraphQLResolverOptions, ValidateMessageOptions};

#[derive(Debug, Clone)]
pub struct Entity<'a> {
    pub name: String,
    pub table_name: String,
    pub skip: bool,
    pub fields: Vec<Field<'a>>,
    pub relations: Vec<Relation>,
    pub graphql: Option<GraphQLTypeOptions>,
    pub graphql_resolver: Option<GraphQLResolverOptions>,
    pub validate: Option<ValidateMessageOptions>,
    pub raw: &'a DescriptorProto,
    pub raw_file: &'a FileDescriptorProto,
}
```

**Step 4: Wire validate option lookup in ir_builder.rs**

In `synapse-gen/src/parser/ir_builder.rs`, in the `build_entity` function (line ~130-164), add validate option lookup and include it in the Entity construction. Add this after the `graphql_resolver` lookup (line ~148-151):

```rust
    let validate = options
        .validate_message_options
        .get(&(file_name.to_string(), msg_name.to_string()))
        .map(|v| ValidateMessageOptions {
            skip: v.skip,
            name: v.name.clone(),
            generate_conversion: v.generate_conversion,
        });
```

And include `validate,` in the Entity struct literal (after `graphql_resolver,`).

**Step 5: Run test to verify it passes**

Run: `cargo test -p synapse-gen test_entity_has_validate_field`
Expected: PASS

**Step 6: Run all synapse-gen tests to check for regressions**

Run: `cargo test -p synapse-gen`
Expected: All 6 tests pass (5 existing + 1 new)

**Step 7: Commit**

```bash
git add synapse-gen/src/ir/entity.rs synapse-gen/src/parser/ir_builder.rs synapse-gen/tests/integration_test.rs
git commit -m "Add validate option to Entity IR type"
```

---

### Task 2: Add synapse-gen dependency to protoc-gen-synapse

**Files:**
- Modify: `protoc-gen-synapse/Cargo.toml:15`

**Step 1: Add the dependency**

Add `synapse-gen` as a path dependency in `protoc-gen-synapse/Cargo.toml` under `[dependencies]`:

```toml
synapse-gen = { path = "../synapse-gen" }
```

**Step 2: Verify it compiles**

Run: `cargo check -p protoc-gen-synapse`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add protoc-gen-synapse/Cargo.toml
git commit -m "Add synapse-gen dependency to protoc-gen-synapse"
```

---

### Task 3: Rewrite ValidateGenerator using CodeGenerator trait

This is the core task. Rewrite `protoc-gen-synapse/src/validate/mod.rs` to define a `ValidateGenerator` struct that implements `synapse_gen::CodeGenerator`. The generator uses IR types instead of raw descriptors and OPTIONS_CACHE.

**Files:**
- Rewrite: `protoc-gen-synapse/src/validate/mod.rs`

**Step 1: Write the new validate module**

Replace the entire contents of `protoc-gen-synapse/src/validate/mod.rs` with the new implementation. The key changes from the old code:

- **No OPTIONS_CACHE access** — uses `message.validate`, `entity.validate`, and `field.validation` from IR types
- **No raw descriptor inspection** — uses `field.field_type` (IR `FieldType` enum), `field.nullable`, `field.repeated` instead of proto type integers and proto3_optional
- **Implements `CodeGenerator`** — `generate_message()` and `generate_entity()` callbacks
- **Returns `Vec<GeneratedFile>`** — instead of `Option<File>`
- **Uses `ctx.package.name`** — for output file path instead of `file.package`

```rust
//! Domain type generation from synapse.validate options
//!
//! Implements `synapse_gen::CodeGenerator` to generate validated domain types
//! from protobuf messages with `(synapse.validate.message).generate_conversion = true`.
//!
//! Generated code includes:
//! - Domain type struct with validated fields
//! - ValidationError with `into_errors()` for rich error responses
//! - `TryFrom<ProtoMessage>` implementation with validation

use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use synapse_gen::ir::options::{ValidateMessageOptions, ValidationRules};
use synapse_gen::ir::{Entity, Field, FieldType, Message, ValidationFieldOptions};
use synapse_gen::{CodeGenerator, GeneratedFile, GeneratorContext, GeneratorError};

/// Generator for validated domain types.
///
/// Produces a Rust file per message/entity that has
/// `(synapse.validate.message).generate_conversion = true` and a non-empty `name`.
pub struct ValidateGenerator;

impl CodeGenerator for ValidateGenerator {
    fn name(&self) -> &str {
        "validate"
    }

    fn generate_message(
        &self,
        ctx: &GeneratorContext,
        message: &Message,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        let validate = match &message.validate {
            Some(v) if v.generate_conversion && !v.name.is_empty() => v,
            _ => return Ok(vec![]),
        };
        generate_domain_type(&ctx.package.name, &message.name, &message.fields, validate)
    }

    fn generate_entity(
        &self,
        ctx: &GeneratorContext,
        entity: &Entity,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        let validate = match &entity.validate {
            Some(v) if v.generate_conversion && !v.name.is_empty() => v,
            _ => return Ok(vec![]),
        };
        generate_domain_type(&ctx.package.name, &entity.name, &entity.fields, validate)
    }
}

// ---------------------------------------------------------------------------
// Shared generation logic
// ---------------------------------------------------------------------------

fn generate_domain_type(
    package_name: &str,
    proto_name: &str,
    fields: &[Field],
    validate: &ValidateMessageOptions,
) -> Result<Vec<GeneratedFile>, GeneratorError> {
    let domain_name = &validate.name;
    let module_name = domain_name.to_snake_case();
    let output_path = format!("{}/{}.rs", package_name.replace('.', "/"), module_name);

    let domain_ident = format_ident!("{}", domain_name);
    let proto_ident = format_ident!("{}", proto_name);
    let error_ident = format_ident!("{}ValidationError", domain_name);
    let field_error_ident = format_ident!("{}FieldError", domain_name);

    let (field_defs, field_validations, field_assignments) =
        generate_fields(fields, &field_error_ident);

    let module_doc = format!("Domain type {} generated from {}", domain_name, proto_name);
    let struct_doc = format!("Validated domain type for {}", proto_name);
    let error_doc = format!("Validation error for {} conversion", domain_name);

    let code = quote! {
        #![doc = #module_doc]
        //!
        //! Generated by protoc-gen-synapse from protobuf message definition.
        //! @generated

        #![allow(missing_docs)]
        #![allow(unused_imports)]

        use super::prelude::*;

        /// A single field validation error
        #[derive(Debug, Clone)]
        pub struct #field_error_ident {
            /// Error code (e.g., "required", "invalid_email", "min_length")
            pub code: String,
            /// Human-readable error message
            pub message: String,
            /// Field name that failed validation
            pub field: String,
        }

        impl std::fmt::Display for #field_error_ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}: {}", self.field, self.message)
            }
        }

        impl std::error::Error for #field_error_ident {}

        #[doc = #struct_doc]
        #[derive(Debug, Clone)]
        pub struct #domain_ident {
            #(#field_defs)*
        }

        #[doc = #error_doc]
        #[derive(Debug)]
        pub struct #error_ident {
            errors: Vec<#field_error_ident>,
        }

        impl #error_ident {
            /// Convert validation errors to a list of field errors
            pub fn into_errors(self) -> Vec<#field_error_ident> {
                self.errors
            }

            /// Get a reference to the validation errors
            pub fn errors(&self) -> &[#field_error_ident] {
                &self.errors
            }
        }

        impl std::fmt::Display for #error_ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "validation failed: {} error(s)", self.errors.len())
            }
        }

        impl std::error::Error for #error_ident {}

        impl TryFrom<#proto_ident> for #domain_ident {
            type Error = #error_ident;

            fn try_from(request: #proto_ident) -> Result<Self, Self::Error> {
                let mut errors = Vec::new();

                #(#field_validations)*

                if !errors.is_empty() {
                    return Err(#error_ident { errors });
                }

                Ok(Self {
                    #(#field_assignments)*
                })
            }
        }
    };

    let content = code.to_string();
    let formatted = match syn::parse_file(&content) {
        Ok(parsed) => prettyplease::unparse(&parsed),
        Err(_) => content,
    };

    Ok(vec![GeneratedFile {
        path: output_path,
        content: formatted,
    }])
}

// ---------------------------------------------------------------------------
// Field processing
// ---------------------------------------------------------------------------

fn generate_fields(
    fields: &[Field],
    field_error_ident: &proc_macro2::Ident,
) -> (Vec<TokenStream>, Vec<TokenStream>, Vec<TokenStream>) {
    let mut field_defs = Vec::new();
    let mut field_validations = Vec::new();
    let mut field_assignments = Vec::new();

    for field in fields {
        let field_opts = field.validation.as_ref();

        // Skip this field if marked as skip
        if field_opts.is_some_and(|opts| opts.skip) {
            continue;
        }

        let field_ident = format_ident!("{}", field.name.to_snake_case());

        // Check if field has custom type
        let custom_type = field_opts.and_then(|opts| {
            if opts.field_type.is_empty() {
                None
            } else {
                Some(opts.field_type.clone())
            }
        });

        // Determine Rust type based on IR field type or custom type
        let rust_type = if let Some(ref type_name) = custom_type {
            let type_ident = format_ident!("{}", type_name);
            quote! { #type_ident }
        } else {
            field_type_to_rust_token(&field.field_type, field.nullable, field.repeated)
        };

        // Generate field definition
        field_defs.push(quote! {
            pub #field_ident: #rust_type,
        });

        // Generate validation code based on field options
        if let Some(opts) = field_opts {
            if let Some(ref rules) = opts.rules {
                let validation = generate_field_validation(
                    &field.name,
                    &field_ident,
                    &field.field_type,
                    field.nullable,
                    rules,
                    field_error_ident,
                );
                if !validation.is_empty() {
                    field_validations.push(validation);
                }
            }
        }

        // Generate field assignment (with type conversion if custom type)
        if custom_type.is_some() {
            let type_ident = format_ident!("{}", custom_type.unwrap());
            let field_name_str = &field.name;
            field_assignments.push(quote! {
                #field_ident: #type_ident::from_str(&request.#field_ident)
                    .map_err(|e| errors.push(#field_error_ident {
                        code: "invalid_format".to_string(),
                        message: e.to_string(),
                        field: #field_name_str.to_string(),
                    }))
                    .unwrap_or_default(),
            });
        } else {
            field_assignments.push(quote! {
                #field_ident: request.#field_ident,
            });
        }
    }

    (field_defs, field_validations, field_assignments)
}

// ---------------------------------------------------------------------------
// Field type to Rust token mapping
// ---------------------------------------------------------------------------

/// Map IR FieldType to Rust type token, wrapping with Option/Vec as needed.
fn field_type_to_rust_token(ft: &FieldType, nullable: bool, repeated: bool) -> TokenStream {
    let base = match ft {
        FieldType::Double => quote! { f64 },
        FieldType::Float => quote! { f32 },
        FieldType::Int64 | FieldType::SFixed64 | FieldType::SInt64 => quote! { i64 },
        FieldType::UInt64 | FieldType::Fixed64 => quote! { u64 },
        FieldType::Int32 | FieldType::SFixed32 | FieldType::SInt32 => quote! { i32 },
        FieldType::UInt32 | FieldType::Fixed32 => quote! { u32 },
        FieldType::Bool => quote! { bool },
        FieldType::String => quote! { String },
        FieldType::Bytes => quote! { Vec<u8> },
        FieldType::Timestamp => quote! { Timestamp },
        FieldType::Duration => quote! { Duration },
        FieldType::Struct => quote! { Value },
        FieldType::Message(name) | FieldType::Enum(name) => {
            let short = name.rsplit('.').next().unwrap_or(name).to_upper_camel_case();
            let ident = format_ident!("{}", short);
            quote! { #ident }
        }
    };

    if repeated {
        quote! { Vec<#base> }
    } else if nullable {
        quote! { Option<#base> }
    } else {
        base
    }
}

// ---------------------------------------------------------------------------
// Validation code generation
// ---------------------------------------------------------------------------

fn generate_field_validation(
    field_name: &str,
    field_ident: &proc_macro2::Ident,
    field_type: &FieldType,
    nullable: bool,
    rules: &ValidationRules,
    field_error_ident: &proc_macro2::Ident,
) -> TokenStream {
    let mut validations = Vec::new();
    let is_string = matches!(field_type, FieldType::String);
    let is_bytes = matches!(field_type, FieldType::Bytes);

    // Required validation
    if rules.required {
        if nullable {
            validations.push(quote! {
                if request.#field_ident.is_none() {
                    errors.push(#field_error_ident {
                        code: "required".to_string(),
                        message: format!("{} is required", #field_name),
                        field: #field_name.to_string(),
                    });
                }
            });
        } else if is_string || is_bytes {
            validations.push(quote! {
                if request.#field_ident.is_empty() {
                    errors.push(#field_error_ident {
                        code: "required".to_string(),
                        message: format!("{} is required", #field_name),
                        field: #field_name.to_string(),
                    });
                }
            });
        }
    }

    // String-specific validations
    if is_string {
        // Email validation
        if rules.email {
            if nullable {
                validations.push(quote! {
                    if let Some(ref value) = request.#field_ident {
                        if !value.is_empty() && !value.contains('@') {
                            errors.push(#field_error_ident {
                                code: "invalid_email".to_string(),
                                message: format!("{} must be a valid email address", #field_name),
                                field: #field_name.to_string(),
                            });
                        }
                    }
                });
            } else {
                validations.push(quote! {
                    if !request.#field_ident.is_empty() && !request.#field_ident.contains('@') {
                        errors.push(#field_error_ident {
                            code: "invalid_email".to_string(),
                            message: format!("{} must be a valid email address", #field_name),
                            field: #field_name.to_string(),
                        });
                    }
                });
            }
        }

        // Length validation
        if let Some(ref length) = rules.length {
            if length.min.is_some() && length.min.unwrap() > 0 {
                let min_val = length.min.unwrap() as usize;
                if nullable {
                    validations.push(quote! {
                        if let Some(ref value) = request.#field_ident {
                            if value.len() < #min_val {
                                errors.push(#field_error_ident {
                                    code: "min_length".to_string(),
                                    message: format!("{} must be at least {} characters", #field_name, #min_val),
                                    field: #field_name.to_string(),
                                });
                            }
                        }
                    });
                } else {
                    validations.push(quote! {
                        if request.#field_ident.len() < #min_val {
                            errors.push(#field_error_ident {
                                code: "min_length".to_string(),
                                message: format!("{} must be at least {} characters", #field_name, #min_val),
                                field: #field_name.to_string(),
                            });
                        }
                    });
                }
            }

            if length.max.is_some() && length.max.unwrap() > 0 {
                let max_val = length.max.unwrap() as usize;
                if nullable {
                    validations.push(quote! {
                        if let Some(ref value) = request.#field_ident {
                            if value.len() > #max_val {
                                errors.push(#field_error_ident {
                                    code: "max_length".to_string(),
                                    message: format!("{} must be at most {} characters", #field_name, #max_val),
                                    field: #field_name.to_string(),
                                });
                            }
                        }
                    });
                } else {
                    validations.push(quote! {
                        if request.#field_ident.len() > #max_val {
                            errors.push(#field_error_ident {
                                code: "max_length".to_string(),
                                message: format!("{} must be at most {} characters", #field_name, #max_val),
                                field: #field_name.to_string(),
                            });
                        }
                    });
                }
            }

            if length.equal.is_some() && length.equal.unwrap() > 0 {
                let equal_val = length.equal.unwrap() as usize;
                if nullable {
                    validations.push(quote! {
                        if let Some(ref value) = request.#field_ident {
                            if value.len() != #equal_val {
                                errors.push(#field_error_ident {
                                    code: "exact_length".to_string(),
                                    message: format!("{} must be exactly {} characters", #field_name, #equal_val),
                                    field: #field_name.to_string(),
                                });
                            }
                        }
                    });
                } else {
                    validations.push(quote! {
                        if request.#field_ident.len() != #equal_val {
                            errors.push(#field_error_ident {
                                code: "exact_length".to_string(),
                                message: format!("{} must be exactly {} characters", #field_name, #equal_val),
                                field: #field_name.to_string(),
                            });
                        }
                    });
                }
            }
        }

        // Pattern validation
        if !rules.pattern.is_empty() {
            let pattern = &rules.pattern;
            if nullable {
                validations.push(quote! {
                    if let Some(ref value) = request.#field_ident {
                        let re = regex::Regex::new(#pattern).expect("invalid regex pattern");
                        if !re.is_match(value) {
                            errors.push(#field_error_ident {
                                code: "pattern".to_string(),
                                message: format!("{} does not match required pattern", #field_name),
                                field: #field_name.to_string(),
                            });
                        }
                    }
                });
            } else {
                validations.push(quote! {
                    {
                        let re = regex::Regex::new(#pattern).expect("invalid regex pattern");
                        if !re.is_match(&request.#field_ident) {
                            errors.push(#field_error_ident {
                                code: "pattern".to_string(),
                                message: format!("{} does not match required pattern", #field_name),
                                field: #field_name.to_string(),
                            });
                        }
                    }
                });
            }
        }
    }

    quote! {
        #(#validations)*
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo check -p protoc-gen-synapse`
Expected: Compiles. The module is defined but the old `validate::generate` is no longer exported; it's replaced by `ValidateGenerator`. The orchestrator (Task 4) still calls the old function — that's fine, we'll fix it in the next task. If there's a compilation error because the orchestrator references `validate::generate`, we handle that in Task 4.

> **Note:** If compilation fails because `generator.rs` still calls `validate::generate`, temporarily add a `#[allow(dead_code)]` or keep the old `generate` function as a private unused function until Task 4 removes the calls. Alternatively, do Task 3 and Task 4 together in one compile cycle.

**Step 3: Commit**

```bash
git add protoc-gen-synapse/src/validate/mod.rs
git commit -m "Rewrite validate generator using synapse-gen CodeGenerator trait"
```

---

### Task 4: Wire ValidateGenerator into the orchestrator

Update the orchestrator to use `ParsedSchema` for the validate generator while keeping legacy generators unchanged. Remove old `validate::generate()` calls and replace them with the new `ValidateGenerator`.

**Files:**
- Modify: `protoc-gen-synapse/src/storage/seaorm/generator.rs:1-191`

**Step 1: Update imports**

At the top of `generator.rs`, add synapse-gen imports and the validate generator import. Remove the `validate` import from the existing imports (line 9):

Replace:
```rust
use crate::{graphql, grpc, typescript, validate};
```
With:
```rust
use crate::{graphql, grpc, typescript};
use crate::validate::ValidateGenerator;
use synapse_gen::{CodeGenerator, GeneratorContext, ParsedSchema};
```

**Step 2: Update `generate_from_bytes` to do dual parsing**

Replace the current `generate_from_bytes` function (lines 180-190) with:

```rust
pub fn generate_from_bytes(bytes: &[u8]) -> Result<CodeGeneratorResponse, GeneratorError> {
    // Parse with synapse-gen for new generators (validate)
    let parsed = ParsedSchema::parse(bytes)
        .map_err(|e| GeneratorError::DecodeError(e.to_string()))?;

    // Pre-process bytes to extract extension data for legacy generators
    options::preprocess_request_bytes(bytes).map_err(GeneratorError::DecodeError)?;

    // Decode with prost for legacy generators
    let request = CodeGeneratorRequest::decode(bytes)
        .map_err(|e| GeneratorError::DecodeError(e.to_string()))?;

    // Run new generators via synapse-gen IR
    let schema = parsed.schema();
    let validate_gen = ValidateGenerator;
    let mut validate_files = Vec::new();

    for package in &schema.packages {
        let ctx = GeneratorContext {
            schema: &schema,
            package,
        };
        for message in &package.messages {
            validate_files.extend(
                validate_gen
                    .generate_message(&ctx, message)
                    .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
            );
        }
        for entity in &package.entities {
            validate_files.extend(
                validate_gen
                    .generate_entity(&ctx, entity)
                    .map_err(|e| GeneratorError::CodeGenError(e.to_string()))?,
            );
        }
    }

    // Run legacy generators
    let mut response = generate(request)?;

    // Merge validate files into the response
    for f in validate_files {
        response.file.push(prost_types::compiler::code_generator_response::File {
            name: Some(f.path),
            content: Some(f.content),
            ..Default::default()
        });
    }

    Ok(response)
}
```

**Step 3: Remove validate calls from `generate()`**

In the `generate()` function, remove the two places where `validate::generate` is called:

1. **Line 57-59** (inside `entity_file_map` loop) — remove:
```rust
            // Generate domain type if has validate options with generate_conversion
            if let Some(generated) = validate::generate(proto_file, message)? {
                files.push(generated);
            }
```

2. **Lines 83-86** (inside non-entity message loop) — remove:
```rust
            // Generate domain type if has validate options with generate_conversion
            if let Some(generated) = validate::generate(file_descriptor, message)? {
                files.push(generated);
            }
```

**Step 4: Verify it compiles**

Run: `cargo check -p protoc-gen-synapse`
Expected: Compiles successfully. The `validate` module is no longer called from `generator.rs`; it only exports `ValidateGenerator`.

**Step 5: Verify the `validate` module's public API**

Make sure `protoc-gen-synapse/src/validate/mod.rs` exports `ValidateGenerator` as `pub struct` and that `protoc-gen-synapse/src/main.rs` still has `mod validate;` (line 23). If the old code had `pub fn generate(...)` as the only public item, the module declaration is already there — just the exports change.

**Step 6: Commit**

```bash
git add protoc-gen-synapse/src/storage/seaorm/generator.rs protoc-gen-synapse/src/validate/mod.rs
git commit -m "Wire ValidateGenerator into orchestrator with dual parsing"
```

---

### Task 5: Verify with full workspace build

**Step 1: Run full workspace check**

Run: `cargo check --workspace`
Expected: All crates compile

**Step 2: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 3: Build protoc-gen-synapse in release mode**

Run: `cargo build -p protoc-gen-synapse --release`
Expected: Binary builds successfully

**Step 4: (Optional) Run the example to verify output**

If the example build infrastructure is available:

Run: `just example-build` (or equivalent)
Expected: Generated output is functionally equivalent. Domain type files should still be produced with the same structure (struct, ValidationError, TryFrom impl).

**Step 5: Commit any fixups if needed**

If any issues were found and fixed during verification:
```bash
git add -A
git commit -m "Fix issues found during validate migration verification"
```
