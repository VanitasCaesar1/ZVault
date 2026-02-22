import { useEffect, useState, useCallback } from "react";
import { useParams, useNavigate } from "react-router";
import { Card, Th, Td, Badge } from "../../components/Table";
import { Modal, FormField, ModalActions } from "./Projects";
import {
  getProject,
  listEnvironments,
  createEnvironment,
  listSecretKeys,
  getSecret,
  setSecret,
  deleteSecret,
  type Project,
  type Environment,
  type SecretKey,
} from "../../lib/cloud-api";

export function CloudProjectDetailPage() {
  const { orgId, projectId } = useParams<{ orgId: string; projectId: string }>();
  const navigate = useNavigate();
  const [project, setProject] = useState<Project | null>(null);
  const [envs, setEnvs] = useState<Environment[]>([]);
  const [activeEnv, setActiveEnv] = useState<string>("");
  const [secrets, setSecrets] = useState<SecretKey[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  // Modals
  const [showEnvModal, setShowEnvModal] = useState(false);
  const [envForm, setEnvForm] = useState({ name: "", slug: "" });
  const [showSecretModal, setShowSecretModal] = useState(false);
  const [secretMode, setSecretMode] = useState<"create" | "view">("create");
  const [secretForm, setSecretForm] = useState({ key: "", value: "", comment: "" });
  const [showDeleteConfirm, setShowDeleteConfirm] = useState<string | null>(null);
  const [formLoading, setFormLoading] = useState(false);
  const [formError, setFormError] = useState("");

  const fetchProject = useCallback(async () => {
    if (!orgId || !projectId) return;
    try {
      const p = await getProject(orgId, projectId);
      setProject(p);
    } catch (e: any) {
      setError(e.message);
    }
  }, [orgId, projectId]);

  const fetchEnvs = useCallback(async () => {
    if (!orgId || !projectId) return;
    try {
      const data = await listEnvironments(orgId, projectId);
      setEnvs(data);
      if (data.length > 0 && !activeEnv) {
        setActiveEnv(data[0].slug);
      }
    } catch (e: any) {
      setError(e.message);
    }
  }, [orgId, projectId, activeEnv]);

  const fetchSecrets = useCallback(async () => {
    if (!orgId || !projectId || !activeEnv) return;
    setLoading(true);
    try {
      const data = await listSecretKeys(orgId, projectId, activeEnv);
      setSecrets(data);
    } catch (e: any) {
      setError(e.message);
      setSecrets([]);
    } finally {
      setLoading(false);
    }
  }, [orgId, projectId, activeEnv]);

  useEffect(() => { fetchProject(); }, [fetchProject]);
  useEffect(() => { fetchEnvs(); }, [fetchEnvs]);
  useEffect(() => { fetchSecrets(); }, [fetchSecrets]);

  const handleCreateEnv = async () => {
    if (!orgId || !projectId) return;
    if (!envForm.name.trim()) { setFormError("Name is required"); return; }
    const slug = envForm.slug.trim() || envForm.name.trim().toLowerCase().replace(/\s+/g, "-").replace(/[^a-z0-9-]/g, "");
    setFormLoading(true);
    setFormError("");
    try {
      await createEnvironment(orgId, projectId, envForm.name.trim(), slug);
      await fetchEnvs();
      setShowEnvModal(false);
      setEnvForm({ name: "", slug: "" });
    } catch (e: any) {
      setFormError(e.message);
    } finally {
      setFormLoading(false);
    }
  };

  const openCreateSecret = () => {
    setSecretMode("create");
    setSecretForm({ key: "", value: "", comment: "" });
    setShowSecretModal(true);
    setFormError("");
  };

  const openViewSecret = async (key: string) => {
    if (!orgId || !projectId) return;
    setSecretMode("view");
    setSecretForm({ key, value: "", comment: "" });
    setShowSecretModal(true);
    setFormError("");
    setFormLoading(true);
    try {
      const s = await getSecret(orgId, projectId, key, activeEnv);
      setSecretForm({ key: s.key, value: s.value, comment: s.comment });
    } catch (e: any) {
      setFormError(e.message);
    } finally {
      setFormLoading(false);
    }
  };

  const handleSaveSecret = async () => {
    if (!orgId || !projectId) return;
    if (!secretForm.key.trim()) { setFormError("Key is required"); return; }
    setFormLoading(true);
    setFormError("");
    try {
      await setSecret(orgId, projectId, activeEnv, secretForm.key.trim(), secretForm.value, secretForm.comment);
      setShowSecretModal(false);
      await fetchSecrets();
    } catch (e: any) {
      setFormError(e.message);
    } finally {
      setFormLoading(false);
    }
  };

  const handleDeleteSecret = async (key: string) => {
    if (!orgId || !projectId) return;
    try {
      await deleteSecret(orgId, projectId, key, activeEnv);
      setShowDeleteConfirm(null);
      await fetchSecrets();
    } catch (e: any) {
      setError(e.message);
    }
  };

  const envColor = (slug: string) => {
    if (slug === "production") return "bg-red-500/12 text-red-700";
    if (slug === "staging") return "bg-amber-500/12 text-amber-700";
    return "bg-green-500/12 text-green-700";
  };

  return (
    <>
      {/* Breadcrumb */}
      <div className="flex items-center gap-2 text-sm text-stone-500 mb-4">
        <button onClick={() => navigate("/cloud/projects")} className="hover:text-amber-600 cursor-pointer">Projects</button>
        <span>/</span>
        <span className="text-stone-800 font-semibold">{project?.name ?? "…"}</span>
      </div>

      {error && <p className="text-sm text-red-600 mb-4">{error}</p>}

      {/* Environment tabs */}
      <div className="flex items-center gap-2 mb-6 flex-wrap">
        {envs.map((env) => (
          <button
            key={env.slug}
            onClick={() => setActiveEnv(env.slug)}
            className={`px-4 py-1.5 rounded-full text-sm font-semibold transition-all cursor-pointer ${
              activeEnv === env.slug
                ? "bg-amber-500 text-amber-900 shadow-sm"
                : "bg-glass border border-stone-300/40 text-stone-600 hover:bg-surface"
            }`}
          >
            {env.name}
          </button>
        ))}
        <button
          onClick={() => { setShowEnvModal(true); setFormError(""); }}
          className="px-3 py-1.5 rounded-full text-sm font-semibold text-stone-400 hover:text-stone-600 border border-dashed border-stone-300 hover:border-stone-400 transition-all cursor-pointer"
        >
          + Add
        </button>
      </div>

      {/* Secrets table */}
      <Card
        title={`Secrets — ${activeEnv}`}
        actions={
          <div className="flex items-center gap-2">
            <Badge variant={envColor(activeEnv).includes("red") ? "danger" : envColor(activeEnv).includes("amber") ? "warning" : "success"}>
              {activeEnv}
            </Badge>
            <button
              onClick={openCreateSecret}
              className="px-3.5 py-[5px] text-xs font-semibold rounded-full bg-amber-500 text-amber-900 hover:bg-amber-600 transition-all cursor-pointer"
            >
              + Add Secret
            </button>
          </div>
        }
      >
        <table className="w-full text-[13px] border-collapse">
          <thead>
            <tr><Th>Key</Th><Th>Version</Th><Th>Comment</Th><Th>Updated</Th><Th>Actions</Th></tr>
          </thead>
          <tbody>
            {loading ? (
              <tr><td colSpan={5} className="px-[18px] py-4 text-center text-stone-400">Loading…</td></tr>
            ) : secrets.length === 0 ? (
              <tr><td colSpan={5} className="px-[18px] py-4 text-center text-stone-400">No secrets in {activeEnv}. Add one to get started.</td></tr>
            ) : secrets.map((s) => (
              <tr key={s.key} className="hover:bg-stone-50">
                <Td><code className="bg-stone-100 text-stone-600 px-1.5 py-0.5 rounded text-xs">{s.key}</code></Td>
                <Td><Badge variant="primary">v{s.version}</Badge></Td>
                <Td><span className="text-stone-500">{s.comment || "—"}</span></Td>
                <Td>{new Date(s.updated_at).toLocaleDateString()}</Td>
                <Td>
                  <div className="flex gap-1.5">
                    <button onClick={() => openViewSecret(s.key)} className="px-3 py-[5px] text-xs font-semibold rounded-full bg-glass border border-stone-300/40 text-stone-700 hover:bg-surface transition-all cursor-pointer">View</button>
                    <button onClick={() => setShowDeleteConfirm(s.key)} className="px-3 py-[5px] text-xs font-semibold rounded-full bg-red-500/10 text-red-700 hover:bg-red-500/20 transition-all cursor-pointer">Delete</button>
                  </div>
                </Td>
              </tr>
            ))}
          </tbody>
        </table>
      </Card>

      {/* Create Environment Modal */}
      {showEnvModal && (
        <Modal title="Create Environment" onClose={() => setShowEnvModal(false)}>
          <FormField label="Environment Name" value={envForm.name} onChange={(v) => setEnvForm((f) => ({ ...f, name: v }))} placeholder="e.g. Preview" />
          <FormField label="Slug (optional)" value={envForm.slug} onChange={(v) => setEnvForm((f) => ({ ...f, slug: v }))} placeholder="preview" />
          {formError && <p className="text-sm text-red-600">{formError}</p>}
          <ModalActions onCancel={() => setShowEnvModal(false)} onSubmit={handleCreateEnv} loading={formLoading} submitLabel="Create" />
        </Modal>
      )}

      {/* Secret Modal (Create / View+Edit) */}
      {showSecretModal && (
        <Modal title={secretMode === "create" ? "Add Secret" : `Secret: ${secretForm.key}`} onClose={() => setShowSecretModal(false)}>
          {secretMode === "create" && (
            <FormField label="Key" value={secretForm.key} onChange={(v) => setSecretForm((f) => ({ ...f, key: v }))} placeholder="DATABASE_URL" />
          )}
          <div>
            <label className="block text-xs font-semibold text-stone-500 mb-1.5">Value</label>
            <textarea
              value={secretForm.value}
              onChange={(e) => setSecretForm((f) => ({ ...f, value: e.target.value }))}
              rows={4}
              placeholder="secret-value-here"
              className="w-full px-3 py-2 text-sm font-mono border border-stone-300 rounded-lg focus:outline-none focus:border-amber-500 bg-stone-50 resize-none"
            />
          </div>
          <FormField label="Comment (optional)" value={secretForm.comment} onChange={(v) => setSecretForm((f) => ({ ...f, comment: v }))} placeholder="Production database connection string" />
          {formError && <p className="text-sm text-red-600">{formError}</p>}
          <div className="flex justify-end gap-2 pt-2">
            <button onClick={() => setShowSecretModal(false)} className="px-4 py-2 text-sm rounded-lg text-stone-600 hover:bg-stone-100 cursor-pointer">Cancel</button>
            <button onClick={handleSaveSecret} disabled={formLoading} className="px-4 py-2 text-sm rounded-lg bg-amber-500 text-amber-900 font-semibold hover:bg-amber-600 disabled:opacity-50 cursor-pointer">
              {formLoading ? "Saving…" : "Save Secret"}
            </button>
          </div>
        </Modal>
      )}

      {/* Delete Confirmation */}
      {showDeleteConfirm && (
        <Modal title="Delete Secret" onClose={() => setShowDeleteConfirm(null)}>
          <p className="text-sm text-stone-600">
            Are you sure you want to delete <code className="bg-stone-100 px-1.5 py-0.5 rounded text-xs font-mono">{showDeleteConfirm}</code> from <strong>{activeEnv}</strong>? This cannot be undone.
          </p>
          <div className="flex justify-end gap-2 pt-2">
            <button onClick={() => setShowDeleteConfirm(null)} className="px-4 py-2 text-sm rounded-lg text-stone-600 hover:bg-stone-100 cursor-pointer">Cancel</button>
            <button onClick={() => handleDeleteSecret(showDeleteConfirm)} className="px-4 py-2 text-sm rounded-lg bg-red-500 text-white font-semibold hover:bg-red-600 cursor-pointer">Delete</button>
          </div>
        </Modal>
      )}
    </>
  );
}
