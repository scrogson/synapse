// Simple user resolver for testing

/**
 * Compute the display name for a user
 * @param {Object} user - The user object
 * @param {Object} args - Arguments (unused)
 * @param {Object} ctx - Context (unused)
 * @returns {string} The display name
 */
export function displayName(user, args, ctx) {
  return user.name || user.email;
}

/**
 * Compute the full name for a user
 * @param {Object} user - The user object
 * @param {Object} args - Arguments (unused)
 * @param {Object} ctx - Context (unused)
 * @returns {string} The full name
 */
export function fullName(user, args, ctx) {
  if (user.firstName && user.lastName) {
    return `${user.firstName} ${user.lastName}`;
  }
  return user.name || "Unknown";
}

/**
 * Async resolver that returns user initials
 * @param {Object} user - The user object
 * @param {Object} args - Arguments (unused)
 * @param {Object} ctx - Context (unused)
 * @returns {Promise<string>} The user's initials
 */
export async function initials(user, args, ctx) {
  // Async operation (Promise.resolve is sufficient for testing async handling)
  await Promise.resolve();

  const name = user.name || "";
  const parts = name.split(" ");
  return parts.map(p => p[0]).join("").toUpperCase();
}

/**
 * Root resolver for fetching a greeting
 * @param {Object} args - The request arguments
 * @param {Object} ctx - Context
 * @returns {string} A greeting message
 */
export function greeting(args, ctx) {
  return `Hello, ${args.name || "World"}!`;
}
