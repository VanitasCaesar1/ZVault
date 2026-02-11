/**
 * GET /api/account/me
 *
 * Returns the current customer's info from Polar.
 * Requires the zvault_customer_id cookie (set by /api/account/lookup).
 */
import type { APIRoute } from "astro";
import { findCustomerById } from "../../../lib/polar";

export const prerender = false;

export const GET: APIRoute = async ({ cookies }) => {
  const customerId = cookies.get("zvault_customer_id")?.value;

  if (!customerId) {
    return new Response(
      JSON.stringify({ error: "Not logged in" }),
      { status: 401, headers: { "Content-Type": "application/json" } }
    );
  }

  const customer = await findCustomerById(customerId);

  if (!customer) {
    // Customer ID in cookie is stale — clear it
    cookies.delete("zvault_customer_id", { path: "/" });
    cookies.delete("zvault_customer_email", { path: "/" });
    return new Response(
      JSON.stringify({ error: "Customer not found" }),
      { status: 404, headers: { "Content-Type": "application/json" } }
    );
  }

  return new Response(
    JSON.stringify({ customer }),
    { status: 200, headers: { "Content-Type": "application/json" } }
  );
};
