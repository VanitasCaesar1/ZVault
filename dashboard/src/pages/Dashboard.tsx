import { useEffect, useState } from "react";
import { useOutletContext } from "react-router";
import { vaultFetch, type SealStatus } from "../lib/api";

interface MountEntry {
  path: string;
  engine_type: string;
  description: string;
}

interface AuditEntry {
  id: string;
  timestamp: string;
  request: { operation: string; path: string };
  response: { status_code: number };
  auth: { token_id: string };
}

interface DashboardStats {
  secretCount: number | null;
  policyCount: number | null;
  mountCount: number | null;
  mounts: MountEntry[];
  recentActivity: AuditEntry[];
  activityBuckets: number[];
}

export function DashboardPage() {
  const { sealStatus } = useOutletContext<{ sealStatus: SealStatus | null }>();
  const [stats, setStats] = useState<DashboardStats>({
    secretCount: null, policyCount: null, mountCount: null,
    mounts: [], recentActivity: [], activityBuckets: [],
  });

  const isUnsealed = sealStatus?.initialized && !sealStatus?.sealed;

  useEffect(() => {
    if (!isUnsealed) return;

    const fetchStats = async () => {
      const [secrets, policies, mounts, audit] = await Promise.allSettled([
        vaultFetch<{ data?: { keys?: string[] }; keys?: string[] }>("/v1/secret/list/"),
        vaultFetch<{ policies?: string[] }>("/v1/sys/policies"),
        vaultFetch<{ mounts?: MountEntry[] }>("/v1/sys/mounts"),
        vaultFetch<{ entries?: AuditEntry[] }>("/v1/sys/audit-log?limit=50"),
      ]);

      const auditEntries =
        audit.status === "fulfilled" ? (audit.value.entries ?? []) : [];

      // Build sparkline: count entries per hour for last 12 hours
      const buckets = buildSparkline(auditEntries, 12);

      setStats({
        secretCount:
          secrets.status === "fulfilled"
            ? (secrets.value.data?.keys?.length ?? secrets.value.keys?.length ?? 0)
            : 0,
        policyCount:
          policies.status === "fulfilled"
            ? (policies.value.policies?.length ?? 0)
            : 0,
        mountCount:
          mounts.status === "fulfilled"
            ? (mounts.value.mounts?.length ?? 0)
            : 0,
        mounts:
          mounts.status === "fulfilled" ? (mounts.value.mounts ?? []) : [],
        recentActivity: auditEntries.slice(0, 6),
        activityBuckets: buckets,
      });
    };

    fetchStats();
  }, [isUnsealed]);

  const sealLabel = !sealStatus
    ? "Loading…"
    : !sealStatus.initialized
      ? "Not Initialized"
      : sealStatus.sealed
        ? "Sealed"
        : "Unsealed";

  const sealColor = !sealStatus
    ? "text-stone-500"
    : !sealStatus.initialized
      ? "text-red-500"
      : sealStatus.sealed
        ? "text-amber-600"
        : "text-green-600";

  const fmt = (n: number | null) => (n === null ? "—" : String(n));

  return (
    <>
      {/* Bento stat cards — mixed sizes */}
      <div className="grid grid-cols-4 gap-[18px] mb-8 max-lg:grid-cols-2 max-sm:grid-cols-1">
        <StatCard label="Seal Status" value={sealLabel} valueClass={sealColor} sub="Requires unseal shares to operate" />
        <StatCard label="Secrets Stored" value={fmt(stats.secretCount)} valueClass="text-amber-500" sub="Across all mounted engines" />
        <StatCard label="Policies" value={fmt(stats.policyCount)} valueClass="text-stone-600" sub="Access control rules defined" />
        <StatCard label="Engines Mounted" value={fmt(stats.mountCount)} valueClass="text-green-600" sub="Active secrets engines" />
      </div>

      {/* Activity sparkline — full width */}
      {stats.activityBuckets.length > 0 && (
        <div className="bg-glass backdrop-blur-[16px] border border-glass-border rounded-[20px] shadow-[0_8px_32px_rgba(0,0,0,.06)] p-6 mb-8">
          <div className="text-[11px] font-bold uppercase tracking-[.7px] text-stone-500 mb-3">
            Activity — Last 12 Hours
          </div>
          <Sparkline buckets={stats.activityBuckets} />
        </div>
      )}

      <div className="grid grid-cols-5 gap-[18px] max-lg:grid-cols-1">
        {/* Mounted Engines — 2 cols */}
        <div className="col-span-2 bg-glass backdrop-blur-[16px] border border-glass-border rounded-[20px] shadow-[0_8px_32px_rgba(0,0,0,.06)] overflow-hidden max-lg:col-span-1">
          <div className="flex items-center justify-between px-[22px] py-[18px] border-b border-stone-200/60 bg-white/25">
            <span className="text-[15px] font-bold text-stone-800">Mounted Engines</span>
          </div>
          <div className="px-[22px] py-4">
            {stats.mounts.length > 0 ? (
              stats.mounts.map((m) => (
                <EngineRow key={m.path} name={m.engine_type} path={m.path} description={m.description} />
              ))
            ) : (
              <>
                <EngineRow name="KV v2" path="secret/" description="Key-value secrets" />
                <EngineRow name="Transit" path="transit/" description="Encryption as a service" />
              </>
            )}
          </div>
        </div>

        {/* Recent Activity — 3 cols, dark card */}
        <div className="col-span-3 bg-sidebar border border-white/6 rounded-[20px] shadow-[0_8px_32px_rgba(0,0,0,.06)] overflow-hidden max-lg:col-span-1">
          <div className="flex items-center justify-between px-[22px] py-[18px] border-b border-white/6 bg-white/4">
            <span className="text-[15px] font-bold text-sidebar-active">Recent Activity</span>
            <a href="/app/audit" className="text-[11px] font-semibold px-3.5 py-[5px] rounded-full bg-amber-500/15 text-amber-400">
              View All
            </a>
          </div>
          <div className="px-[22px] py-2">
            {stats.recentActivity.length > 0 ? (
              stats.recentActivity.map((e, i) => (
                <ActivityRow
                  key={e.id ?? i}
                  color={opColor(e.request?.operation, e.response?.status_code)}
                  text={e.request?.path ?? "—"}
                  op={e.request?.operation ?? "—"}
                  time={timeAgo(e.timestamp)}
                />
              ))
            ) : (
              <div className="py-6 text-center text-stone-500 text-sm">
                No activity yet. Operations will appear here.
              </div>
            )}
          </div>
        </div>
      </div>
    </>
  );
}


// ── Helper components ────────────────────────────────────────────────

function StatCard({ label, value, valueClass, sub }: { label: string; value: string; valueClass: string; sub: string }) {
  return (
    <div className="bg-glass backdrop-blur-[16px] border border-glass-border rounded-[20px] p-6 shadow-[0_8px_32px_rgba(0,0,0,.06)] hover:-translate-y-0.5 hover:shadow-[0_12px_40px_rgba(0,0,0,.08)] transition-all">
      <div className="text-[11px] font-bold uppercase tracking-[.7px] text-stone-500 mb-2.5">{label}</div>
      <div className={`text-[34px] font-extrabold leading-none tracking-tight ${valueClass}`}>{value}</div>
      <div className="text-xs text-stone-400 mt-2">{sub}</div>
    </div>
  );
}

function EngineRow({ name, path, description: _description }: { name: string; path: string; description?: string }) {
  return (
    <div className="flex items-center justify-between py-3.5 border-b border-stone-200/60 last:border-b-0">
      <div className="flex items-center gap-3">
        <div className="w-9 h-9 rounded-[10px] flex items-center justify-center bg-amber-500/12">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="w-[18px] h-[18px] text-amber-500">
            <path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 11-7.778 7.778 5.5 5.5 0 017.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4" />
          </svg>
        </div>
        <div>
          <div className="font-bold text-sm text-stone-800">{name}</div>
          <div className="text-xs text-stone-400 font-mono">{path}</div>
        </div>
      </div>
      <span className="text-[11px] font-bold px-3 py-1 rounded-full bg-green-500/12 text-green-700">Active</span>
    </div>
  );
}

function ActivityRow({ color, text, op, time }: { color: string; text: string; op: string; time: string }) {
  return (
    <div className="flex items-center gap-3.5 py-3 border-b border-white/4 last:border-b-0">
      <span className={`w-2.5 h-2.5 rounded-full shrink-0 ${color}`} />
      <span className="flex-1 text-[13px] text-amber-200/80">
        <code className="bg-white/6 text-amber-200/80 px-1.5 py-0.5 rounded text-xs">{text}</code>
        {" — "}{op}
      </span>
      <span className="text-[11px] text-stone-500 whitespace-nowrap">{time}</span>
    </div>
  );
}

function Sparkline({ buckets }: { buckets: number[] }) {
  const max = Math.max(...buckets, 1);
  return (
    <div className="flex items-end gap-1.5 h-12">
      {buckets.map((count, i) => {
        const height = Math.max((count / max) * 100, 4);
        return (
          <div
            key={i}
            className="flex-1 rounded-t bg-amber-500/60 hover:bg-amber-500 transition-colors"
            style={{ height: `${height}%` }}
            title={`${count} operations`}
          />
        );
      })}
    </div>
  );
}

// ── Utility functions ────────────────────────────────────────────────

function opColor(op?: string, status?: number): string {
  if (status && status >= 400) return "bg-red-500";
  switch (op) {
    case "read": return "bg-green-500";
    case "write": return "bg-amber-400";
    case "login": return "bg-blue-400";
    case "encrypt": case "decrypt": return "bg-purple-400";
    case "delete": return "bg-red-400";
    default: return "bg-stone-400";
  }
}

function timeAgo(ts: string): string {
  try {
    const diff = Date.now() - new Date(ts).getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return "just now";
    if (mins < 60) return `${mins} min ago`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    return `${Math.floor(hours / 24)}d ago`;
  } catch {
    return "—";
  }
}

function buildSparkline(entries: AuditEntry[], hours: number): number[] {
  const now = Date.now();
  const buckets = new Array(hours).fill(0);
  for (const e of entries) {
    try {
      const age = now - new Date(e.timestamp).getTime();
      const bucket = Math.floor(age / 3600000);
      if (bucket >= 0 && bucket < hours) {
        buckets[hours - 1 - bucket] += 1;
      }
    } catch {
      // skip malformed timestamps
    }
  }
  return buckets;
}
