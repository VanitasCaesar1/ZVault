/**
 * Clerk configuration for ZVault Cloud dashboard.
 *
 * Set `VITE_CLERK_PUBLISHABLE_KEY` in your `.env.local` or `.env`.
 * Get it from https://dashboard.clerk.com → API Keys → Publishable Key.
 */

export const CLERK_PUBLISHABLE_KEY =
  import.meta.env.VITE_CLERK_PUBLISHABLE_KEY ?? "";

/** Whether Clerk cloud auth is configured. */
export const CLERK_ENABLED = CLERK_PUBLISHABLE_KEY.length > 0;
