import { Webhooks } from "@polar-sh/astro";
import * as ed from "@noble/ed25519";
import { sha512 } from "@noble/hashes/sha2.js";

// Required: set the sha512 sync function for ed25519 v3 + noble/hashes v2.
ed.hashes.sha512 = sha512;

export const prerender = false;

// Ed25519 private key (base64) — must match the public key embedded in the CLI.
// In production, load from env: import.meta.env.ZVAULT_SIGNING_KEY
const SIGNING_KEY_B64 = import.meta.env.ZVAULT_SIGNING_KEY ?? "tDltVn8JVCeiL10rrbQQJf3E4RTVvSsVnEXMiTxWhFc=";

// Map Polar product IDs to ZVault tiers.
const PRODUCT_TIER_MAP: Record<string, string> = {
  "49f34606-431c-4215-a0c9-19ea745e5a93": "pro",
  "c42a3bec-5db8-4cf2-b9c6-48416604353e": "team",
  "a2aaaded-328e-4320-a493-76bf7b898e45": "enterprise",
};

/**
 * Sign a license payload with Ed25519 and return a license key string.
 *
 * Format: `<base64(payload)>.<base64(signature)>`
 * This matches what the CLI's `verify_license_key()` expects.
 */
function signLicenseKey(payload: object): string {
  const privateKeyBytes = Uint8Array.from(atob(SIGNING_KEY_B64), (c) => c.charCodeAt(0));
  const payloadJson = JSON.stringify(payload);
  const payloadB64 = btoa(payloadJson);
  const payloadBytes = new TextEncoder().encode(payloadB64);

  // Sign the base64-encoded payload (matches CLI verification: sign over base64 string).
  const signature = ed.sign(payloadBytes, privateKeyBytes);
  const sigB64 = btoa(String.fromCharCode(...signature));

  return `${payloadB64}.${sigB64}`;
}

/** Build an ISO 8601 timestamp string. */
function isoNow(): string {
  return new Date().toISOString().replace(/\.\d{3}Z$/, "Z");
}

/** Build an expiry date 1 year from now. */
function isoOneYearFromNow(): string {
  const d = new Date();
  d.setFullYear(d.getFullYear() + 1);
  return d.toISOString().replace(/\.\d{3}Z$/, "Z");
}

export const POST = Webhooks({
  webhookSecret: import.meta.env.POLAR_WEBHOOK_SECRET!,

  onSubscriptionCreated: async (payload) => {
    console.log("[polar] subscription created:", payload.data.id);
  },

  onSubscriptionActive: async (payload) => {
    const sub = payload.data;
    console.log("[polar] subscription active:", sub.id);

    // Resolve tier from product ID.
    const productId = sub.product_id ?? "";
    const tier = PRODUCT_TIER_MAP[productId] ?? "pro";
    const email = sub.customer?.email ?? sub.user?.email ?? "unknown";

    // Generate a signed license key.
    const licensePayload = {
      tier,
      email,
      issued_at: isoNow(),
      expires_at: isoOneYearFromNow(),
      license_id: `polar_${sub.id}`,
    };

    const licenseKey = signLicenseKey(licensePayload);

    console.log("[polar] license key generated for", email, "tier:", tier);
    console.log("[polar] key:", licenseKey.slice(0, 40) + "...");

    // TODO: Send the license key to the customer via email or store in Polar metadata.
    // For now, the key is logged. In production, integrate with an email service
    // (e.g., Resend, Postmark) to deliver the key automatically.
  },

  onSubscriptionCanceled: async (payload) => {
    console.log("[polar] subscription canceled:", payload.data.id);
    // License key auto-expires based on expires_at in the signed payload.
    // No revocation needed — the CLI checks expiry locally.
  },

  onSubscriptionRevoked: async (payload) => {
    console.log("[polar] subscription revoked:", payload.data.id);
  },
});
