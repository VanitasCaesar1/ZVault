/**
 * Customer portal redirect.
 *
 * If a `customer_id` cookie exists (set after login), creates an
 * authenticated Polar portal session. Otherwise, redirects to
 * Polar's hosted portal where customers enter their email.
 */
import type { APIRoute } from "astro";
import { createPortalSession } from "../../lib/polar";

export const prerender = false;

const POLAR_PORTAL_FALLBACK = "https://polar.sh/zvault-cloud/portal";

export const GET: APIRoute = async ({ cookies, redirect }) => {
  const customerId = cookies.get("zvault_customer_id")?.value;

  if (customerId) {
    const portalUrl = await createPortalSession(customerId);
    if (portalUrl) {
      return redirect(portalUrl, 302);
    }
  }

  // Fallback: Polar's hosted portal (customer enters email)
  return redirect(POLAR_PORTAL_FALLBACK, 302);
};
