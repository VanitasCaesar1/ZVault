import { useEffect, useState, useCallback } from "react";
import { vaultFetch } from "../lib/api";
import { Th, Td, Badge } from "../components/Table";

interface Policy {
  name: string;
  rules: number;
  builtin: boolean;
}

interface PolicyRule {
  path: string;
  capabilities: string[];
}

interface PolicyDetail {
  name: string;
  rules: PolicyRule[];
}

interface PolicyModalState {
  open: boolean;
  mode: "create" | "edit" | "view";
  name: string;
  rulesJson: string;
  loading: boolean;
  error: string;
}

const EMPTY_RULES = JSON.stringify(
  [{ path: "secret/data/*", capabilities: ["read", "list"] }],
  null, 2,
);

export function PoliciesPage() {
  const [policies, setPolicies] = useState<Policy[]>([]);
  const [loading, setLoading] = useState(true);
  const [modal, setModal] = useState<PolicyModalState>({
    open: false, mode: "create", name: "", rulesJson: EMPTY_RULES,
    loading: false, error: "",
  });

  const fetchPolicies = useCallback(() => {
    setLoading(true);
    vaultFetch<{ policies?: string[] }>("/v1/sys/policies")
      .then((data) => {
        setPolicies(
          (data.policies ?? []).map((name) => ({
            name,
            rules: name === "root" ? 1 : name === "default" ? 2 : 0,
            builtin: name === "root" || name === "default",
          }))
        );
      })
      .catch(() => setPolicies([]))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => { fetchPolicies(); }, [fetchPolicies]);

  const openCreate = () =>
    setModal({ open: true, mode: "create", name: "", rulesJson: EMPTY_RULES, loading: false, error: "" });

  const openEditOrView = (name: string, builtin: boolean) => {
    const mode = builtin ? "view" : "edit";
    setModal({ open: true, mode, name, rulesJson: "", loading: true, error: "" });
    vaultFetch<PolicyDetail>(`/v1/sys/policies/${name}`)
      .then((data) => {
        setModal((m) => ({
          ...m,
          rulesJson: JSON.stringify(data.rules ?? [], null, 2),
          loading: false,
        }));
      })
      .catch((e) => {
        setModal((m) => ({ ...m, rulesJson: "[]", loading: false, error: e.message }));
      });
  };

  const validateRules = (json: string): PolicyRule[] | null => {
    try {
      const parsed = JSON.parse(json);
      if (!Array.isArray(parsed)) return null;
      for (const rule of parsed) {
        if (typeof rule.path !== "string" || !Array.isArray(rule.capabilities)) return null;
      }
      return parsed;
    } catch {
      return null;
    }
  };

  const handleSave = async () => {
    const name = modal.name.trim();
    if (!name) {
      setModal((m) => ({ ...m, error: "Policy name is required" }));
      return;
    }
    if (!/^[a-zA-Z0-9_-]+$/.test(name)) {
      setModal((m) => ({ ...m, error: "Name may only contain letters, numbers, _ and -" }));
      return;
    }
    const rules = validateRules(modal.rulesJson);
    if (!rules) {
      setModal((m) => ({ ...m, error: 'Invalid JSON. Expected: [{"path": "...", "capabilities": ["read"]}]' }));
      return;
    }
    setModal((m) => ({ ...m, loading: true, error: "" }));
    try {
      await vaultFetch(`/v1/sys/policies/${name}`, {
        method: "POST",
        body: JSON.stringify({ rules }),
      });
      setModal((m) => ({ ...m, open: false }));
      fetchPolicies();
    } catch (e: any) {
      setModal((m) => ({ ...m, loading: false, error: e.message }));
    }
  };

  const handleDelete = async (name: string) => {
    if (!confirm(`Delete policy "${name}"?`)) return;
    try {
      await vaultFetch(`/v1/sys/policies/${name}`, { method: "DELETE" });
      fetchPolicies();
    } catch {
      // ignore
    }
  };

  const closeModal = () => setModal((m) => ({ ...m, open: false }));

  return (
    <>
      <div className="flex justify-between items-center mb-6">
        <p className="text-sm text-stone-500">Define path-based access rules for tokens and auth methods.</p>
        <button
          onClick={openCreate}
          className="px-4 py-2 rounded-full bg-amber-500 text-amber-900 text-[13px] font-semibold hover:bg-amber-600 transition-all cursor-pointer"
        >
          + New Policy
        </button>
      </div>

      <div className="bg-glass backdrop-blur-[16px] border border-glass-border rounded-[20px] shadow-[0_8px_32px_rgba(0,0,0,.06)] overflow-hidden">
        <table className="w-full text-[13px] border-collapse">
          <thead>
            <tr><Th>Name</Th><Th>Rules</Th><Th>Type</Th><Th>Actions</Th></tr>
          </thead>
          <tbody>
            {loading ? (
              <tr><td colSpan={4} className="px-[18px] py-4 text-center text-stone-400">Loading…</td></tr>
            ) : policies.length === 0 ? (
              <tr><td colSpan={4} className="px-[18px] py-4 text-center text-stone-400">No policies found</td></tr>
            ) : policies.map((p) => (
              <tr key={p.name} className="hover:bg-stone-50">
                <Td><strong>{p.name}</strong></Td>
                <Td>{p.rules} rule{p.rules !== 1 ? "s" : ""}</Td>
                <Td>
                  <Badge variant={p.builtin ? "warning" : "primary"}>
                    {p.builtin ? "Built-in" : "Custom"}
                  </Badge>
                </Td>
                <Td>
                  <div className="flex gap-1.5">
                    <button
                      onClick={() => openEditOrView(p.name, p.builtin)}
                      className="px-3 py-[5px] text-xs font-semibold rounded-full bg-glass border border-stone-300/40 text-stone-700 hover:bg-surface transition-all cursor-pointer"
                    >
                      {p.builtin ? "View" : "Edit"}
                    </button>
                    {!p.builtin && (
                      <button
                        onClick={() => handleDelete(p.name)}
                        className="px-3 py-[5px] text-xs font-semibold rounded-full border border-red-200 text-red-600 hover:bg-red-50 transition-all cursor-pointer"
                      >
                        Delete
                      </button>
                    )}
                  </div>
                </Td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Policy Modal */}
      {modal.open && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm" onClick={closeModal}>
          <div className="bg-white rounded-2xl shadow-2xl w-full max-w-lg mx-4 overflow-hidden" onClick={(e) => e.stopPropagation()}>
            <div className="px-6 py-4 border-b border-stone-200 flex items-center justify-between">
              <h3 className="text-base font-bold text-stone-800">
                {modal.mode === "create" ? "Create Policy" : modal.mode === "edit" ? `Edit: ${modal.name}` : `Policy: ${modal.name}`}
              </h3>
              <button onClick={closeModal} className="text-stone-400 hover:text-stone-600 text-xl leading-none cursor-pointer">&times;</button>
            </div>
            <div className="px-6 py-4 space-y-4">
              {modal.mode === "create" && (
                <div>
                  <label className="block text-xs font-semibold text-stone-500 mb-1.5">Policy Name</label>
                  <input
                    type="text" value={modal.name}
                    onChange={(e) => setModal((m) => ({ ...m, name: e.target.value, error: "" }))}
                    placeholder="e.g. app-readonly"
                    className="w-full px-3 py-2 text-sm border border-stone-300 rounded-lg focus:outline-none focus:border-amber-500"
                  />
                </div>
              )}
              <div>
                <label className="block text-xs font-semibold text-stone-500 mb-1.5">
                  Rules (JSON array)
                </label>
                <textarea
                  value={modal.rulesJson}
                  onChange={(e) => setModal((m) => ({ ...m, rulesJson: e.target.value, error: "" }))}
                  rows={10}
                  readOnly={modal.mode === "view"}
                  className="w-full px-3 py-2 text-sm font-mono border border-stone-300 rounded-lg focus:outline-none focus:border-amber-500 bg-stone-50 resize-none"
                />
                {modal.mode !== "view" && (
                  <p className="text-[11px] text-stone-400 mt-1">
                    Format: [{"{"}"path": "secret/data/*", "capabilities": ["read", "list"]{"}"}]
                  </p>
                )}
              </div>
              {modal.error && <p className="text-sm text-red-600">{modal.error}</p>}
            </div>
            <div className="px-6 py-3 border-t border-stone-200 flex justify-end gap-2">
              <button onClick={closeModal} className="px-4 py-2 text-sm rounded-lg text-stone-600 hover:bg-stone-100 cursor-pointer">
                {modal.mode === "view" ? "Close" : "Cancel"}
              </button>
              {modal.mode !== "view" && (
                <button
                  onClick={handleSave}
                  disabled={modal.loading}
                  className="px-4 py-2 text-sm rounded-lg bg-amber-500 text-amber-900 font-semibold hover:bg-amber-600 disabled:opacity-50 cursor-pointer"
                >
                  {modal.loading ? "Saving…" : "Save Policy"}
                </button>
              )}
            </div>
          </div>
        </div>
      )}
    </>
  );
}
