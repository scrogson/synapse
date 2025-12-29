# Synapse

**Define your data model once in Protocol Buffers. Get type-safe database entities, gRPC services, and GraphQL APIs—all generated.**

Synapse is a code generation framework that turns annotated `.proto` files into a complete, production-ready backend stack.

## What You Get

From a single proto definition like this:

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
  option (synapse.graphql.message) = { node: true };

  int64 id = 1 [(synapse.storage.column).primary_key = true];
  string email = 2 [(synapse.storage.column).unique = true];
  string name = 3;
  optional string bio = 4;
}

service UserService {
  option (synapse.storage.service) = { generate_storage: true };
  option (synapse.graphql.service) = {};
  option (synapse.grpc.service) = {};

  rpc GetUser(GetUserRequest) returns (GetUserResponse);
  rpc ListUsers(ListUsersRequest) returns (UserConnection);
  rpc CreateUser(CreateUserRequest) returns (CreateUserResponse);
}
```

Synapse generates:

| Layer | What's Generated |
|-------|------------------|
| **Database** | SeaORM entities, migrations, relation definitions |
| **gRPC** | Tonic service traits, request/response types, server implementations |
| **GraphQL** | async-graphql types, Query/Mutation resolvers, Relay connections, DataLoaders |
| **Filters** | Type-safe filter inputs (`UserFilter`, `PostFilter`) with `eq`, `neq`, `gt`, `in`, etc. |
| **Pagination** | Relay-compliant cursor pagination (`edges`, `nodes`, `pageInfo`) |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Proto Definitions                       │
│                   (with synapse.* options)                  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    protoc-gen-synapse                       │
│                                                             │
│  Parses proto files, reads synapse.* extensions,            │
│  generates Rust code for all layers                         │
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
                    │                  │
                    │  - gRPC Server   │
                    │  - GraphQL API   │
                    │  - PostgreSQL    │
                    └──────────────────┘
```

## Key Features

### N+1 Prevention with DataLoaders

GraphQL relation resolvers use DataLoaders for efficient batching:

```graphql
# This query...
{
  users {
    edges {
      node {
        posts { id title }  # DataLoader batches all user IDs
      }
    }
  }
}
```

```sql
-- ...generates only 2 queries, not N+1
SELECT * FROM users;
SELECT * FROM posts WHERE author_id IN (1, 2, 3, 4, 5, 6);
```

### Dual Access Patterns for Relations

HasMany relations provide both patterns:

```graphql
type User {
  # DataLoader-backed array (efficient for batch loading)
  posts: [Post!]!

  # Paginated connection (cursor pagination per-user)
  postsCollection(first: Int, after: String): PostConnection!
}
```

### Relay-Compliant Pagination

All list endpoints return Relay connections with cursor pagination:

```graphql
{
  users(first: 10, after: "cursor123") {
    edges {
      cursor
      node { id name }
    }
    pageInfo {
      hasNextPage
      endCursor
    }
  }
}
```

### Type-Safe Filters

Auto-generated filter types for every entity:

```graphql
{
  users(filter: {
    email: { contains: "@example.com" }
    createdAt: { gte: "2024-01-01" }
  }) {
    edges { node { id } }
  }
}
```

### Relay Node Interface

Entities marked with `node: true` implement the Relay Node interface:

```graphql
{
  node(id: "VXNlcjox") {
    ... on User {
      name
      email
    }
  }
}
```

### Validated Domain Types

Request messages with validation annotations generate domain types with `TryFrom` validation:

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

Generated code validates on conversion:

```rust
// Proto type -> Validated domain type
let create_user: CreateUser = request.try_into()?;

// Validation errors have structured fields
// CreateUserValidationError { errors: [CreateUserFieldError { code, message, field }] }
```

### Partial Override Pattern

Storage traits support partial overrides—override specific methods while using generated defaults for others:

```rust
impl UserServiceStorage for MyCustomStorage {
    fn db(&self) -> &DatabaseConnection { &self.db }

    // Override just create_user with custom business logic
    async fn create_user(&self, request: CreateUser) -> Result<CreateUserResponse, StorageError> {
        // Custom pre-processing
        log::info!("Creating user: {}", request.email);

        // Delegate to generated default
        user_service_storage_defaults::create_user(self.db(), request).await
    }

    // All other methods use trait defaults automatically
}
```

## Quick Start

### 1. Define Your Schema

Create your proto files with Synapse annotations:

```protobuf
// blog.entities.proto
syntax = "proto3";
package blog;

import "synapse/storage/options.proto";
import "synapse/graphql/options.proto";

message User {
  option (synapse.graphql.message) = { node: true };
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
  string name = 3;
  optional string bio = 4;
}

message Post {
  option (synapse.graphql.message) = { node: true };
  option (synapse.storage.entity) = {
    table_name: "posts"
    relations: [{
      name: "author"
      type: RELATION_TYPE_BELONGS_TO
      related: "User"
      foreign_key: "author_id"
    }]
  };

  int64 id = 1 [(synapse.storage.column).primary_key = true];
  string title = 2;
  string content = 3;
  bool published = 4;
  int64 author_id = 5 [(synapse.graphql.field).skip = true];
}
```

### 2. Define Your Services

```protobuf
// blog.proto
syntax = "proto3";
package blog;

import "blog.entities.proto";
import "synapse/graphql/options.proto";
import "synapse/grpc/options.proto";
import "synapse/storage/options.proto";

service UserService {
  option (synapse.graphql.service) = {};
  option (synapse.grpc.service) = {};
  option (synapse.storage.service) = {
    generate_storage: true
    generate_implementation: true
  };

  rpc GetUser(GetUserRequest) returns (GetUserResponse) {
    option (synapse.graphql.method) = {
      name: "user"
      operation: "Query"
    };
  }

  rpc ListUsers(ListUsersRequest) returns (UserConnection) {
    option (synapse.graphql.method) = {
      name: "users"
      operation: "Query"
    };
  }

  rpc CreateUser(CreateUserRequest) returns (CreateUserResponse) {
    option (synapse.graphql.method) = {
      name: "createUser"
      operation: "Mutation"
    };
  }
}
```

### 3. Generate Code

Add to your `build.rs`:

```rust
fn main() {
    // Run protoc-gen-synapse
    let status = std::process::Command::new("buf")
        .args(["generate"])
        .status()
        .expect("Failed to run buf generate");

    if !status.success() {
        panic!("Code generation failed");
    }
}
```

### 4. Wire Up Your Application

```rust
use generated::blog::{
    user_service_server::UserServiceServer,
    user_service_client::UserServiceClient,
    SeaOrmUserServiceStorage,
    UserServiceGrpcService,
    graphql::{build_schema, AppSchema},
};

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to database
    let db = Database::connect("postgres://...").await?;

    // Create storage and gRPC service
    let storage = SeaOrmUserServiceStorage::new(db);
    let grpc_service = UserServiceGrpcService::new(storage);

    // Start gRPC server
    let grpc_server = TonicServer::builder()
        .add_service(UserServiceServer::new(grpc_service))
        .serve(grpc_addr);
    tokio::spawn(grpc_server);

    // Create gRPC client for GraphQL
    let channel = Channel::from_static("http://localhost:50051").connect().await?;
    let user_client = UserServiceClient::new(channel);

    // Build GraphQL schema with DataLoaders
    let schema = build_schema(user_client);

    // Start GraphQL server
    let app = Router::new()
        .route("/graphql", post(graphql_handler))
        .with_state(schema);

    axum::serve(listener, app).await?;
    Ok(())
}
```

## Proto Options Reference

### `synapse.storage.entity`

```protobuf
option (synapse.storage.entity) = {
  table_name: "users"           // Database table name
  skip: false                   // Skip generation for this entity
  relations: [...]              // Relation definitions
};
```

### `synapse.storage.column`

```protobuf
int64 id = 1 [(synapse.storage.column) = {
  primary_key: true             // Mark as primary key
  auto_increment: true          // Auto-increment (default for PKs)
  unique: true                  // Add unique constraint
  column_name: "user_id"        // Override column name
  default_expr: "Expr::..."     // SeaORM default expression
}];
```

### `synapse.storage.entity.relations`

```protobuf
relations: [
  {
    name: "posts"                      // Field name in GraphQL
    type: RELATION_TYPE_HAS_MANY       // HAS_MANY, BELONGS_TO, HAS_ONE
    related: "Post"                    // Related entity name
    foreign_key: "author_id"           // Foreign key column
    references: "id"                   // Referenced column (for BELONGS_TO)
  }
]
```

### `synapse.graphql.message`

```protobuf
option (synapse.graphql.message) = {
  skip: false           // Skip GraphQL generation
  node: true            // Implement Relay Node interface
  type_name: "User"     // Override GraphQL type name
  input_type: false     // Generate as InputObject instead
};
```

### `synapse.graphql.field`

```protobuf
int64 author_id = 5 [(synapse.graphql.field) = {
  skip: true            // Hide from GraphQL schema
  name: "authorId"      // Override field name
  deprecated: { reason: "Use author instead" }
}];
```

### `synapse.graphql.method`

```protobuf
rpc GetUser(...) returns (...) {
  option (synapse.graphql.method) = {
    name: "user"              // GraphQL field name
    operation: "Query"        // Query, Mutation, or Subscription
    description: "Get user"   // Field description
    skip: false               // Skip this method
  };
}
```

### `synapse.storage.service`

```protobuf
option (synapse.storage.service) = {
  generate_storage: true          // Generate storage trait
  generate_implementation: true   // Generate SeaORM implementation
};
```

### `synapse.validate.message`

```protobuf
option (synapse.validate.message) = {
  generate_conversion: true       // Generate TryFrom<Proto> for domain type
  name: "CreateUser"              // Name of the generated domain type
};
```

### `synapse.validate.field`

```protobuf
string email = 1 [(synapse.validate.field).rules = {
  required: true                  // Field must be non-empty (strings) or Some (optionals)
  email: true                     // Must contain @ (basic email check)
  length: { min: 1, max: 100 }    // String length constraints
  pattern: "^[a-z0-9-]+$"         // Regex pattern match
}];
```

## Project Structure

```
your-project/
├── proto/
│   ├── iam/
│   │   ├── entities.proto       # IAM entity definitions (User, Org, Team)
│   │   └── services.proto       # IAM services and request/response types
│   └── blog/
│       ├── entities.proto       # Blog entity definitions (Author, Post)
│       └── services.proto       # Blog services and request/response types
├── src/
│   ├── generated/               # Generated Rust code
│   │   ├── iam/
│   │   │   ├── mod.rs           # Proto types + re-exports
│   │   │   ├── entities/        # SeaORM entity models
│   │   │   ├── storage/         # Storage traits, defaults, SeaORM impl
│   │   │   ├── grpc/            # gRPC service implementations
│   │   │   ├── graphql/         # GraphQL types, resolvers, DataLoaders
│   │   │   ├── create_user.rs   # Validated domain type
│   │   │   └── update_user.rs   # Validated domain type
│   │   └── blog/
│   │       └── ...              # Same structure
│   └── main.rs
├── build.rs
└── Cargo.toml
```

## Example

See the [`examples/unified`](examples/unified) directory for a complete working example with:

- **Multi-service architecture**: IAM (Users, Organizations, Teams) + Blog (Authors, Posts)
- **Cross-service relations**: Blog Author belongs_to IAM User
- **Validated domain types**: Request validation with `TryFrom` conversions
- **Partial override pattern**: Override specific storage methods while using defaults for others
- **Multiple deployment modes**: Monolith, microservices, or gateway-only
- PostgreSQL database with SeaORM 2.0
- gRPC APIs (Tonic)
- GraphQL API (async-graphql + Axum)
- Relay-style pagination and filtering

```bash
# Start PostgreSQL
just db-up

# Run as monolith (all services in one process)
just example-run

# Or run as microservices
just demo  # Starts IAM, Blog, and Gateway separately

# GraphQL: http://localhost:4000
# IAM gRPC: localhost:50051
# Blog gRPC: localhost:50052
```

## Design Principles

1. **Proto is the source of truth** - All schema information lives in `.proto` files
2. **Generate everything** - Database, gRPC, GraphQL from one definition
3. **Type safety end-to-end** - Compile-time guarantees across all layers
4. **Performance by default** - DataLoaders, connection pooling, efficient queries
5. **Relay compliance** - Cursor pagination, Node interface, global IDs
6. **Escape hatches** - Override generated code when needed

## Status

| Component | Status |
|-----------|--------|
| SeaORM entities | ✅ Complete |
| gRPC services | ✅ Complete |
| GraphQL types | ✅ Complete |
| Relay connections | ✅ Complete |
| DataLoaders | ✅ Complete |
| Filters & ordering | ✅ Complete |
| Validated domain types | ✅ Complete |
| Partial override pattern | ✅ Complete |
| Cross-package relations | ✅ Complete |
| Elixir backend (Phoenix, Ecto, Absinthe, gRPC) | 🔮 Planned |

## License

MIT
