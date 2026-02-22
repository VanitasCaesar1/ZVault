import { useEffect, useState, useCallback } from "react";
import { Card, Th, Td, Badge } from "../../components/Table";
import { Modal, FormField, ModalActions } from "./Projects";
import {
  listOrgs,
  listProjects,
  listEnvironments,
  listServiceTokens,
  createServiceToken,
  revokeServiceToken,
  type Organization,
  type Project,
  type Environment,
  type ServiceToken,
} from "../../lib/cloud-api";

export function CloudServiceTokensPage() {
  const [, setOrgs] = useState<Organization[]>([]);
  const [selectedOrg, setSelectedOrg] = useState<Organization | null>(null);
  const [projects, setProjects] = useState<Project[]>([]);
  const [selectedProject, setSelectedProject] = useState<Project | null>(null);
  const [envs, setEnvs] = useState<Environment[]>([]);
  const [tokens, setTokens] = useState<ServiceToken[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  // Create modal
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [tokenForm, setTokenForm] = useState({ name: "", envId: "", expiresInDays: "" });
  const [formLoading, setFormLoading] = useState(false);
  const [formError, setFormError] = useState("");

  // Reveal token (shown once after creation)
  const [revealedToken, setRevealedToken] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const fetchOrgs = useCallback(async () => {
    try {
      const data = await listOrgs();
      setOrgs(data);
      if (data.length > 0 && !selectedOrg) setSelectedOrg(data[0]);
    } catch (e: any) { setError(e.message); }
  }, [selectedOrg]);

  const fetchProjects = useCallback(async () => {
    if (!selectedOrg) return;
    try {
      const data = await listProjects(selectedOrg.id);
      setProjects(data);
      if (data.length > 0 && !selectedProject) setSelectedProject(data[0]);
    } catch (e: any) { setError(e.message); }
  }, [selectedOrg, selectedProject]);

  const fetchEnvs = useCallback(async () => {
    if (!selectedOrg || !selectedProject) return;
    try {
      const data = await listEnvironments(selectedOrg.id, selectedProject.id);
      setEnvs(data);
    } catch { /* ignore */ }
  }, [selectedOrg, selectedProject]);

  const fetchTokens = useCallback(async () => {
    if (!selectedOrg || !selectedProject) return;
    setLoading(true);
    try {
      const data = await listServiceTokens(selectedOrg.id, selectedProject.id);
      setTokens(data);
    } catch (e: any) { setError(e.message); }
    finally { setLoading(false); }
  }, [selectedOrg, selectedProject]);

  useEffect(() => { fetchOrgs(); }, [fetchOrgs]);
  useEffect(() => { fetchProjects(); }, [fetchProjects]);
  useEffect(() => { fetchEnvs(); }, [fetchEnvs]);
  useEffect(() => { fetchTokens(); }, [fetchTokens]);

  const handleCreate = async () => {
    if (!selectedOrg || !selectedProject) return;
    if (!tokenForm.name.trim()) { setFormError("Name is required"); return; }
    setFormLoading(true);
    setFormError("");
    try {
      const expires = tokenForm.expiresInDays ? parseInt(tokenForm.expiresInDays, 10) : undefined;
      const result = await createServiceToken(
        selectedOrg.id, selectedProject.id, tokenForm.name.trim(),
        tokenForm.envId || undefined, ["read", "write"], expires
      );
      setRevealedToken(result.token);
      setShowCreateModal(false);
      setTokenForm({ name: "", envId: "", expiresInDays: "" });
      await fetchTokens();
    } catch (e: any) { setFormError(e.message); }
    finally { setFormLoading(false); }
  };

  const handleRevoke = async (tokenId: string) => {
    if (!selectedOrg || !selectedProject) return;
    try {
      await revokeServiceToken(selectedOrg.id, selectedProject.id, tokenId);
      await fetchTokens();
    } catch (e: any) { setError(e.message); }
  };

  const copyToken = () => {
    if (revealedToken) {
      navigator.clipboard.writeText(revealedToken);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <>
      <div className="flex justify-between items-center mb-6">
        <div className="flex items-center gap-3">
          <p className="text-sm text-stone-500">Service tokens for CI/CD and production runtimes.</p>
          {projects.length > 1 && (
            <select
              value={selectedProject?.id ?? ""}
              onChange={(e) => { const p = projects.find((x) => x.id === e.target.value); if (p) setSelectedProject(p); }}
              className="px-3 py-1.5 text-sm border border-stone-300/40 rounded-lg bg-glass focus:outline-none focus:border-amber-500"
            >
              {projects.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
            </select>
          )}
        </div>
        {selectedProject && (
          <button
            onClick={() => { setShowCreateModal(true); setFormError(""); }}
            className="px-4 py-2 rounded-full bg-amber-500 text-amber-900 text-[13px] font-semibold hover:bg-amber-600 transition-all cursor-pointer"
          >
            + Create Token
          </button>
        )}
      </div>

      {error && <p className="text-sm text-red-600 mb-4">{error}</p>}

      {/* Revealed token banner */}
      {revealedToken && (
        <div className="bg-green-50 border border-green-200 rounded-2xl p-4 mb-6">
          <p className="text-sm font-semibold text-green-800 mb-2">Token created — copy it now, it won't be shown again.</p>
          <div className="flex items-center gap-2">
            <code className="flex-1 bg-white border border-green-200 rounded-lg px-3 py-2 text-sm font-mono text-green-900 break-all">{revealedToken}</code>
            <button onClick={copyToken} className="px-3 py-2 text-xs font-semibold rounded-lg bg-green-600 text-white hover:bg-green-700 cursor-pointer shrink-0">
              {copied ? "Copied!" : "Copy"}
            </button>
          </div>
          <button onClick={() => setRevealedToken(null)} className="text-xs text-green-600 hover:text-green-800 mt-2 cursor-pointer">Dismiss</button>
        </div>
      )}

      <Card title={`Service Tokens${selectedProject ? ` — ${selectedProject.name}` : ""}`} actions={<span className="text-xs text-stone-400">{tokens.filter((t) => !t.revoked_at).length} active</span>}>
        <table className="w-full text-[13px] border-collapse">
          <thead>
            <tr><Th>Name</Th><Th>Prefix</Th><Th>Scope</Th><Th>Last Used</Th><Th>Expires</Th><Th>Status</Th><Th>Actions</Th></tr>
          </thead>
          <tbody>
            {loading ? (
              <tr><td colSpan={7} className="px-[18px] py-4 text-center text-stone-400">Loading…</td></tr>
            ) : tokens.length === 0 ? (
              <tr><td colSpan={7} className="px-[18px] py-4 text-center text-stone-400">No service tokens yet.</td></tr>
            ) : tokens.map((t) => (
              <tr key={t.id} className="hover:bg-stone-50">
                <Td><span className="font-medium text-stone-800">{t.name}</span></Td>
                <Td><code className="bg-stone-100 text-stone-600 px-1.5 py-0.5 rounded text-xs">{t.token_prefix}</code></Td>
                <Td>{t.environment_id ? envs.find((e) => e.id === t.environment_id)?.name ?? "Scoped" : "All envs"}</Td>
                <Td>{t.last_used_at ? new Date(t.last_used_at).toLocaleDateString() : "Never"}</Td>
                <Td>{t.expires_at ? new Date(t.expires_at).toLocaleDateString() : "Never"}</Td>
                <Td>{t.revoked_at ? <Badge variant="danger">Revoked</Badge> : <Badge variant="success">Active</Badge>}</Td>
                <Td>
                  {!t.revoked_at && (
                    <button onClick={() => handleRevoke(t.id)} className="px-3 py-[5px] text-xs font-semibold rounded-full bg-red-500/10 text-red-700 hover:bg-red-500/20 transition-all cursor-pointer">Revoke</button>
                  )}
                </Td>
              </tr>
            ))}
          </tbody>
        </table>
      </Card>

      {showCreateModal && (
        <Modal title="Create Service Token" onClose={() => setShowCreateModal(false)}>
          <FormField label="Token Name" value={tokenForm.name} onChange={(v) => setTokenForm((f) => ({ ...f, name: v }))} placeholder="railway-deploy" />
          <div>
            <label className="block text-xs font-semibold text-stone-500 mb-1.5">Environment Scope (optional)</label>
            <select
              value={tokenForm.envId}
              onChange={(e) => setTokenForm((f) => ({ ...f, envId: e.target.value }))}
              className="w-full px-3 py-2 text-sm border border-stone-300 rounded-lg focus:outline-none focus:border-amber-500"
            >
              <option value="">All environments</option>
              {envs.map((e) => <option key={e.id} value={e.id}>{e.name}</option>)}
            </select>
          </div>
          <FormField label="Expires in (days, optional)" value={tokenForm.expiresInDays} onChange={(v) => setTokenForm((f) => ({ ...f, expiresInDays: v }))} placeholder="90" type="number" />
          {formError && <p className="text-sm text-red-600">{formError}</p>}
          <ModalActions onCancel={() => setShowCreateModal(false)} onSubmit={handleCreate} loading={formLoading} submitLabel="Create Token" />
        </Modal>
      )}
    </>
  );
}
