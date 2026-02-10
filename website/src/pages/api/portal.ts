import { CustomerPortal } from "@polar-sh/astro";

export const prerender = false;

export const GET = CustomerPortal({
  accessToken: import.meta.env.POLAR_ACCESS_TOKEN!,
});
