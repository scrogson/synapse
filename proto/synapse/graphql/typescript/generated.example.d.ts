/**
 * Generated Resolver Contracts
 *
 * This file is AUTO-GENERATED from proto definitions.
 * DO NOT EDIT MANUALLY.
 *
 * It defines type-safe interfaces for custom resolvers.
 * Implement these interfaces in your resolver modules.
 */

import type {
  FieldResolver,
  RootResolver,
  ResolverContext,
  DataLoader,
  Connection,
  Nullable,
  RequestInfo,
} from "@synapse/runtime";

// =============================================================================
// Generated Types from Proto Messages
// =============================================================================

// iam/entities.proto

export interface User {
  id: number;
  email: string;
  name: string;
  avatarUrl: Nullable<string>;
  organizationId: Nullable<number>;
  isActive: boolean;
  createdAt: string; // ISO 8601 timestamp
  updatedAt: string;
}

export interface Organization {
  id: number;
  name: string;
  slug: string;
  description: Nullable<string>;
  createdAt: string;
  updatedAt: string;
}

export interface Team {
  id: number;
  name: string;
  slug: string;
  description: Nullable<string>;
  organizationId: number;
  createdAt: string;
  updatedAt: string;
}

// blog/entities.proto

export interface Author {
  id: number;
  userId: number;
  penName: string;
  bio: Nullable<string>;
  createdAt: string;
  updatedAt: string;
}

export interface Post {
  id: number;
  title: string;
  content: string;
  published: boolean;
  authorId: number;
  createdAt: string;
  updatedAt: string;
}

// =============================================================================
// Application-Defined Context
// =============================================================================

/**
 * Your application's authenticated user type.
 * Define this based on your auth system (JWT claims, session, etc.)
 *
 * This is NOT defined by Synapse - you define it in your application.
 */
export interface CurrentUser {
  id: number;
  email: string;
  name: string;
  // Add your own fields: roles, permissions, tenant_id, etc.
}

// =============================================================================
// Generated DataLoaders Interface
// =============================================================================

export interface GeneratedDataLoaders {
  // IAM loaders
  userById: DataLoader<number, User | null>;
  organizationById: DataLoader<number, Organization | null>;
  teamById: DataLoader<number, Team | null>;
  usersByOrganization: DataLoader<number, User[]>;
  teamsByOrganization: DataLoader<number, Team[]>;

  // Blog loaders
  authorById: DataLoader<number, Author | null>;
  postById: DataLoader<number, Post | null>;
  authorByUserId: DataLoader<number, Author | null>;
  postsByAuthor: DataLoader<number, Post[]>;
}

/**
 * Application's resolver context.
 *
 * Extends the base ResolverContext with:
 * - Your CurrentUser type
 * - Generated DataLoaders
 * - Any app-specific context (tenant, feature flags, etc.)
 */
export interface AppResolverContext extends ResolverContext<CurrentUser> {
  dataLoaders: GeneratedDataLoaders;
  // Add app-specific context here:
  // tenant?: Tenant;
  // featureFlags?: FeatureFlags;
}

// =============================================================================
// Virtual Field Resolver Contracts
// =============================================================================

/**
 * Virtual field resolvers for User type.
 *
 * Defined by:
 *   option (synapse.graphql.resolver) = {
 *     fields: [
 *       { name: "fullName", type: "String!" },
 *       { name: "postCount", type: "Int!", arguments: [...] }
 *     ]
 *   };
 *
 * Implement in: resolvers/user.ts
 */
export interface UserVirtualFields {
  /**
   * User's full name (computed from name parts or profile).
   * @returns The user's display name
   */
  fullName: FieldResolver<User, {}, string, AppResolverContext>;

  /**
   * Count of posts authored by this user.
   * @param args.published - Filter by published status (default: true)
   * @returns Number of posts
   */
  postCount: FieldResolver<
    User,
    { published?: boolean },
    number,
    AppResolverContext
  >;
}

/**
 * Virtual field resolvers for Author type.
 */
export interface AuthorVirtualFields {
  /**
   * Reading time estimate for all posts by this author.
   * @returns Formatted reading time (e.g., "5 min read")
   */
  totalReadingTime: FieldResolver<Author, {}, string, AppResolverContext>;

  /**
   * Author's popularity score based on post engagement.
   */
  popularityScore: FieldResolver<Author, {}, number, AppResolverContext>;
}

/**
 * Virtual field resolvers for Post type.
 */
export interface PostVirtualFields {
  /**
   * Estimated reading time for the post.
   */
  readingTime: FieldResolver<Post, {}, string, AppResolverContext>;

  /**
   * Post excerpt (first N characters of content).
   * @param args.length - Maximum length (default: 200)
   */
  excerpt: FieldResolver<Post, { length?: number }, string, AppResolverContext>;

  /**
   * Word count of post content.
   */
  wordCount: FieldResolver<Post, {}, number, AppResolverContext>;
}

// =============================================================================
// Field Override Resolver Contracts
// =============================================================================

/**
 * Field override resolvers for User type.
 *
 * Use these to transform existing proto field values in GraphQL.
 * The resolver receives the original field value on the parent object.
 */
export interface UserFieldOverrides {
  /**
   * Override email field to mask domain for privacy.
   *
   * Defined by:
   *   string email = 2 [(synapse.graphql.field_resolver) = {
   *     deno: { function: "maskEmail" }
   *   }];
   */
  maskEmail: FieldResolver<User, {}, string, AppResolverContext>;
}

// =============================================================================
// RPC/Method Resolver Contracts
// =============================================================================

/**
 * Custom RPC resolvers for UserService.
 *
 * Use these for custom Query/Mutation implementations
 * instead of generated gRPC calls.
 */
export interface UserServiceResolvers {
  /**
   * Custom metrics endpoint with analytics integration.
   *
   * Defined by:
   *   rpc GetUserMetrics(...) returns (...) {
   *     option (synapse.graphql.method_resolver) = { ... };
   *   };
   */
  getUserMetrics: RootResolver<
    { userId: number; dateRange?: DateRangeInput },
    UserMetrics,
    AppResolverContext
  >;
}

export interface BlogServiceResolvers {
  /**
   * Full-text search across posts with custom ranking.
   */
  searchPosts: RootResolver<
    { query: string; first?: number; after?: string },
    Connection<Post>,
    AppResolverContext
  >;
}

// =============================================================================
// Input Types (for RPC resolvers)
// =============================================================================

export interface DateRangeInput {
  start: string;
  end: string;
}

export interface UserMetrics {
  userId: number;
  postCount: number;
  totalViews: number;
  avgReadTime: number;
  topPosts: Post[];
}

// =============================================================================
// Module Export Contract
// =============================================================================

/**
 * Expected exports from resolvers/user.ts
 *
 * The module must export functions matching the resolver contracts.
 * Names must match the field/method names in camelCase.
 */
export interface UserResolverModule {
  // Virtual fields
  fullName: UserVirtualFields["fullName"];
  postCount: UserVirtualFields["postCount"];

  // Field overrides
  maskEmail?: UserFieldOverrides["maskEmail"];
}

/**
 * Expected exports from resolvers/author.ts
 */
export interface AuthorResolverModule {
  totalReadingTime: AuthorVirtualFields["totalReadingTime"];
  popularityScore: AuthorVirtualFields["popularityScore"];
}

/**
 * Expected exports from resolvers/post.ts
 */
export interface PostResolverModule {
  readingTime: PostVirtualFields["readingTime"];
  excerpt: PostVirtualFields["excerpt"];
  wordCount: PostVirtualFields["wordCount"];
}
