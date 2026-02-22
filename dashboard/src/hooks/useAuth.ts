/**
 * Unified auth hook that works in both vault-token and Clerk modes.
 *
 * When Clerk is configured (`CLERK_ENABLED`), the app is wrapped in
 * `ClerkProvider` so Clerk hooks are safe to call. When it's not
 * configured, this hook returns a stub so callers don't need conditionals.
 */

import { useUser, useClerk, useAuth as useClerkAuth } from "@clerk/clerk-react";
import { CLERK_ENABLED } from "../lib/clerk";

export interface AuthState {
  /** True when Clerk is configured and the user has an active session. */
  isCloudAuthenticated: boolean;
  /** True while Clerk SDK is initializing. */
  isCloudLoading: boolean;
  /** Cloud user profile (if authenticated). */
  user?: { name?: string; email?: string; picture?: string };
  /** Redirect to Clerk sign-in. */
  signIn: () => void;
  /** Get a session token for API calls. */
  getToken: () => Promise<string | null>;
  /** Sign out of Clerk. */
  signOut: () => Promise<void>;
}

const STUB: AuthState = {
  isCloudAuthenticated: false,
  isCloudLoading: false,
  user: undefined,
  signIn: () => {},
  getToken: () => Promise.resolve(null),
  signOut: () => Promise.resolve(),
};

/**
 * Use this hook in components that need auth state.
 *
 * When `CLERK_ENABLED` is false the app is NOT wrapped in `ClerkProvider`,
 * so we must not call Clerk hooks. We return a static stub instead.
 */
export function useAuth(): AuthState {
  if (!CLERK_ENABLED) {
    return STUB;
  }

  // Safe to call — ClerkProvider is present when CLERK_ENABLED is true.
  // eslint-disable-next-line react-hooks/rules-of-hooks
  const { isSignedIn, isLoaded, user } = useUser();
  // eslint-disable-next-line react-hooks/rules-of-hooks
  const clerk = useClerk();
  // eslint-disable-next-line react-hooks/rules-of-hooks
  const { getToken } = useClerkAuth();

  return {
    isCloudAuthenticated: !!isSignedIn,
    isCloudLoading: !isLoaded,
    user: user
      ? {
          name: user.fullName ?? undefined,
          email: user.primaryEmailAddress?.emailAddress,
          picture: user.imageUrl,
        }
      : undefined,
    signIn: () => clerk.openSignIn(),
    getToken: () => getToken(),
    signOut: () => clerk.signOut(),
  };
}
