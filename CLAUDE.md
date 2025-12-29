# Synapse

A protobuf-centric code generation framework for building type-safe backends.

## Vision

Define your data model once in Protocol Buffers with Synapse annotations. Generate type-safe database entities, gRPC services, GraphQL APIs, and validated domain types—all from a single source of truth.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Proto Definitions                        │
│                   (with synapse.* options)                   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    protoc-gen-synapse                        │
│                                                              │
│  Parses proto files, reads synapse.* extensions,             │
│  generates code for all layers                               │
└─────────────────────────────────────────────────────────────┘
                              │
          ┌───────────────────┼───────────────────┐
          ▼                   ▼                   ▼
    ┌──────────┐        ┌──────────┐        ┌──────────┐
    │  SeaORM  │        │   gRPC   │        │ GraphQL  │
    │ Entities │        │ Services │        │ Resolvers│
    └──────────┘        └──────────┘        └──────────┘
          │                   │                   │
          └───────────────────┼───────────────────┘
                              ▼
                    ┌──────────────────┐
                    │  Your Application │
                    └──────────────────┘
```

## Namespaces

| Namespace | Purpose |
|-----------|---------|
| `synapse.storage` | Database entities, columns, relations |
| `synapse.graphql` | GraphQL types, queries, mutations, field options |
| `synapse.grpc` | gRPC service generation |
| `synapse.validate` | Request validation and domain type generation |
| `synapse.relay` | Relay-style pagination types (PageInfo, filters, connections) |

### `synapse.storage`

Database/persistence layer options for entities, columns, and relations.

```protobuf
message User {
  option (synapse.storage.entity) = {
    table_name: "users"
    relations: [{
      name: "posts"
      type: RELATION_TYPE_HAS_MANY
      related: "Post"
      foreign_key: "author_id"
    }]
  };

  int64 id = 1 [(synapse.storage.column).primary_key = true];
  string email = 2 [(synapse.storage.column).unique = true];
  optional string bio = 3;  // nullable in DB
}
```

### `synapse.validate`

Request validation with domain type generation.

```protobuf
message CreateUserRequest {
  option (synapse.validate.message) = {
    generate_conversion: true
    name: "CreateUser"
  };

  string email = 1 [(synapse.validate.field).rules = {
    required: true
    email: true
  }];

  string name = 2 [(synapse.validate.field).rules = {
    required: true
    length: { min: 1, max: 100 }
  }];
}
```

Generates `CreateUser` domain type with `TryFrom<CreateUserRequest>` validation.

### `synapse.graphql`

GraphQL type and resolver generation.

```protobuf
message User {
  option (synapse.graphql.message) = { node: true };  // Relay Node interface

  int64 author_id = 5 [(synapse.graphql.field).skip = true];  // Hide from schema
}

service UserService {
  option (synapse.graphql.service) = {};

  rpc GetUser(GetUserRequest) returns (GetUserResponse) {
    option (synapse.graphql.query) = { name: "user" };
  };
}
```

## Storage Backends

| Backend | Language | Status |
|---------|----------|--------|
| SeaORM | Rust | ✅ Complete |
| Elixir (Phoenix, Ecto, Absinthe, gRPC) | Elixir | 🔮 Planned |

## Project Structure

```
synapse/
├── proto/
│   └── synapse/
│       ├── storage/options.proto   # Storage layer options
│       ├── graphql/options.proto   # GraphQL options
│       ├── grpc/options.proto      # gRPC options
│       ├── validate/options.proto  # Validation options
│       └── relay/types.proto       # Relay pagination types
├── protoc-gen-synapse/             # Main protoc plugin (Rust)
│   └── src/
│       ├── main.rs
│       ├── options/                # Proto option parsing
│       ├── storage/                # Storage trait & defaults generation
│       │   ├── seaorm/             # SeaORM-specific generation
│       │   ├── traits.rs           # Storage trait generation
│       │   └── defaults.rs         # Default impl generation
│       ├── graphql/                # GraphQL generation
│       ├── grpc/                   # gRPC service generation
│       └── validate/               # Domain type generation
├── examples/
│   └── unified/                    # Complete example
│       ├── proto/
│       │   ├── iam/                # IAM service (User, Org, Team)
│       │   └── blog/               # Blog service (Author, Post)
│       └── src/
│           ├── generated/          # Generated code
│           └── bin/                # Binaries (monolith, gateway, services)
└── Justfile                        # Development commands
```

## Key Features

### Partial Override Pattern

Storage traits support partial overrides—implement `db()` and override only the methods you need:

```rust
impl UserServiceStorage for MyStorage {
    fn db(&self) -> &DatabaseConnection { &self.db }

    // Override just this method
    async fn create_user(&self, request: CreateUser) -> Result<...> {
        // Custom logic, then delegate to default
        user_service_storage_defaults::create_user(self.db(), request).await
    }
    // All other methods use trait defaults
}
```

### Cross-Package Relations

Entities can reference types from other packages:

```protobuf
// blog/entities.proto
message Author {
  option (synapse.storage.entity) = {
    relations: [{
      name: "user"
      type: RELATION_TYPE_BELONGS_TO
      related: "iam.User"  // Cross-package reference
      foreign_key: "user_id"
    }]
  };
}
```

### Relay-Style Pagination

Auto-generated filter and connection types with cursor pagination:

```graphql
query {
  users(first: 10, filter: { email: { contains: "@example.com" } }) {
    edges { cursor node { id name } }
    pageInfo { hasNextPage endCursor }
  }
}
```

## Development

```bash
# Build the plugin
just build-release

# Build and run the example
just example-build
just example-run      # Run as monolith
just demo             # Run as microservices

# Run tests
just test

# Generate test fixtures
just generate
```

## Design Principles

1. **Proto is the source of truth** - All schema information lives in .proto files
2. **Generate everything** - Database, gRPC, GraphQL, validation from one definition
3. **Type safety end-to-end** - Compile-time guarantees across all layers
4. **Performance by default** - DataLoaders, connection pooling, efficient queries
5. **Escape hatches** - Override generated code via partial override pattern
