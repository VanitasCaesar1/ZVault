/**
 * POST /api/account/logout
 *
 * Clears session cookies.
 */
import type { APIRoute } from "astro";

export const prerender = false;

export const POST: APIRoute = async ({ cookies, redirect }) => {
  cookies.delete("zvault_customer_id", { path: "/" });
  cookies.delete("zvault_customer_email", { path: "/" });
  return redirect("/account/login", 302);
};
