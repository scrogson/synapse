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

## Project Structure

```
your-project/
├── proto/
│   ├── blog.entities.proto      # Entity definitions
│   ├── blog.proto               # Services and request/response types
│   └── blog.synapse.proto       # Auto-generated (filters, connections)
├── src/
│   ├── generated/               # Generated Rust code
│   │   └── blog/
│   │       ├── mod.rs           # Proto types + gRPC
│   │       ├── entities/        # SeaORM entities
│   │       ├── storage/         # Storage traits + implementations
│   │       └── graphql/         # GraphQL types + resolvers
│   └── main.rs
├── build.rs
└── Cargo.toml
```

## Examples

See the [`examples/full-stack`](examples/full-stack) directory for a complete working example with:

- PostgreSQL database
- gRPC API (Tonic)
- GraphQL API (async-graphql + Axum)
- Apollo Sandbox UI

```bash
cd examples/full-stack
docker-compose up -d   # Start PostgreSQL
cargo run              # Start servers

# GraphQL: http://localhost:8080
# gRPC: localhost:50060
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
| Ecto backend | 🔮 Planned |
| GORM backend | 🔮 Planned |

## License

MIT
