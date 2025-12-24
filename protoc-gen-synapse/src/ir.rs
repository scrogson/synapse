//! Intermediate Representation (IR) for code generation
//!
//! The IR is a backend-agnostic representation of the proto schema
//! that can be transformed into code for any target language/ORM.

/// A database entity (table/model)
#[derive(Debug, Clone)]
pub struct Entity {
    /// Entity name (from proto message name)
    pub name: String,

    /// Database table name
    pub table_name: String,

    /// Package/module path
    pub package: String,

    /// Fields/columns
    pub fields: Vec<Field>,

    /// Relations to other entities
    pub relations: Vec<Relation>,
}

/// A field/column in an entity
#[derive(Debug, Clone)]
pub struct Field {
    /// Field name (snake_case for most languages)
    pub name: String,

    /// Original proto field name
    pub proto_name: String,

    /// Field type
    pub field_type: FieldType,

    /// Whether the field is nullable (from proto `optional`)
    pub nullable: bool,

    /// Primary key
    pub primary_key: bool,

    /// Auto-increment
    pub auto_increment: bool,

    /// Unique constraint
    pub unique: bool,

    /// Custom column name override
    pub column_name: Option<String>,

    /// Default value expression
    pub default_value: Option<String>,

    /// Embed as JSON
    pub embed: bool,

    /// Backend-specific type hints
    pub type_hints: std::collections::HashMap<String, String>,
}

/// Field types
#[derive(Debug, Clone)]
pub enum FieldType {
    Bool,
    Int32,
    Int64,
    Uint32,
    Uint64,
    Float,
    Double,
    String,
    Bytes,
    Timestamp,
    Message(String),
    Enum(String),
}

/// A relation between entities
#[derive(Debug, Clone)]
pub struct Relation {
    /// Relation name (for generated code)
    pub name: String,

    /// Type of relation
    pub relation_type: RelationType,

    /// Related entity name
    pub related_entity: String,

    /// Foreign key column
    pub foreign_key: String,

    /// Referenced column (usually "id")
    pub references: String,

    /// Through table (for many-to-many)
    pub through: Option<String>,
}

/// Relation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationType {
    BelongsTo,
    HasOne,
    HasMany,
    ManyToMany,
}

/// An enum definition
#[derive(Debug, Clone)]
pub struct Enum {
    /// Enum name
    pub name: String,

    /// Package/module path
    pub package: String,

    /// Variants
    pub variants: Vec<EnumVariant>,

    /// How to store in database
    pub db_type: EnumDbType,
}

/// An enum variant
#[derive(Debug, Clone)]
pub struct EnumVariant {
    /// Variant name
    pub name: String,

    /// Proto number value
    pub number: i32,

    /// Custom string value for database
    pub string_value: Option<String>,

    /// Custom integer value for database
    pub int_value: Option<i32>,
}

/// How to store enum in database
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnumDbType {
    #[default]
    String,
    Integer,
}

/// A service method (for storage trait generation)
#[derive(Debug, Clone)]
pub struct Method {
    /// Method name
    pub name: String,

    /// Input type
    pub input_type: String,

    /// Output type
    pub output_type: String,
}

/// A complete proto file's IR
#[derive(Debug, Clone, Default)]
pub struct FileIr {
    /// Package name
    pub package: String,

    /// Entities to generate
    pub entities: Vec<Entity>,

    /// Enums to generate
    pub enums: Vec<Enum>,

    /// Services (for storage trait generation)
    pub services: Vec<Service>,
}

/// A service definition
#[derive(Debug, Clone)]
pub struct Service {
    /// Service name
    pub name: String,

    /// Generated trait name
    pub trait_name: String,

    /// Methods
    pub methods: Vec<Method>,
}
