import { useState } from "react";
import { PricingTable, Protect } from "@clerk/clerk-react";
import { CLERK_ENABLED } from "../lib/clerk";

type PlanScope = "user" | "organization";

export function BillingPage() {
  const [scope, setScope] = useState<PlanScope>("user");

  if (!CLERK_ENABLED) {
    return (
      <div className="max-w-2xl">
        <h2 className="text-lg font-bold text-stone-800 mb-2">Billing</h2>
        <p className="text-sm text-stone-500 mb-6">
          Billing is available on ZVault Cloud. Set{" "}
          <code className="text-xs bg-stone-100 px-1.5 py-0.5 rounded font-mono">
            VITE_CLERK_PUBLISHABLE_KEY
          </code>{" "}
          to enable cloud features.
        </p>
        <a
          href="https://zvault.cloud/pricing"
          target="_blank"
          rel="noopener noreferrer"
          className="inline-flex items-center gap-2 px-4 py-2.5 rounded-full bg-amber-500 text-amber-900 font-semibold text-sm hover:bg-amber-600 transition-colors"
        >
          View Plans on zvault.cloud
        </a>
      </div>
    );
  }

  return (
    <div className="max-w-5xl">
      <div className="mb-8">
        <h2 className="text-lg font-bold text-stone-800 mb-1">Plans & Billing</h2>
        <p className="text-sm text-stone-500">
          Choose a plan that fits your needs. Upgrade or downgrade anytime.
        </p>
      </div>

      {/* Scope toggle: Individual vs Team/Org */}
      <div className="flex items-center gap-1 bg-stone-100 rounded-full p-1 w-fit mb-8">
        <button
          onClick={() => setScope("user")}
          className={`px-4 py-1.5 rounded-full text-sm font-semibold transition-all cursor-pointer ${
            scope === "user"
              ? "bg-white text-stone-800 shadow-sm"
              : "text-stone-500 hover:text-stone-700"
          }`}
        >
          Individual
        </button>
        <button
          onClick={() => setScope("organization")}
          className={`px-4 py-1.5 rounded-full text-sm font-semibold transition-all cursor-pointer ${
            scope === "organization"
              ? "bg-white text-stone-800 shadow-sm"
              : "text-stone-500 hover:text-stone-700"
          }`}
        >
          Team & Organization
        </button>
      </div>

      <Protect
        fallback={
          <div className="bg-glass border border-glass-border rounded-2xl p-8 text-center">
            <p className="text-sm text-stone-500 mb-4">
              Sign in to view and manage your subscription.
            </p>
          </div>
        }
      >
        <PricingTable key={scope} for={scope} />
      </Protect>
    </div>
  );
}
