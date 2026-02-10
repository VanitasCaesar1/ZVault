import { useEffect, useState } from "react";
import { vaultFetch } from "../lib/api";
import { Th, Td, Badge } from "../components/Table";

interface AuditEntry {
  id: string;
  timestamp: string;
  request: {
    operation: string;
    path: string;
    remote_addr: string;
  };
  response: {
    status_code: number;
    error?: string;
  };
  auth: {
    token_id: string;
    policies: string[];
  };
}

interface AuditLogResponse {
  entries: AuditEntry[];
  count: number;
}

export function AuditPage() {
  const [entries, setEntries] = useState<AuditEntry[]>([]);
  const [filter, setFilter] = useState("");
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    vaultFetch<AuditLogResponse>("/v1/sys/audit-log?limit=200")
      .then((data) => setEntries(data.entries ?? []))
      .catch(() => setEntries([]))
      .finally(() => setLoading(false));
  }, []);

  const filtered = entries.filter((e) =>
    (e.request?.path ?? "").toLowerCase().includes(filter.toLowerCase())
  );

  const fmtTime = (ts: string) => {
    try {
      const d = new Date(ts);
      return d.toLocaleString("en-US", {
        month: "short", day: "numeric",
        hour: "2-digit", minute: "2-digit", second: "2-digit",
        hour12: false,
      });
    } catch {
      return ts;
    }
  };

  const truncHash = (h: string) =>
    h.length > 12 ? `hmac:${h.slice(0, 4)}…${h.slice(-4)}` : h;

  return (
    <>
      <div className="flex justify-between items-center mb-6">
        <p className="text-sm text-stone-500">
          Immutable record of every operation. Sensitive fields are HMAC'd.
        </p>
        <div className="flex gap-2">
          <input
            type="text" value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Filter by path..."
            className="w-[200px] px-3.5 py-[7px] text-[13px] border border-stone-300/40 rounded-full bg-glass focus:outline-none focus:border-amber-500"
          />
        </div>
      </div>

      <div className="bg-glass backdrop-blur-[16px] border border-glass-border rounded-[20px] shadow-[0_8px_32px_rgba(0,0,0,.06)] overflow-hidden">
        <table className="w-full text-[13px] border-collapse">
          <thead>
            <tr>
              <Th>Timestamp</Th>
              <Th>Operation</Th>
              <Th>Path</Th>
              <Th>Actor</Th>
              <Th>Status</Th>
            </tr>
          </thead>
          <tbody>
            {loading ? (
              <tr>
                <td colSpan={5} className="px-[18px] py-4 text-center text-stone-400">
                  Loading…
                </td>
              </tr>
            ) : filtered.length === 0 ? (
              <tr>
                <td colSpan={5} className="px-[18px] py-4 text-center text-stone-400">
                  {entries.length === 0
                    ? "No audit entries yet. Operations will appear here once the vault is in use."
                    : "No entries match your filter."}
                </td>
              </tr>
            ) : (
              filtered.map((e, i) => (
                <tr key={e.id ?? i} className="hover:bg-stone-50">
                  <Td>
                    <span className="font-mono text-xs">{fmtTime(e.timestamp)}</span>
                  </Td>
                  <Td>{e.request?.operation ?? "—"}</Td>
                  <Td>
                    <code className="bg-stone-100 text-stone-600 px-1.5 py-0.5 rounded text-xs">
                      {e.request?.path ?? "—"}
                    </code>
                  </Td>
                  <Td>
                    <code className="text-xs">{truncHash(e.auth?.token_id ?? "—")}</code>
                  </Td>
                  <Td>
                    <Badge variant={e.response?.status_code < 400 ? "success" : "danger"}>
                      {e.response?.status_code ?? "—"}
                    </Badge>
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
