# Synapse

A federated, protobuf-centric data platform.

## Vision

Synapse provides a unified schema definition layer using Protocol Buffers that generates type-safe code across multiple languages and storage backends. Define your data model once in proto, get consistent APIs everywhere.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Proto Definitions                        │
│                                                              │
│  message User {                                              │
│    int64 id = 1 [(synapse.storage.column).primary_key = true];│
│    string email = 2 [(synapse.storage.column).unique = true];│
│  }                                                           │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    protoc-gen-synapse                        │
│                                                              │
│  Parses proto + synapse.* options, builds abstract IR        │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
        ┌──────────┐   ┌──────────┐   ┌──────────┐
        │  SeaORM  │   │   Ecto   │   │   GORM   │
        │ Backend  │   │ Backend  │   │ Backend  │
        └──────────┘   └──────────┘   └──────────┘
              │               │               │
              ▼               ▼               ▼
        ┌──────────┐   ┌──────────┐   ┌──────────┐
        │   Rust   │   │  Elixir  │   │    Go    │
        │ Entities │   │ Schemas  │   │  Models  │
        └──────────┘   └──────────┘   └──────────┘
```

## Namespaces

### `synapse.storage`

Database/persistence layer options. Backend-agnostic annotations for entities, columns, and relations.

```protobuf
import "synapse/storage/options.proto";

message User {
  option (synapse.storage.entity) = {
    table_name: "users"
  };

  int64 id = 1 [(synapse.storage.column) = {
    primary_key: true
  }];

  string email = 2 [(synapse.storage.column) = {
    unique: true
  }];

  optional string bio = 3;  // nullable in DB

  repeated Post posts = 4 [(synapse.storage.relation) = {
    type: HAS_MANY
    foreign_key: "author_id"
  }];
}
```

### Future Namespaces

- `synapse.api` - API layer (REST/GraphQL generation)
- `synapse.event` - Event sourcing / message bus
- `synapse.cache` - Caching layer annotations
- `synapse.search` - Search index definitions

## Storage Backends

| Backend | Language | Status |
|---------|----------|--------|
| SeaORM | Rust | In Progress (see protoc-gen-seaorm) |
| Ecto | Elixir | Planned |
| GORM | Go | Planned |
| Diesel | Rust | Planned |
| SQLAlchemy | Python | Planned |

## Core Concepts

### Entity Options (`synapse.storage.entity`)

```protobuf
message EntityOptions {
  // Database table name (defaults to snake_case of message name)
  string table_name = 1;

  // Skip code generation for this message
  bool skip = 2;

  // Relation definitions (message-level)
  repeated RelationDef relations = 3;
}
```

### Column Options (`synapse.storage.column`)

```protobuf
message ColumnOptions {
  // Primary key
  bool primary_key = 1;

  // Auto-increment (defaults true for primary keys)
  bool auto_increment = 2;

  // Unique constraint
  bool unique = 3;

  // Override column name
  string column_name = 4;

  // Default value expression
  string default_value = 5;

  // Backend-specific type hints
  map<string, string> type_hints = 10;
}
```

### Relation Options (`synapse.storage.relation`)

```protobuf
enum RelationType {
  RELATION_TYPE_UNSPECIFIED = 0;
  BELONGS_TO = 1;
  HAS_ONE = 2;
  HAS_MANY = 3;
  MANY_TO_MANY = 4;
}

message RelationDef {
  RelationType type = 1;
  string foreign_key = 2;
  string references = 3;
  string through = 4;  // For many-to-many
}
```

## Design Principles

1. **Proto is the source of truth** - All schema information lives in .proto files
2. **Backend-agnostic by default** - Core options work across all backends
3. **Escape hatches for specifics** - `type_hints` map for backend-specific needs
4. **Convention over configuration** - Sensible defaults (snake_case tables, auto_increment PKs)
5. **Nullable from proto** - Use `optional` keyword, not custom options

## Project Structure

```
synapse/
├── proto/
│   └── synapse/
│       ├── storage/
│       │   └── options.proto    # Storage layer options
│       ├── api/
│       │   └── options.proto    # API layer options (future)
│       └── event/
│           └── options.proto    # Event layer options (future)
├── protoc-gen-synapse/          # Main protoc plugin (Rust)
│   ├── src/
│   │   ├── ir/                  # Intermediate representation
│   │   ├── backends/
│   │   │   ├── mod.rs
│   │   │   ├── seaorm.rs
│   │   │   ├── ecto.rs
│   │   │   └── gorm.rs
│   │   └── main.rs
│   └── Cargo.toml
└── examples/
    ├── rust-seaorm/
    ├── elixir-ecto/
    └── go-gorm/
```

## Migration from protoc-gen-seaorm

The existing `protoc-gen-seaorm` project will be refactored:

1. Rename proto options: `seaorm.*` → `synapse.storage.*`
2. Extract backend-agnostic IR (intermediate representation)
3. Move SeaORM-specific code to `backends/seaorm.rs`
4. Add new backends (Ecto, GORM, etc.)

## Getting Started

```bash
# Install the plugin
cargo install protoc-gen-synapse

# Generate Rust/SeaORM code
protoc --synapse_out=backend=seaorm:./gen proto/*.proto

# Generate Elixir/Ecto code
protoc --synapse_out=backend=ecto:./gen proto/*.proto
```

## Development

```bash
# Build
cargo build

# Test
cargo test

# Run with specific backend
echo "..." | protoc-gen-synapse --backend=seaorm
```
