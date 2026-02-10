import { useState } from "react";
import { vaultFetch } from "../lib/api";

interface UnsealResponse {
  sealed: boolean;
  threshold: number;
  progress: number;
}

export function UnsealPage() {
  const [share, setShare] = useState("");
  const [loading, setLoading] = useState(false);
  const [progress, setProgress] = useState<{ current: number; threshold: number } | null>(null);
  const [unsealed, setUnsealed] = useState(false);
  const [error, setError] = useState("");

  async function handleSubmit() {
    setError("");
    if (!share.trim()) return;
    setLoading(true);
    try {
      const data = await vaultFetch<UnsealResponse>("/v1/sys/unseal", {
        method: "POST",
        body: JSON.stringify({ share: share.trim() }),
      });
      setShare("");
      if (!data.sealed) {
        setUnsealed(true);
      } else {
        setProgress({ current: data.progress, threshold: data.threshold });
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Unseal failed");
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="max-w-[640px]">
      <div className="bg-glass backdrop-blur-[16px] border border-glass-border rounded-[20px] p-[30px] shadow-[0_8px_32px_rgba(0,0,0,.06)] mb-[22px]">
        <div className="text-2xl mb-4">🔑</div>
        <h3 className="text-lg font-extrabold tracking-tight mb-2">Submit Unseal Share</h3>
        <p className="text-sm text-stone-500 leading-relaxed mb-[18px]">
          Enter unseal key shares one at a time. When the threshold is reached,
          the vault will unseal and begin serving requests.
        </p>
        <div className="mb-5">
          <label className="block text-[13px] font-bold text-stone-700 mb-[7px]">Unseal Key Share</label>
          <input
            type="text" value={share}
            onChange={(e) => setShare(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSubmit()}
            placeholder="Paste a base64-encoded unseal share..."
            className="w-full px-4 py-[11px] border border-stone-300/40 rounded-[10px] text-[13px] font-mono bg-glass backdrop-blur-[8px] text-stone-800 focus:outline-none focus:border-amber-500 focus:ring-[3px] focus:ring-amber-500/12 placeholder:text-stone-400"
          />
        </div>

        {error && (
          <div className="bg-red-500/10 text-red-700 px-4 py-2.5 rounded-[10px] text-[13px] font-semibold mb-4">{error}</div>
        )}

        <button onClick={handleSubmit} disabled={loading}
          className="px-5 py-2.5 rounded-full bg-amber-500 text-amber-900 font-semibold text-[13px] hover:bg-amber-600 hover:shadow-[0_4px_20px_rgba(0,0,0,.12)] hover:-translate-y-px transition-all disabled:opacity-50 cursor-pointer">
          {loading ? "Submitting…" : "Submit Share"}
        </button>

        {progress && (
          <div className="mt-5">
            <span className="inline-block px-[18px] py-2 rounded-full bg-amber-500/12 text-amber-700 text-[13px] font-bold">
              Progress: {progress.current} / {progress.threshold} shares submitted
            </span>
          </div>
        )}
      </div>

      {unsealed && (
        <div className="bg-glass backdrop-blur-[16px] border border-glass-border rounded-[20px] p-[30px] shadow-[0_8px_32px_rgba(0,0,0,.06)]">
          <div className="inline-flex items-center justify-center w-9 h-9 rounded-full bg-gradient-to-br from-green-500 to-green-400 text-white text-sm font-extrabold mb-4">✓</div>
          <h3 className="text-lg font-extrabold tracking-tight mb-2">Vault Unsealed</h3>
          <p className="text-sm text-stone-500 leading-relaxed mb-[18px]">
            The vault is now unsealed and ready to serve requests. All secrets engines are active.
          </p>
          <a href="/app" className="inline-flex px-5 py-2.5 rounded-full bg-amber-500 text-amber-900 font-semibold text-[13px] hover:bg-amber-600 transition-all">
            Go to Dashboard
          </a>
        </div>
      )}
    </div>
  );
}
