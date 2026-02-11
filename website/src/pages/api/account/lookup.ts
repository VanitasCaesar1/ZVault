/**
 * POST /api/account/lookup
 *
 * Looks up a customer by email in Polar. If found, sets a session
 * cookie and returns customer info. This is a simple email-based
 * "login" — Polar is the source of truth for customer identity.
 *
 * For a production app you'd want email verification (magic link),
 * but for an MVP where the email is already verified by Polar's
 * checkout flow, this is sufficient.
 */
import type { APIRoute } from "astro";
import { findCustomerByEmail } from "../../../lib/polar";

export const prerender = false;

export const POST: APIRoute = async ({ request, cookies }) => {
  const body = await request.json().catch(() => null);
  const email = body?.email?.trim()?.toLowerCase();

  if (!email || !email.includes("@")) {
    return new Response(
      JSON.stringify({ error: "Valid email is required" }),
      { status: 400, headers: { "Content-Type": "application/json" } }
    );
  }

  const customer = await findCustomerByEmail(email);

  if (!customer) {
    return new Response(
      JSON.stringify({ error: "No account found for this email. Purchase a plan first or check the email you used at checkout." }),
      { status: 404, headers: { "Content-Type": "application/json" } }
    );
  }

  // Set session cookies (24h expiry)
  cookies.set("zvault_customer_id", customer.id, {
    path: "/",
    maxAge: 86400,
    httpOnly: true,
    secure: import.meta.env.PROD,
    sameSite: "lax",
  });
  cookies.set("zvault_customer_email", customer.email, {
    path: "/",
    maxAge: 86400,
    httpOnly: false, // readable by client for display
    secure: import.meta.env.PROD,
    sameSite: "lax",
  });

  return new Response(
    JSON.stringify({ ok: true, customer }),
    { status: 200, headers: { "Content-Type": "application/json" } }
  );
};
