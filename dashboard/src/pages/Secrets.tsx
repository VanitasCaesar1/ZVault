import { useEffect, useState, useCallback } from "react";
import { vaultFetch } from "../lib/api";
import { Card } from "../components/Table";
import { Th, Td, Badge } from "../components/Table";

interface SecretEntry {
  key: string;
  version: number;
  updated: string;
}

interface SecretModalState {
  open: boolean;
  mode: "create" | "view";
  path: string;
  data: string;
  loading: boolean;
  error: string;
}

export function SecretsPage() {
  const [secrets, setSecrets] = useState<SecretEntry[]>([]);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(true);
  const [modal, setModal] = useState<SecretModalState>({
    open: false, mode: "create", path: "", data: '{\n  "key": "value"\n}',
    loading: false, error: "",
  });

  const fetchSecrets = useCallback(() => {
    setLoading(true);
    vaultFetch<{ data?: { keys?: string[] }; keys?: string[] }>("/v1/secret/list/")
      .then((data) => {
        const keys = data.data?.keys ?? data.keys ?? [];
        setSecrets(keys.map((k) => ({ key: k, version: 1, updated: "—" })));
      })
      .catch(() => setSecrets([]))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => { fetchSecrets(); }, [fetchSecrets]);

  const filtered = secrets.filter((s) =>
    s.key.toLowerCase().includes(search.toLowerCase())
  );

  const openCreate = () =>
    setModal({ open: true, mode: "create", path: "", data: '{\n  "key": "value"\n}', loading: false, error: "" });

  const openView = (key: string) => {
    setModal({ open: true, mode: "view", path: key, data: "", loading: true, error: "" });
    vaultFetch<{ data?: Record<string, unknown> }>(`/v1/secret/data/${key}`)
      .then((res) => {
        setModal((m) => ({
          ...m,
          data: JSON.stringify(res.data ?? {}, null, 2),
          loading: false,
        }));
      })
      .catch((e) => {
        setModal((m) => ({ ...m, data: "", loading: false, error: e.message }));
      });
  };

  const handleSave = async () => {
    if (!modal.path.trim()) {
      setModal((m) => ({ ...m, error: "Path is required" }));
      return;
    }
    try {
      JSON.parse(modal.data);
    } catch {
      setModal((m) => ({ ...m, error: "Invalid JSON" }));
      return;
    }
    setModal((m) => ({ ...m, loading: true, error: "" }));
    try {
      await vaultFetch(`/v1/secret/data/${modal.path.trim()}`, {
        method: "POST",
        body: modal.data,
      });
      setModal((m) => ({ ...m, open: false }));
      fetchSecrets();
    } catch (e: any) {
      setModal((m) => ({ ...m, loading: false, error: e.message }));
    }
  };

  const closeModal = () => setModal((m) => ({ ...m, open: false }));

  return (
    <>
      <div className="flex justify-between items-center mb-6">
        <p className="text-sm text-stone-500">
          Browse and manage secrets across all mounted KV engines.
        </p>
        <button
          onClick={openCreate}
          className="px-4 py-2 rounded-full bg-amber-500 text-amber-900 text-[13px] font-semibold hover:bg-amber-600 transition-all cursor-pointer"
        >
          + New Secret
        </button>
      </div>

      <Card
        title="secret/ (KV v2)"
        actions={
          <input
            type="text" value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search secrets..."
            className="w-60 px-3.5 py-[7px] text-[13px] border border-stone-300/40 rounded-full bg-glass focus:outline-none focus:border-amber-500"
          />
        }
      >
        <table className="w-full text-[13px] border-collapse">
          <thead>
            <tr><Th>Path</Th><Th>Version</Th><Th>Last Modified</Th><Th>Actions</Th></tr>
          </thead>
          <tbody>
            {loading ? (
              <tr><td colSpan={4} className="px-[18px] py-4 text-center text-stone-400">Loading…</td></tr>
            ) : filtered.length === 0 ? (
              <tr><td colSpan={4} className="px-[18px] py-4 text-center text-stone-400">No secrets found</td></tr>
            ) : filtered.map((s) => (
              <tr key={s.key} className="hover:bg-stone-50">
                <Td><code className="bg-stone-100 text-stone-600 px-1.5 py-0.5 rounded text-xs">{s.key}</code></Td>
                <Td><Badge variant="primary">v{s.version}</Badge></Td>
                <Td>{s.updated}</Td>
                <Td>
                  <button
                    onClick={() => openView(s.key)}
                    className="px-3 py-[5px] text-xs font-semibold rounded-full bg-glass border border-stone-300/40 text-stone-700 hover:bg-surface transition-all cursor-pointer"
                  >
                    View
                  </button>
                </Td>
              </tr>
            ))}
          </tbody>
        </table>
      </Card>

      {/* Secret Modal */}
      {modal.open && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm" onClick={closeModal}>
          <div className="bg-white rounded-2xl shadow-2xl w-full max-w-lg mx-4 overflow-hidden" onClick={(e) => e.stopPropagation()}>
            <div className="px-6 py-4 border-b border-stone-200 flex items-center justify-between">
              <h3 className="text-base font-bold text-stone-800">
                {modal.mode === "create" ? "Create Secret" : `Secret: ${modal.path}`}
              </h3>
              <button onClick={closeModal} className="text-stone-400 hover:text-stone-600 text-xl leading-none cursor-pointer">&times;</button>
            </div>
            <div className="px-6 py-4 space-y-4">
              {modal.mode === "create" && (
                <div>
                  <label className="block text-xs font-semibold text-stone-500 mb-1.5">Path</label>
                  <input
                    type="text" value={modal.path}
                    onChange={(e) => setModal((m) => ({ ...m, path: e.target.value, error: "" }))}
                    placeholder="e.g. prod/database"
                    className="w-full px-3 py-2 text-sm border border-stone-300 rounded-lg focus:outline-none focus:border-amber-500"
                  />
                </div>
              )}
              <div>
                <label className="block text-xs font-semibold text-stone-500 mb-1.5">
                  Data (JSON)
                </label>
                <textarea
                  value={modal.data}
                  onChange={(e) => setModal((m) => ({ ...m, data: e.target.value, error: "" }))}
                  rows={8}
                  readOnly={modal.mode === "view"}
                  className="w-full px-3 py-2 text-sm font-mono border border-stone-300 rounded-lg focus:outline-none focus:border-amber-500 bg-stone-50 resize-none"
                />
              </div>
              {modal.error && (
                <p className="text-sm text-red-600">{modal.error}</p>
              )}
            </div>
            <div className="px-6 py-3 border-t border-stone-200 flex justify-end gap-2">
              <button onClick={closeModal} className="px-4 py-2 text-sm rounded-lg text-stone-600 hover:bg-stone-100 cursor-pointer">
                {modal.mode === "view" ? "Close" : "Cancel"}
              </button>
              {modal.mode === "create" && (
                <button
                  onClick={handleSave}
                  disabled={modal.loading}
                  className="px-4 py-2 text-sm rounded-lg bg-amber-500 text-amber-900 font-semibold hover:bg-amber-600 disabled:opacity-50 cursor-pointer"
                >
                  {modal.loading ? "Saving…" : "Save Secret"}
                </button>
              )}
            </div>
          </div>
        </div>
      )}
    </>
  );
}
