import { useEffect, useState, useCallback } from "react";
import { Card, Th, Td, Badge } from "../../components/Table";
import {
  listOrgs,
  listProjects,
  listAudit,
  type Organization,
  type Project,
  type AuditEntry,
} from "../../lib/cloud-api";

const actionColors: Record<string, string> = {
  "secret.created": "success",
  "secret.updated": "warning",
  "secret.deleted": "danger",
  "project.created": "info",
  "environment.created": "info",
  "token.created": "info",
  "token.revoked": "danger",
  "member.invited": "info",
  "member.removed": "danger",
};

export function CloudAuditPage() {
  const [orgs, setOrgs] = useState<Organization[]>([]);
  const [selectedOrg, setSelectedOrg] = useState<Organization | null>(null);
  const [projects, setProjects] = useState<Project[]>([]);
  const [selectedProject, setSelectedProject] = useState<Project | null>(null);
  const [entries, setEntries] = useState<AuditEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [offset, setOffset] = useState(0);
  const limit = 50;

  const fetchOrgs = useCallback(async () => {
    try {
      const data = await listOrgs();
      setOrgs(data);
      if (data.length > 0 && !selectedOrg) setSelectedOrg(data[0]);
    } catch (e: any) {
      setError(e.message);
    }
  }, [selectedOrg]);

  const fetchProjects = useCallback(async () => {
    if (!selectedOrg) return;
    try {
      const data = await listProjects(selectedOrg.id);
      setProjects(data);
      if (data.length > 0 && !selectedProject) setSelectedProject(data[0]);
    } catch (e: any) {
      setError(e.message);
    }
  }, [selectedOrg, selectedProject]);

  const fetchAudit = useCallback(async () => {
    if (!selectedOrg || !selectedProject) return;
    setLoading(true);
    try {
      const data = await listAudit(selectedOrg.id, selectedProject.id, limit, offset);
      setEntries(data);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }, [selectedOrg, selectedProject, offset]);

  useEffect(() => { fetchOrgs(); }, [fetchOrgs]);
  useEffect(() => { fetchProjects(); }, [fetchProjects]);
  useEffect(() => { fetchAudit(); }, [fetchAudit]);

  return (
    <>
      <div className="flex justify-between items-center mb-6">
        <div className="flex items-center gap-3">
          {orgs.length > 1 && (
            <select
              value={selectedOrg?.id ?? ""}
              onChange={(e) => {
                const org = orgs.find((o) => o.id === e.target.value);
                if (org) { setSelectedOrg(org); setSelectedProject(null); setOffset(0); }
              }}
              className="px-3 py-1.5 text-sm border border-stone-300/40 rounded-lg bg-glass focus:outline-none focus:border-amber-500"
            >
              {orgs.map((o) => (
                <option key={o.id} value={o.id}>{o.name}</option>
              ))}
            </select>
          )}
          {projects.length > 0 && (
            <select
              value={selectedProject?.id ?? ""}
              onChange={(e) => {
                const proj = projects.find((p) => p.id === e.target.value);
                if (proj) { setSelectedProject(proj); setOffset(0); }
              }}
              className="px-3 py-1.5 text-sm border border-stone-300/40 rounded-lg bg-glass focus:outline-none focus:border-amber-500"
            >
              {projects.map((p) => (
                <option key={p.id} value={p.id}>{p.name}</option>
              ))}
            </select>
          )}
        </div>
        <button
          onClick={() => fetchAudit()}
          className="px-4 py-2 rounded-full bg-glass border border-stone-300/40 text-stone-700 text-[13px] font-semibold hover:bg-surface transition-all cursor-pointer"
        >
          Refresh
        </button>
      </div>

      {error && <p className="text-sm text-red-600 mb-4">{error}</p>}

      <Card
        title="Audit Log"
        actions={<span className="text-xs text-stone-400">{entries.length} entries</span>}
      >
        <table className="w-full text-[13px] border-collapse">
          <thead>
            <tr><Th>Time</Th><Th>Action</Th><Th>Resource</Th><Th>Actor</Th><Th>Environment</Th><Th>Detail</Th></tr>
          </thead>
          <tbody>
            {loading ? (
              <tr><td colSpan={6} className="px-[18px] py-4 text-center text-stone-400">Loading…</td></tr>
            ) : entries.length === 0 ? (
              <tr><td colSpan={6} className="px-[18px] py-4 text-center text-stone-400">No audit entries yet.</td></tr>
            ) : entries.map((entry) => (
              <tr key={entry.id} className="hover:bg-stone-50">
                <Td>
                  <span className="text-stone-500 text-xs whitespace-nowrap">
                    {new Date(entry.created_at).toLocaleString()}
                  </span>
                </Td>
                <Td>
                  <Badge variant={actionColors[entry.action] ?? "muted"}>
                    {entry.action}
                  </Badge>
                </Td>
                <Td><span className="font-medium text-stone-700">{entry.resource}</span></Td>
                <Td>
                  <span className="text-stone-500">
                    {entry.actor_type === "service_token" ? "Service Token" : entry.actor_id?.slice(0, 8) ?? "—"}
                  </span>
                </Td>
                <Td>
                  {entry.env_slug ? (
                    <code className="bg-stone-100 text-stone-600 px-1.5 py-0.5 rounded text-xs">{entry.env_slug}</code>
                  ) : (
                    <span className="text-stone-400">—</span>
                  )}
                </Td>
                <Td>
                  <span className="text-stone-400 text-xs truncate max-w-[200px] block">
                    {Object.keys(entry.detail).length > 0 ? JSON.stringify(entry.detail) : "—"}
                  </span>
                </Td>
              </tr>
            ))}
          </tbody>
        </table>
      </Card>

      {/* Pagination */}
      {entries.length > 0 && (
        <div className="flex justify-center gap-3 mt-4">
          <button
            onClick={() => setOffset((o) => Math.max(0, o - limit))}
            disabled={offset === 0}
            className="px-4 py-2 text-xs font-semibold rounded-full bg-glass border border-stone-300/40 text-stone-700 hover:bg-surface disabled:opacity-40 cursor-pointer disabled:cursor-default transition-all"
          >
            ← Previous
          </button>
          <span className="text-xs text-stone-400 self-center">
            {offset + 1}–{offset + entries.length}
          </span>
          <button
            onClick={() => setOffset((o) => o + limit)}
            disabled={entries.length < limit}
            className="px-4 py-2 text-xs font-semibold rounded-full bg-glass border border-stone-300/40 text-stone-700 hover:bg-surface disabled:opacity-40 cursor-pointer disabled:cursor-default transition-all"
          >
            Next →
          </button>
        </div>
      )}
    </>
  );
}
