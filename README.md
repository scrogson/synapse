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
  option (synapse.graphql.type) = { node: true };

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
| **TypeScript** | `.d.ts` contracts for custom Deno resolvers |
| **Filters** | Type-safe filter inputs (`UserFilter`, `PostFilter`) with `eq`, `neq`, `gt`, `in`, etc. |
| **Pagination** | Relay-compliant cursor pagination (`edges`, `nodes`, `pageInfo`) |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Proto Definitions                       │
│                   (with synapse.* options)                  │
└─────────────────────────────────────────────────────────────┘
          │                                       │
          ▼                                       ▼
┌───────────────────────┐          ┌──────────────────────────┐
│   synapse-proto-gen   │          │     protoc-gen-synapse    │
│                       │          │                          │
│  Generates auxiliary  │          │  Main protoc plugin      │
│  proto types (filters,│          │  (uses synapse-gen IR)   │
│  connections, etc.)   │          │                          │
└───────────────────────┘          └──────────────────────────┘
                                              │
                          ┌───────────────────┼──────────────────┐
                          ▼                   ▼                  ▼
                   ┌──────────┐        ┌──────────┐       ┌──────────┐
                   │  SeaORM  │        │   gRPC   │       │ GraphQL  │
                   │ Entities │        │ Services │       │ Resolvers│
                   └──────────┘        └──────────┘       └──────────┘
                          │                   │                  │
                          └───────────────────┼──────────────────┘
                                              ▼
                                   ┌──────────────────┐
                                   │  Your Application │
                                   │                  │
                                   │  - gRPC Server   │
                                   │  - GraphQL API   │
                                   │  - PostgreSQL    │
                                   │  - Deno Resolvers│
                                   └──────────────────┘

Crate Responsibilities:

  synapse-gen          IR types, ParsedSchema, CodeGenerator trait
  protoc-gen-synapse   Protoc plugin composing all generators
  synapse-deno         Deno runtime for custom TypeScript resolvers
  synapse-proto-gen    CLI to generate filter/connection proto types
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

### Custom Resolvers (Deno)

Define virtual/computed fields that exist only in GraphQL, resolved at runtime by TypeScript functions in a secure Deno sandbox:

```protobuf
message User {
  option (synapse.graphql.resolver) = {
    deno: { module: "resolvers/user.ts" }
    fields: [
      {
        name: "displayName"
        type: "String!"
        description: "User's display name (name or email)"
      },
      {
        name: "postCount"
        type: "Int!"
        arguments: [{ name: "published", type: "Boolean" }]
      }
    ]
  };
}
```

Synapse generates type-safe `.d.ts` contracts and you implement the resolvers:

```typescript
// resolvers/user.ts
import type { UserResolverModule } from "./generated";

export const displayName: UserResolverModule["displayName"] = (user) => {
  return user.name || user.email.split("@")[0];
};

export const postCount: UserResolverModule["postCount"] = async (user, args, ctx) => {
  const posts = await ctx.dataLoaders.postsByAuthor.load(user.id);
  if (args.published !== undefined) {
    return posts.filter((p) => p.published === args.published).length;
  }
  return posts.length;
};
```

Resolvers work at three levels:
- **Message-level** (`synapse.graphql.resolver`): Virtual/computed fields
- **Field-level** (`synapse.graphql.field_resolver`): Override existing field resolution
- **Method-level** (`synapse.graphql.method_resolver`): Custom RPC implementations

The Deno sandbox uses deny-by-default permissions—network, filesystem, and environment access must be explicitly granted per-resolver.

### Context Injection

Auto-populate request fields from server-side context, preventing client impersonation:

```protobuf
message CreateAuthorRequest {
  // Populated server-side from authenticated user — not exposed in GraphQL input
  int64 user_id = 1 [(synapse.graphql.field).from_context = {
    path: "current_user.id"
    required: true
  }];

  string pen_name = 2;
  optional string bio = 3;
}
```

When `from_context` is set, the field is excluded from the GraphQL input type and injected server-side from the request context. If `required: true`, the request fails with `UNAUTHENTICATED` when the value is missing.

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
  option (synapse.graphql.type) = { node: true };
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
  option (synapse.graphql.type) = { node: true };
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
    option (synapse.graphql.query) = {
      name: "user"
      output_type: "User"
      output_field: "user"
    };
  }

  rpc ListUsers(ListUsersRequest) returns (UserConnection) {
    option (synapse.graphql.query) = {
      name: "users"
    };
  }

  rpc CreateUser(CreateUserRequest) returns (CreateUserResponse) {
    option (synapse.graphql.mutation) = {
      name: "createUser"
      output_type: "User"
      output_field: "user"
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

### `synapse.graphql.type`

```protobuf
option (synapse.graphql.type) = {
  skip: false           // Skip GraphQL generation
  node: true            // Implement Relay Node interface
  name: "User"          // Override GraphQL type name
  input: false          // Generate as InputObject instead
};
```

### `synapse.graphql.field`

```protobuf
int64 author_id = 5 [(synapse.graphql.field) = {
  skip: true            // Hide from GraphQL schema
  name: "authorId"      // Override field name
  deprecated: { reason: "Use author instead" }
  from_context: {       // Populate from request context (see Context Injection)
    path: "current_user.id"
    required: true
  }
}];
```

### `synapse.graphql.query`

```protobuf
rpc GetUser(...) returns (...) {
  option (synapse.graphql.query) = {
    name: "user"              // GraphQL field name on Query type
    output_type: "User"       // Unwrap response to this type
    output_field: "user"      // Field path to extract from response
    skip: false               // Skip this method
  };
}
```

### `synapse.graphql.mutation`

```protobuf
rpc CreateUser(...) returns (...) {
  option (synapse.graphql.mutation) = {
    name: "createUser"        // GraphQL field name on Mutation type
    input_type: "CreateUserInput"  // Override input type name
    output_type: "User"       // Unwrap response to this type
    output_field: "user"      // Field path to extract from response
    skip: false               // Skip this method
  };
}
```

### `synapse.graphql.resolver`

```protobuf
option (synapse.graphql.resolver) = {
  deno: { module: "resolvers/user.ts" }   // Deno module for all virtual fields
  fields: [
    {
      name: "displayName"                 // GraphQL field name
      type: "String!"                     // GraphQL type
      description: "User's display name"  // Field description
      arguments: [                        // Optional field arguments
        { name: "format", type: "String", default_value: "\"short\"" }
      ]
      deno: { function: "customFn" }      // Override function name per-field
    }
  ]
};
```

### `synapse.graphql.field_resolver`

```protobuf
string email = 2 [(synapse.graphql.field_resolver) = {
  deno: {
    module: "resolvers/user.ts"
    function: "maskEmail"     // Function to transform this field's value
  }
}];
```

### `synapse.graphql.method_resolver`

```protobuf
rpc SearchPosts(...) returns (...) {
  option (synapse.graphql.method_resolver) = {
    deno: {
      module: "resolvers/search.ts"
      function: "searchPosts"
      timeout_ms: 10000
      permissions: { net: ["search-api.internal"] }
    }
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
  length: { min: 1, max: 100 }   // String length constraints
  pattern: "^[a-z0-9-]+$"        // Regex pattern match
}];
```

## Building Custom Generators

The `synapse-gen` crate provides the `CodeGenerator` trait for building your own code generators on top of the Synapse IR:

```rust
use synapse_gen::{CodeGenerator, GeneratedFile, GeneratorContext, GeneratorError, SynapseGenerator};
use synapse_gen::ir::Entity;

struct MyGenerator;

impl CodeGenerator for MyGenerator {
    fn name(&self) -> &str { "my-generator" }

    fn generate_entity(
        &self,
        ctx: &GeneratorContext,
        entity: &Entity,
    ) -> Result<Vec<GeneratedFile>, GeneratorError> {
        // Access entity fields, relations, storage options, etc.
        let table = &entity.storage.as_ref().unwrap().table_name;
        Ok(vec![GeneratedFile {
            path: format!("{}/{}.rs", ctx.package.name, entity.name),
            content: format!("// Generated for table: {}", table),
        }])
    }
}

// Compose generators into a protoc plugin
fn main() -> Result<(), Box<dyn std::error::Error>> {
    SynapseGenerator::new()
        .add(MyGenerator)
        .run()
}
```

The trait provides callbacks for each IR element:
- `generate_entity()` — messages with `synapse.storage.entity`
- `generate_service()` — proto services
- `generate_enum()` — proto enums
- `generate_message()` — non-entity messages (request/response types)
- `finalize_package()` — package-level rollup files (e.g., `mod.rs`)
- `finalize()` — top-level files after all packages

## Project Structure

```
synapse/
├── synapse-gen/                # IR framework & CodeGenerator trait
│   └── src/
│       ├── ir/                 # IR types (Entity, Service, Field, etc.)
│       ├── parser/             # ParsedSchema (prost-reflect extension parsing)
│       ├── builder.rs          # SynapseGenerator protoc plugin builder
│       └── generator.rs        # CodeGenerator trait definition
├── protoc-gen-synapse/         # Main protoc plugin
│   └── src/
│       ├── storage/            # SeaORM entity, trait, defaults generation
│       ├── graphql/            # GraphQL schema + resolver generation
│       ├── grpc/               # gRPC service generation
│       ├── typescript/         # TypeScript .d.ts contract generation
│       └── validate/           # Validated domain type generation
├── synapse-deno/               # Deno runtime for custom resolvers
│   └── src/
│       ├── resolver.rs         # DenoResolver, DenoConfig, DenoPermissions
│       └── runtime.rs          # V8 runtime management
├── synapse-proto-gen/          # CLI: generate auxiliary proto types
│   └── src/                    # Generates filters, connections, CRUD messages
├── proto/synapse/              # Synapse proto option definitions
│   ├── storage/options.proto   # Storage layer options
│   ├── graphql/
│   │   ├── options.proto       # GraphQL type/field/query/mutation options
│   │   ├── resolver.proto      # Custom resolver options (Deno)
│   │   └── context.proto       # Context injection definitions
│   ├── grpc/options.proto      # gRPC options
│   ├── validate/options.proto  # Validation options
│   └── relay/types.proto       # Relay pagination types
└── examples/unified/           # Complete working example
```

## Example

See the [`examples/unified`](examples/unified) directory for a complete working example with:

- **Multi-service architecture**: IAM (Users, Organizations, Teams) + Blog (Authors, Posts)
- **Cross-service relations**: Blog Author belongs_to IAM User
- **Custom resolvers**: Virtual fields (displayName) via Deno TypeScript
- **Context injection**: `from_context` for server-side user ID population
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
2. **Generate everything** - Database, gRPC, GraphQL, TypeScript from one definition
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
| Custom resolvers (Deno) | ✅ Complete |
| TypeScript contracts | ✅ Complete |
| Context injection | ✅ Complete |
| Generator framework (`synapse-gen`) | ✅ Complete |
| Elixir backend (Phoenix, Ecto, Absinthe, gRPC) | 🔮 Planned |

## License

MIT
