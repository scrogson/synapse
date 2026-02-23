#[derive(Debug, Clone)]
pub struct Relation {
    pub name: String,
    pub relation_type: RelationType,
    pub related: String,
    pub foreign_key: String,
    pub references: String,
    pub through: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelationType {
    HasOne,
    HasMany,
    BelongsTo,
    ManyToMany,
}
