/**
 * Synapse GraphQL Resolver Runtime Types
 *
 * Base types for custom Deno resolvers. These are provided by Synapse
 * and imported by generated resolver contracts.
 */

// =============================================================================
// Core Resolver Types
// =============================================================================

/**
 * Generic field resolver function signature.
 *
 * @typeParam Parent - The parent object type (e.g., User for user.fullName)
 * @typeParam Args - Arguments passed to the field (empty object {} if none)
 * @typeParam Result - The return type of the field
 * @typeParam Context - The resolver context type (defaults to ResolverContext)
 */
export type FieldResolver<
  Parent,
  Args extends Record<string, unknown>,
  Result,
  Context = ResolverContext
> = (
  parent: Parent,
  args: Args,
  ctx: Context
) => Result | Promise<Result>;

/**
 * Root resolver function signature (for Query/Mutation fields).
 * Same as FieldResolver but Parent is always undefined.
 */
export type RootResolver<
  Args extends Record<string, unknown>,
  Result,
  Context = ResolverContext
> = FieldResolver<undefined, Args, Result, Context>;

// =============================================================================
// Context Types
// =============================================================================

/**
 * DataLoader interface for batched data fetching.
 * Prevents N+1 queries in GraphQL resolvers.
 */
export interface DataLoader<K, V> {
  /** Load a single value by key */
  load(key: K): Promise<V>;
  /** Load multiple values by keys */
  loadMany(keys: K[]): Promise<(V | Error)[]>;
  /** Clear a specific key from the cache */
  clear(key: K): this;
  /** Clear all cached values */
  clearAll(): this;
  /** Prime the cache with a value */
  prime(key: K, value: V): this;
}

/**
 * Available DataLoaders, keyed by entity type.
 * Generated based on proto message definitions.
 */
export interface DataLoaders {
  [key: string]: DataLoader<unknown, unknown>;
}

/**
 * Request metadata available in resolver context.
 */
export interface RequestInfo {
  /** Distributed tracing ID */
  traceId: string;
  /** Unique request ID */
  requestId: string;
  /** Client IP address */
  ip?: string;
  /** User agent string */
  userAgent?: string;
  /** Selected request headers */
  headers: Record<string, string>;
}

/**
 * Base resolver context passed to all resolver functions.
 *
 * Applications extend this with their own context types:
 *
 * @example
 * ```typescript
 * interface AppContext extends ResolverContext<MyUser> {
 *   tenant: Tenant;
 *   featureFlags: FeatureFlags;
 * }
 * ```
 *
 * @typeParam User - Your application's user type (or `unknown` if untyped)
 */
export interface ResolverContext<User = unknown> {
  /** The authenticated user (null if unauthenticated) */
  currentUser: User | null;

  /** DataLoaders for efficient batched loading */
  dataLoaders: DataLoaders;

  /** Request metadata */
  request: RequestInfo;
}

// =============================================================================
// Relay Types
// =============================================================================

/**
 * Relay-style PageInfo for cursor pagination.
 */
export interface PageInfo {
  hasNextPage: boolean;
  hasPreviousPage: boolean;
  startCursor?: string;
  endCursor?: string;
}

/**
 * Relay-style Edge containing a node and its cursor.
 */
export interface Edge<T> {
  cursor: string;
  node: T;
}

/**
 * Relay-style Connection for paginated results.
 */
export interface Connection<T> {
  edges: Edge<T>[];
  pageInfo: PageInfo;
  totalCount?: number;
}

// =============================================================================
// Utility Types
// =============================================================================

/**
 * Make all properties in T optional recursively.
 */
export type DeepPartial<T> = {
  [P in keyof T]?: T[P] extends object ? DeepPartial<T[P]> : T[P];
};

/**
 * Extract the element type from an array type.
 */
export type ElementOf<T> = T extends (infer E)[] ? E : never;

/**
 * Helper type for nullable fields (proto optional).
 */
export type Nullable<T> = T | null | undefined;
