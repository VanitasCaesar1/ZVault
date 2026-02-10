import { useState } from "react";
import { vaultFetch } from "../lib/api";
import { Th, Td, Badge } from "../components/Table";

interface LeaseInfo {
  lease_id: string;
  engine_path: string;
  issued_at: string;
  ttl_secs: number;
  renewable: boolean;
  expired: boolean;
}

export function LeasesPage() {
  const [lookupId, setLookupId] = useState("");
  const [leases, setLeases] = useState<LeaseInfo[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const lookupLease = async () => {
    if (!lookupId.trim()) return;
    setError(null);
    setLoading(true);
    try {
      const resp = await vaultFetch<LeaseInfo>("/v1/sys/leases/lookup", {
        method: "POST",
        body: JSON.stringify({ lease_id: lookupId.trim() }),
      });
      // Add to list if not already present.
      setLeases((prev) => {
        const exists = prev.some((l) => l.lease_id === resp.lease_id);
        return exists
          ? prev.map((l) => (l.lease_id === resp.lease_id ? resp : l))
          : [resp, ...prev];
      });
      setLookupId("");
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : "Lookup failed";
      setError(msg);
    } finally {
      setLoading(false);
    }
  };

  const renewLease = async (leaseId: string) => {
    try {
      const resp = await vaultFetch<LeaseInfo>("/v1/sys/leases/renew", {
        method: "POST",
        body: JSON.stringify({ lease_id: leaseId }),
      });
      setLeases((prev) =>
        prev.map((l) => (l.lease_id === leaseId ? resp : l))
      );
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : "Renew failed";
      setError(msg);
    }
  };

  const revokeLease = async (leaseId: string) => {
    try {
      await vaultFetch("/v1/sys/leases/revoke", {
        method: "POST",
        body: JSON.stringify({ lease_id: leaseId }),
      });
      setLeases((prev) => prev.filter((l) => l.lease_id !== leaseId));
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : "Revoke failed";
      setError(msg);
    }
  };

  const formatTtl = (secs: number): string => {
    if (secs <= 0) return "Expired";
    if (secs < 60) return `${secs}s`;
    if (secs < 3600) return `${Math.floor(secs / 60)}m`;
    if (secs < 86400) return `${Math.floor(secs / 3600)}h`;
    return `${Math.floor(secs / 86400)}d`;
  };

  return (
    <>
      <div className="mb-6">
        <p className="text-sm text-stone-500">
          Look up, renew, and revoke leases for dynamic credentials.
        </p>
      </div>

      {/* Lookup bar */}
      <div className="flex gap-2 mb-6">
        <input
          type="text"
          value={lookupId}
          onChange={(e) => setLookupId(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && lookupLease()}
          placeholder="Enter lease ID to look up…"
          className="flex-1 px-4 py-2.5 text-[13px] border border-stone-300/40 rounded-full bg-glass focus:outline-none focus:border-amber-500"
        />
        <button
          onClick={lookupLease}
          disabled={loading || !lookupId.trim()}
          className="px-5 py-2.5 rounded-full bg-amber-500 text-amber-900 text-[13px] font-semibold hover:bg-amber-600 transition-all cursor-pointer disabled:opacity-50"
        >
          {loading ? "Looking up…" : "Lookup"}
        </button>
      </div>

      {error && (
        <div className="mb-4 px-4 py-3 rounded-xl bg-red-50 border border-red-200 text-red-700 text-sm">
          {error}
        </div>
      )}

      <div className="bg-glass backdrop-blur-[16px] border border-glass-border rounded-[20px] shadow-[0_8px_32px_rgba(0,0,0,.06)] overflow-hidden">
        <table className="w-full text-[13px] border-collapse">
          <thead>
            <tr>
              <Th>Lease ID</Th>
              <Th>Engine</Th>
              <Th>Issued</Th>
              <Th>TTL</Th>
              <Th>Status</Th>
              <Th>Actions</Th>
            </tr>
          </thead>
          <tbody>
            {leases.length === 0 ? (
              <tr>
                <Td colSpan={6}>
                  <span className="text-stone-400">
                    No leases loaded. Use the lookup bar above to find a lease by ID.
                  </span>
                </Td>
              </tr>
            ) : (
              leases.map((l) => (
                <tr key={l.lease_id} className="hover:bg-stone-50">
                  <Td>
                    <code className="text-xs">{l.lease_id}</code>
                  </Td>
                  <Td>{l.engine_path}</Td>
                  <Td>
                    <span className="font-mono text-xs">
                      {new Date(l.issued_at).toLocaleString()}
                    </span>
                  </Td>
                  <Td>{formatTtl(l.ttl_secs)}</Td>
                  <Td>
                    <Badge variant={l.expired ? "danger" : "success"}>
                      {l.expired ? "Expired" : "Active"}
                    </Badge>
                  </Td>
                  <Td>
                    {!l.expired ? (
                      <div className="flex gap-1">
                        {l.renewable && (
                          <button
                            onClick={() => renewLease(l.lease_id)}
                            className="px-3 py-[5px] text-xs font-semibold rounded-full bg-glass border border-stone-300/40 text-stone-700 hover:bg-surface transition-all cursor-pointer"
                          >
                            Renew
                          </button>
                        )}
                        <button
                          onClick={() => revokeLease(l.lease_id)}
                          className="px-3 py-[5px] text-xs font-semibold rounded-full bg-red-500 text-white hover:bg-red-600 transition-all cursor-pointer"
                        >
                          Revoke
                        </button>
                      </div>
                    ) : (
                      <span className="text-xs text-stone-400">Expired</span>
                    )}
                  </Td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </>
  );
}
