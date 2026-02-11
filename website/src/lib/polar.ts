/**
 * Polar API helper for customer lookup and session management.
 * Uses the @polar-sh/sdk for typed API calls.
 */
import { Polar } from "@polar-sh/sdk";

const POLAR_ORG_ID = "2eb1c165-a876-4932-baf7-6119c4c06816";

/** Product ID → tier mapping */
const PRODUCT_TIER_MAP: Record<string, string> = {
  "49f34606-431c-4215-a0c9-19ea745e5a93": "pro",
  "c42a3bec-5db8-4cf2-b9c6-48416604353e": "team",
  "a2aaaded-328e-4320-a493-76bf7b898e45": "enterprise",
};

function getPolar(): Polar {
  return new Polar({
    accessToken: import.meta.env.POLAR_ACCESS_TOKEN ?? "",
  });
}

export interface CustomerInfo {
  id: string;
  email: string;
  name: string | null;
  createdAt: string;
  subscriptions: SubscriptionInfo[];
  tier: string;
  licenseKeys: LicenseKeyInfo[];
}

export interface SubscriptionInfo {
  id: string;
  status: string;
  productId: string;
  productName: string;
  tier: string;
  currentPeriodEnd: string | null;
  cancelAtPeriodEnd: boolean;
}

export interface LicenseKeyInfo {
  id: string;
  key: string;
  status: string;
  tier: string;
  activations: number;
  maxActivations: number | null;
}

/**
 * Look up a Polar customer by email.
 * Returns null if no customer found.
 */
export async function findCustomerByEmail(email: string): Promise<CustomerInfo | null> {
  const polar = getPolar();

  try {
    const customers = await polar.customers.list({
      email,
      organizationId: POLAR_ORG_ID,
      limit: 1,
    });

    const items = customers.result?.items ?? [];
    if (items.length === 0) return null;

    const customer = items[0];
    return await buildCustomerInfo(polar, customer);
  } catch (e) {
    console.error("[polar] customer lookup failed:", e);
    return null;
  }
}

/**
 * Look up a Polar customer by ID.
 */
export async function findCustomerById(customerId: string): Promise<CustomerInfo | null> {
  const polar = getPolar();

  try {
    const customer = await polar.customers.get({ id: customerId });
    if (!customer) return null;
    return await buildCustomerInfo(polar, customer);
  } catch (e) {
    console.error("[polar] customer get failed:", e);
    return null;
  }
}

/**
 * Create an authenticated customer portal session URL.
 */
export async function createPortalSession(customerId: string): Promise<string | null> {
  const polar = getPolar();

  try {
    const session = await polar.customerSessions.create({
      customerId,
    });
    return session.customerPortalUrl ?? null;
  } catch (e) {
    console.error("[polar] portal session creation failed:", e);
    return null;
  }
}

/** Build full CustomerInfo from a Polar customer object */
async function buildCustomerInfo(polar: Polar, customer: any): Promise<CustomerInfo> {
  // Fetch subscriptions
  let subscriptions: SubscriptionInfo[] = [];
  try {
    const subs = await polar.subscriptions.list({
      customerId: customer.id,
      organizationId: POLAR_ORG_ID,
      limit: 10,
    });
    subscriptions = (subs.result?.items ?? []).map((s: any) => ({
      id: s.id,
      status: s.status,
      productId: s.productId ?? s.product_id ?? "",
      productName: s.product?.name ?? "ZVault",
      tier: PRODUCT_TIER_MAP[s.productId ?? s.product_id ?? ""] ?? "pro",
      currentPeriodEnd: s.currentPeriodEnd ?? s.current_period_end ?? null,
      cancelAtPeriodEnd: s.cancelAtPeriodEnd ?? s.cancel_at_period_end ?? false,
    }));
  } catch (e) {
    console.error("[polar] subscription fetch failed:", e);
  }

  // Fetch license keys
  let licenseKeys: LicenseKeyInfo[] = [];
  try {
    const keys = await polar.licenseKeys.list({
      customerId: customer.id,
      organizationId: POLAR_ORG_ID,
      limit: 10,
    });
    licenseKeys = (keys.result?.items ?? []).map((k: any) => ({
      id: k.id,
      key: k.key,
      status: k.status,
      tier: PRODUCT_TIER_MAP[k.benefitId ?? k.benefit_id ?? ""] ?? "pro",
      activations: k.activations?.length ?? k.usage ?? 0,
      maxActivations: k.limitActivations ?? k.limit_activations ?? null,
    }));
  } catch (e) {
    console.error("[polar] license key fetch failed:", e);
  }

  // Determine highest tier
  const activeSub = subscriptions.find((s) => s.status === "active");
  const tier = activeSub?.tier ?? (licenseKeys.length > 0 ? licenseKeys[0].tier : "free");

  return {
    id: customer.id,
    email: customer.email,
    name: customer.name ?? null,
    createdAt: customer.createdAt ?? customer.created_at ?? "",
    subscriptions,
    tier,
    licenseKeys,
  };
}
