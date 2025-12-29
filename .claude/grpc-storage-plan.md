# Plan: gRPC/Tonic + Storage Trait Generation for Synapse

## Overview

Add gRPC service generation and complete storage trait generation in protoc-gen-synapse. The architecture separates concerns:
- `synapse.storage` - data access layer (storage traits)
- `synapse.grpc` - transport layer (tonic services)
- `synapse.graphql` - future API layer (placeholder)

## Architecture

```
Proto Service Definition
    ↓
┌───────────────────────────────────────────────┐
│              Generated Code                    │
├───────────────────────────────────────────────┤
│  Storage Trait (synapse.storage.service_storage)
│    async fn get_user(&self, req) -> Result<User, StorageError>
│                       ↑
│  gRPC Service (synapse.grpc.service)           │
│    impl UserService for UserGrpcService<S>     │
│      - validates request → domain type         │
│      - calls storage.get_user(domain)          │
│      - converts result → Response              │
└───────────────────────────────────────────────┘
```

## Implementation Steps

### Phase 1: New Proto Options

**Files to create:**

1. `/Users/scrogson/github/scrogson/synapse/proto/synapse/grpc/options.proto`
   - `ServiceOptions`: generate_tonic, struct_name, skip, storage_trait
   - `MethodOptions`: skip, method_name, input_type (domain type ref)

2. `/Users/scrogson/github/scrogson/synapse/proto/synapse/graphql/options.proto`
   - Placeholder with basic ServiceOptions (skip, generate)

### Phase 2: Build System Updates

**File to modify:** `/Users/scrogson/github/scrogson/synapse/protoc-gen-synapse/build.rs`
- Add new proto files to prost-build compilation
- Include in FileDescriptorSet generation

**File to modify:** `/Users/scrogson/github/scrogson/synapse/protoc-gen-synapse/src/options.rs`
- Add module includes for synapse.grpc and synapse.graphql

### Phase 3: Options Parsing

**File to modify:** `/Users/scrogson/github/scrogson/synapse/protoc-gen-synapse/src/backends/seaorm/options.rs`

Add parsing for:
- gRPC service options (extension 52001)
- gRPC method options (extension 52002)
- Validate message options (extension 51001) - for domain type resolution

Cache entries:
```rust
grpc_service_options: HashMap<(String, String), grpc::ServiceOptions>,
grpc_method_options: HashMap<(String, String, String), grpc::MethodOptions>,
validate_message_options: HashMap<(String, String), validate::MessageOptions>,
```

### Phase 4: Complete Storage Trait Generation

**File to modify:** `/Users/scrogson/github/scrogson/synapse/protoc-gen-synapse/src/backends/seaorm/service.rs`

Complete `resolve_domain_type()` function:
```rust
fn resolve_domain_type(file_name: &str, message_name: &str) -> String {
    if let Some(opts) = get_cached_validate_message_options(file_name, message_name) {
        if opts.generate_conversion && !opts.name.is_empty() {
            return opts.name.clone();
        }
    }
    message_name.to_string()
}
```

### Phase 5: gRPC Service Generation

**File to create:** `/Users/scrogson/github/scrogson/synapse/protoc-gen-synapse/src/backends/seaorm/grpc.rs`

Generates:
```rust
pub struct UserGrpcService<S: UserStorage> {
    storage: S,
}

#[tonic::async_trait]
impl user_service_server::UserService for UserGrpcService<S> {
    async fn get_user(&self, req: Request<GetUserRequest>)
        -> Result<Response<GetUserResponse>, Status>
    {
        // Optional: validate + convert to domain type
        let validated = GetUser::try_from(req.into_inner())?;

        self.storage.get_user(validated)
            .await
            .map(Response::new)
            .map_err(|e| Status::from(ServiceError::Storage(e)))
    }
}
```

**File to create:** `/Users/scrogson/github/scrogson/synapse/protoc-gen-synapse/src/backends/seaorm/errors.rs`

Error types with tonic::Status conversion:
- `ValidationError` - validation failures → Status::invalid_argument
- `ServiceError` - wraps ValidationError + StorageError
- `impl From<ServiceError> for tonic::Status`

### Phase 6: Wire into Generator

**File to modify:** `/Users/scrogson/github/scrogson/synapse/protoc-gen-synapse/src/backends/seaorm/mod.rs`
- Add `mod grpc;` and `mod errors;`

**File to modify:** `/Users/scrogson/github/scrogson/synapse/protoc-gen-synapse/src/backends/seaorm/generator.rs`
```rust
for svc in &file_descriptor.service {
    // Storage trait (existing)
    if let Some(generated) = service::generate(file_descriptor, svc)? {
        files.push(generated);
    }
    // gRPC service (new)
    if let Some(generated) = grpc::generate(file_descriptor, svc)? {
        files.push(generated);
    }
}
```

### Phase 7: Testing

**File to create:** `/Users/scrogson/github/scrogson/synapse/protoc-gen-synapse/tests/fixtures/user_service.proto`

Test service with both storage and gRPC options enabled.

## Generated File Naming

| Generator | Output Pattern |
|-----------|---------------|
| Storage trait | `{package}/{service}_storage.rs` |
| gRPC service | `{package}/{service}_grpc.rs` |

## Key Design Decisions

1. **Storage trait from `synapse.storage.service_storage`** - keeps data access separate from transport
2. **gRPC service from `synapse.grpc.service`** - transport-specific, depends on storage trait
3. **Domain types via `synapse.validate`** - optional validation layer between gRPC and storage
4. **Error conversion** - `StorageError` → `tonic::Status` via `ServiceError` wrapper
