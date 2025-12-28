# Synapse Gateway Example

A GraphQL gateway that federates multiple gRPC services into a unified API.

## Architecture

```
                       ┌─────────────┐
                       │   Gateway   │
                       │  (GraphQL)  │
                       │  :4000      │
                       └──────┬──────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
       ┌───────────┐   ┌───────────┐   ┌───────────┐
       │   Blog    │   │   IAM     │   │  (Other)  │
       │  (gRPC)   │   │  (gRPC)   │   │  Services │
       │  :50060   │   │  :50061   │   │           │
       └───────────┘   └───────────┘   └───────────┘
```

## Services

### Blog Service (full-stack example)
- **User** - Blog authors
- **Post** - Blog posts

### IAM Service
- **User** - Identity users
- **Organization** - Organizations
- **Team** - Teams within organizations

## Running

1. Start the database:
```bash
docker-compose -f ../full-stack/docker-compose.yml up -d
```

2. Start the Blog service:
```bash
cd ../full-stack
cargo run
```

3. Start the IAM service:
```bash
cd ../iam
cargo run
```

4. Start the Gateway:
```bash
cargo run
```

5. Open Apollo Sandbox at http://localhost:4000

## GraphQL Schema

The gateway merges queries and mutations from both services:

### Queries
```graphql
# Blog service (users renamed to authors)
author(id: Int!): User
authors(...): UserConnection
post(id: Int!): Post
posts(...): PostConnection

# IAM service
user(id: Int!): User
users(...): UserConnection
organization(id: Int!): Organization
organizations(...): OrganizationConnection
team(id: Int!): Team
teams(...): TeamConnection
```

### Mutations
```graphql
# Blog service (users renamed to authors)
createAuthor(input: CreateUserInput!): User!
updateAuthor(id: Int!, input: UpdateUserInput!): User!
deleteAuthor(id: Int!): Boolean!
createPost(input: CreatePostInput!): Post!
updatePost(id: Int!, input: UpdatePostInput!): Post!
deletePost(id: Int!): Boolean!

# IAM service
createUser(input: CreateUserInput!): User!
updateUser(id: Int!, input: UpdateUserInput!): User!
deleteUser(id: Int!): Boolean!
createOrganization(input: CreateOrganizationInput!): Organization!
updateOrganization(id: Int!, input: UpdateOrganizationInput!): Organization!
deleteOrganization(id: Int!): Boolean!
createTeam(input: CreateTeamInput!): Team!
updateTeam(id: Int!, input: UpdateTeamInput!): Team!
deleteTeam(id: Int!): Boolean!
```

### Handling Name Conflicts

When services have overlapping entity names (like `User` in both Blog and IAM), the gateway resolves conflicts by using domain-appropriate names:

1. **Blog users → authors**: The blog domain calls them "authors" (`author`, `createAuthor`, etc.)
2. **IAM users → users**: The IAM domain keeps the generic "user" name
3. **Type names**: Each service has its own `User` type in different Rust packages

## Configuration

Environment variables:
- `BLOG_GRPC_ENDPOINT` - Blog service address (default: `http://127.0.0.1:50060`)
- `IAM_GRPC_ENDPOINT` - IAM service address (default: `http://127.0.0.1:50061`)
- `GATEWAY_ADDR` - Gateway listen address (default: `0.0.0.0:4000`)

## DataLoaders

The gateway uses DataLoaders for efficient N+1 prevention:

- `BlogUserLoader` - Batch fetch blog authors by ID
- `PostLoader` - Batch fetch posts by ID
- `PostsByUserLoader` - Batch fetch posts by author
- `IamUserLoader` - Batch fetch IAM users by ID
- `OrganizationLoader` - Batch fetch organizations by ID
- `TeamLoader` - Batch fetch teams by ID
- `TeamsByOrganizationLoader` - Batch fetch teams by org
- `UsersByOrganizationLoader` - Batch fetch users by org

## Future: Cross-Service References

A future enhancement would allow entities to reference each other across services:

```protobuf
// In blog.proto
message Post {
  int64 author_id = 3 [(synapse.graphql.field) = {
    foreign_entity: "iam.User"  // Reference IAM user, not blog user
  }];
}
```

This would generate resolvers that use the IAM DataLoader to fetch the author.
