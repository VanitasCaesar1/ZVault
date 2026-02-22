import { useEffect, useState, useCallback } from "react";
import { useNavigate } from "react-router";
import { Card, Th, Td, Badge } from "../../components/Table";
import {
  listOrgs,
  listProjects,
  createOrg,
  createProject,
  type Organization,
  type Project,
} from "../../lib/cloud-api";

export function CloudProjectsPage() {
  const navigate = useNavigate();
  const [orgs, setOrgs] = useState<Organization[]>([]);
  const [selectedOrg, setSelectedOrg] = useState<Organization | null>(null);
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  // Modals
  const [showOrgModal, setShowOrgModal] = useState(false);
  const [showProjectModal, setShowProjectModal] = useState(false);
  const [orgForm, setOrgForm] = useState({ name: "", slug: "" });
  const [projectForm, setProjectForm] = useState({ name: "", slug: "", description: "" });
  const [formLoading, setFormLoading] = useState(false);
  const [formError, setFormError] = useState("");

  const fetchOrgs = useCallback(async () => {
    try {
      const data = await listOrgs();
      setOrgs(data);
      if (data.length > 0 && !selectedOrg) {
        setSelectedOrg(data[0]);
      }
    } catch (e: any) {
      setError(e.message);
    }
  }, [selectedOrg]);

  const fetchProjects = useCallback(async () => {
    if (!selectedOrg) return;
    setLoading(true);
    try {
      const data = await listProjects(selectedOrg.id);
      setProjects(data);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }, [selectedOrg]);

  useEffect(() => { fetchOrgs(); }, [fetchOrgs]);
  useEffect(() => { fetchProjects(); }, [fetchProjects]);

  const handleCreateOrg = async () => {
    if (!orgForm.name.trim()) { setFormError("Name is required"); return; }
    const slug = orgForm.slug.trim() || orgForm.name.trim().toLowerCase().replace(/\s+/g, "-").replace(/[^a-z0-9-]/g, "");
    setFormLoading(true);
    setFormError("");
    try {
      const org = await createOrg(orgForm.name.trim(), slug);
      setOrgs((prev) => [org, ...prev]);
      setSelectedOrg(org);
      setShowOrgModal(false);
      setOrgForm({ name: "", slug: "" });
    } catch (e: any) {
      setFormError(e.message);
    } finally {
      setFormLoading(false);
    }
  };

  const handleCreateProject = async () => {
    if (!selectedOrg) return;
    if (!projectForm.name.trim()) { setFormError("Name is required"); return; }
    const slug = projectForm.slug.trim() || projectForm.name.trim().toLowerCase().replace(/\s+/g, "-").replace(/[^a-z0-9-]/g, "");
    setFormLoading(true);
    setFormError("");
    try {
      const project = await createProject(selectedOrg.id, projectForm.name.trim(), slug, projectForm.description.trim());
      setProjects((prev) => [project, ...prev]);
      setShowProjectModal(false);
      setProjectForm({ name: "", slug: "", description: "" });
    } catch (e: any) {
      setFormError(e.message);
    } finally {
      setFormLoading(false);
    }
  };

  return (
    <>
      <div className="flex justify-between items-center mb-6">
        <div className="flex items-center gap-3">
          {orgs.length > 1 && (
            <select
              value={selectedOrg?.id ?? ""}
              onChange={(e) => {
                const org = orgs.find((o) => o.id === e.target.value);
                if (org) setSelectedOrg(org);
              }}
              className="px-3 py-1.5 text-sm border border-stone-300/40 rounded-lg bg-glass focus:outline-none focus:border-amber-500"
            >
              {orgs.map((o) => (
                <option key={o.id} value={o.id}>{o.name}</option>
              ))}
            </select>
          )}
          {selectedOrg && (
            <Badge variant="info">{selectedOrg.tier}</Badge>
          )}
        </div>
        <div className="flex gap-2">
          <button
            onClick={() => { setShowOrgModal(true); setFormError(""); }}
            className="px-4 py-2 rounded-full bg-glass border border-stone-300/40 text-stone-700 text-[13px] font-semibold hover:bg-surface transition-all cursor-pointer"
          >
            + New Org
          </button>
          {selectedOrg && (
            <button
              onClick={() => { setShowProjectModal(true); setFormError(""); }}
              className="px-4 py-2 rounded-full bg-amber-500 text-amber-900 text-[13px] font-semibold hover:bg-amber-600 transition-all cursor-pointer"
            >
              + New Project
            </button>
          )}
        </div>
      </div>

      {error && <p className="text-sm text-red-600 mb-4">{error}</p>}

      {orgs.length === 0 && !loading ? (
        <EmptyState
          title="No organizations yet"
          description="Create your first organization to start managing secrets in the cloud."
          action={() => { setShowOrgModal(true); setFormError(""); }}
          actionLabel="Create Organization"
        />
      ) : (
        <Card
          title={`Projects${selectedOrg ? ` — ${selectedOrg.name}` : ""}`}
          actions={
            <span className="text-xs text-stone-400">{projects.length} project{projects.length !== 1 ? "s" : ""}</span>
          }
        >
          <table className="w-full text-[13px] border-collapse">
            <thead>
              <tr><Th>Name</Th><Th>Slug</Th><Th>Description</Th><Th>Created</Th><Th>Actions</Th></tr>
            </thead>
            <tbody>
              {loading ? (
                <tr><td colSpan={5} className="px-[18px] py-4 text-center text-stone-400">Loading…</td></tr>
              ) : projects.length === 0 ? (
                <tr><td colSpan={5} className="px-[18px] py-4 text-center text-stone-400">No projects yet. Create one to get started.</td></tr>
              ) : projects.map((p) => (
                <tr key={p.id} className="hover:bg-stone-50 cursor-pointer" onClick={() => navigate(`/cloud/projects/${selectedOrg!.id}/${p.id}`)}>
                  <Td><span className="font-semibold text-stone-800">{p.name}</span></Td>
                  <Td><code className="bg-stone-100 text-stone-600 px-1.5 py-0.5 rounded text-xs">{p.slug}</code></Td>
                  <Td><span className="text-stone-500">{p.description || "—"}</span></Td>
                  <Td>{new Date(p.created_at).toLocaleDateString()}</Td>
                  <Td>
                    <button
                      onClick={(e) => { e.stopPropagation(); navigate(`/cloud/projects/${selectedOrg!.id}/${p.id}`); }}
                      className="px-3 py-[5px] text-xs font-semibold rounded-full bg-glass border border-stone-300/40 text-stone-700 hover:bg-surface transition-all cursor-pointer"
                    >
                      Open
                    </button>
                  </Td>
                </tr>
              ))}
            </tbody>
          </table>
        </Card>
      )}

      {/* Create Org Modal */}
      {showOrgModal && (
        <Modal title="Create Organization" onClose={() => setShowOrgModal(false)}>
          <FormField label="Organization Name" value={orgForm.name} onChange={(v) => setOrgForm((f) => ({ ...f, name: v }))} placeholder="My Company" />
          <FormField label="Slug (optional)" value={orgForm.slug} onChange={(v) => setOrgForm((f) => ({ ...f, slug: v }))} placeholder="my-company" />
          {formError && <p className="text-sm text-red-600">{formError}</p>}
          <ModalActions onCancel={() => setShowOrgModal(false)} onSubmit={handleCreateOrg} loading={formLoading} submitLabel="Create" />
        </Modal>
      )}

      {/* Create Project Modal */}
      {showProjectModal && (
        <Modal title="Create Project" onClose={() => setShowProjectModal(false)}>
          <FormField label="Project Name" value={projectForm.name} onChange={(v) => setProjectForm((f) => ({ ...f, name: v }))} placeholder="my-saas" />
          <FormField label="Slug (optional)" value={projectForm.slug} onChange={(v) => setProjectForm((f) => ({ ...f, slug: v }))} placeholder="my-saas" />
          <FormField label="Description" value={projectForm.description} onChange={(v) => setProjectForm((f) => ({ ...f, description: v }))} placeholder="Production SaaS app" />
          <p className="text-xs text-stone-400">3 default environments (development, staging, production) will be created.</p>
          {formError && <p className="text-sm text-red-600">{formError}</p>}
          <ModalActions onCancel={() => setShowProjectModal(false)} onSubmit={handleCreateProject} loading={formLoading} submitLabel="Create Project" />
        </Modal>
      )}
    </>
  );
}

// ── Shared UI helpers ────────────────────────────────────────────────

function EmptyState({ title, description, action, actionLabel }: { title: string; description: string; action: () => void; actionLabel: string }) {
  return (
    <div className="bg-glass backdrop-blur-[16px] border border-glass-border rounded-[20px] p-12 text-center">
      <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-amber-500/12 flex items-center justify-center">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="w-8 h-8 text-amber-500">
          <path d="M12 2v4m0 12v4M2 12h4m12 0h4" /><circle cx="12" cy="12" r="3" />
        </svg>
      </div>
      <h3 className="text-lg font-bold text-stone-800 mb-2">{title}</h3>
      <p className="text-sm text-stone-500 mb-6 max-w-md mx-auto">{description}</p>
      <button onClick={action} className="px-5 py-2.5 rounded-full bg-amber-500 text-amber-900 text-sm font-semibold hover:bg-amber-600 transition-all cursor-pointer">
        {actionLabel}
      </button>
    </div>
  );
}

export function Modal({ title, onClose, children }: { title: string; onClose: () => void; children: React.ReactNode }) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm" onClick={onClose}>
      <div className="bg-white rounded-2xl shadow-2xl w-full max-w-lg mx-4 overflow-hidden" onClick={(e) => e.stopPropagation()}>
        <div className="px-6 py-4 border-b border-stone-200 flex items-center justify-between">
          <h3 className="text-base font-bold text-stone-800">{title}</h3>
          <button onClick={onClose} className="text-stone-400 hover:text-stone-600 text-xl leading-none cursor-pointer">&times;</button>
        </div>
        <div className="px-6 py-4 space-y-4">{children}</div>
      </div>
    </div>
  );
}

export function FormField({ label, value, onChange, placeholder, type = "text" }: { label: string; value: string; onChange: (v: string) => void; placeholder?: string; type?: string }) {
  return (
    <div>
      <label className="block text-xs font-semibold text-stone-500 mb-1.5">{label}</label>
      <input
        type={type} value={value} onChange={(e) => onChange(e.target.value)} placeholder={placeholder}
        className="w-full px-3 py-2 text-sm border border-stone-300 rounded-lg focus:outline-none focus:border-amber-500"
      />
    </div>
  );
}

export function ModalActions({ onCancel, onSubmit, loading, submitLabel }: { onCancel: () => void; onSubmit: () => void; loading: boolean; submitLabel: string }) {
  return (
    <div className="flex justify-end gap-2 pt-2">
      <button onClick={onCancel} className="px-4 py-2 text-sm rounded-lg text-stone-600 hover:bg-stone-100 cursor-pointer">Cancel</button>
      <button onClick={onSubmit} disabled={loading} className="px-4 py-2 text-sm rounded-lg bg-amber-500 text-amber-900 font-semibold hover:bg-amber-600 disabled:opacity-50 cursor-pointer">
        {loading ? "Creating…" : submitLabel}
      </button>
    </div>
  );
}
