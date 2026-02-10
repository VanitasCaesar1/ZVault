import { useState } from "react";
import { vaultFetch } from "../lib/api";

interface InitResponse {
  unseal_shares: string[];
  root_token: string;
}

export function InitPage() {
  const [shares, setShares] = useState(5);
  const [threshold, setThreshold] = useState(3);
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<InitResponse | null>(null);
  const [error, setError] = useState("");

  async function handleInit() {
    setError("");
    setLoading(true);
    try {
      const data = await vaultFetch<InitResponse>("/v1/sys/init", {
        method: "POST",
        body: JSON.stringify({ shares, threshold }),
      });
      setResult(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Initialization failed");
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="max-w-[640px]">
      <div className="bg-glass backdrop-blur-[16px] border border-glass-border rounded-[20px] p-[30px] shadow-[0_8px_32px_rgba(0,0,0,.06)] mb-[22px]">
        <div className="inline-flex items-center justify-center w-9 h-9 rounded-full bg-gradient-to-br from-amber-400 to-amber-500 text-amber-900 text-sm font-extrabold mb-4 shadow-[0_4px_12px_rgba(0,0,0,.1)]">
          1
        </div>
        <h3 className="text-lg font-extrabold tracking-tight mb-2">Initialize Your Vault</h3>
        <p className="text-sm text-stone-500 leading-relaxed mb-[18px]">
          Generate the root encryption key and split the unseal key into Shamir shares.
          This can only be done once. Store the shares securely — they cannot be recovered.
        </p>

        <div className="mb-5">
          <label className="block text-[13px] font-bold text-stone-700 mb-[7px]">Number of Key Shares</label>
          <input
            type="number" min={2} max={10} value={shares}
            onChange={(e) => setShares(Number(e.target.value))}
            className="w-full px-4 py-[11px] border border-stone-300/40 rounded-[10px] text-[13px] bg-glass backdrop-blur-[8px] text-stone-800 focus:outline-none focus:border-amber-500 focus:ring-[3px] focus:ring-amber-500/12"
          />
          <p className="text-xs text-stone-400 mt-1.5">Total unseal key shares to generate (2–10)</p>
        </div>

        <div className="mb-5">
          <label className="block text-[13px] font-bold text-stone-700 mb-[7px]">Key Threshold</label>
          <input
            type="number" min={2} max={10} value={threshold}
            onChange={(e) => setThreshold(Number(e.target.value))}
            className="w-full px-4 py-[11px] border border-stone-300/40 rounded-[10px] text-[13px] bg-glass backdrop-blur-[8px] text-stone-800 focus:outline-none focus:border-amber-500 focus:ring-[3px] focus:ring-amber-500/12"
          />
          <p className="text-xs text-stone-400 mt-1.5">Minimum shares required to unseal (2 to share count)</p>
        </div>

        {error && (
          <div className="bg-red-500/10 text-red-700 px-4 py-2.5 rounded-[10px] text-[13px] font-semibold mb-4">{error}</div>
        )}

        {!result && (
          <button onClick={handleInit} disabled={loading}
            className="px-5 py-2.5 rounded-full bg-amber-500 text-amber-900 font-semibold text-[13px] hover:bg-amber-600 hover:shadow-[0_4px_20px_rgba(0,0,0,.12)] hover:-translate-y-px transition-all disabled:opacity-50 cursor-pointer">
            {loading ? "Initializing…" : "Initialize Vault"}
          </button>
        )}
      </div>

      {result && (
        <div className="bg-glass backdrop-blur-[16px] border border-glass-border rounded-[20px] p-[30px] shadow-[0_8px_32px_rgba(0,0,0,.06)]">
          <div className="inline-flex items-center justify-center w-9 h-9 rounded-full bg-gradient-to-br from-amber-400 to-amber-500 text-amber-900 text-sm font-extrabold mb-4 shadow-[0_4px_12px_rgba(0,0,0,.1)]">
            2
          </div>
          <h3 className="text-lg font-extrabold tracking-tight mb-2">Save Your Unseal Shares</h3>
          <p className="text-sm text-stone-500 leading-relaxed mb-[18px]">
            These shares are shown <strong>once</strong> and never stored by ZVault.
            Distribute them to trusted operators.
          </p>

          {result.unseal_shares.map((share, i) => (
            <div key={i} className="bg-sidebar text-amber-200 p-[18px] rounded-[10px] font-mono text-xs leading-relaxed break-all mb-2">
              <span className="text-amber-400">Share {i + 1}:</span>{" "}
              <span className="text-green-400">{share}</span>
            </div>
          ))}

          <div className="mt-5">
            <label className="block text-[13px] font-bold text-stone-700 mb-[7px]">Root Token</label>
            <div className="bg-sidebar text-amber-200 p-[18px] rounded-[10px] font-mono text-xs leading-relaxed break-all">
              {result.root_token}
            </div>
            <p className="text-xs text-stone-400 mt-1.5">
              Use this token for initial authentication. Create scoped tokens and revoke this one.
            </p>
          </div>

          <div className="mt-5">
            <a href="/app/unseal" className="inline-flex px-5 py-2.5 rounded-full bg-amber-500 text-amber-900 font-semibold text-[13px] hover:bg-amber-600 transition-all">
              Proceed to Unseal
            </a>
          </div>
        </div>
      )}
    </div>
  );
}
