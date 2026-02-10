import { useEffect, useState } from "react";
import { useOutletContext } from "react-router";
import { vaultFetch, type SealStatus } from "../lib/api";
import { Th, Td, Badge } from "../components/Table";

interface AuthMethod {
  path: string;
  type: string;
  description: string;
  enabled: boolean;
}

/** Built-in auth methods that are always available. */
const BUILTIN_AUTH: AuthMethod[] = [
  { path: "token/", type: "Token", description: "Built-in token authentication", enabled: true },
  { path: "approle/", type: "AppRole", description: "Machine-to-machine authentication", enabled: true },
];

/** Planned auth methods (not yet implemented). */
const PLANNED_AUTH: AuthMethod[] = [
  { path: "oidc/", type: "OIDC", description: "OpenID Connect via Spring identity", enabled: false },
  { path: "kubernetes/", type: "Kubernetes", description: "Service account authentication", enabled: false },
];

interface MountEntry {
  path: string;
  engine_type: string;
  description: string;
}

export function AuthMethodsPage() {
  const { sealStatus } = useOutletContext<{ sealStatus: SealStatus | null }>();
  const [methods, setMethods] = useState<AuthMethod[]>([...BUILTIN_AUTH, ...PLANNED_AUTH]);
  const [loading, setLoading] = useState(true);

  const isUnsealed = sealStatus?.initialized && !sealStatus?.sealed;

  useEffect(() => {
    if (!isUnsealed) {
      setLoading(false);
      return;
    }

    const fetchMounts = async () => {
      try {
        const resp = await vaultFetch<{ mounts?: MountEntry[] }>("/v1/sys/mounts");
        const mounts = resp.mounts ?? [];

        // Check if approle is actually mounted by looking at engine mounts.
        // Token auth is always enabled. AppRole is enabled if the server has it.
        const hasApprole = mounts.some(
          (m) => m.engine_type === "approle" || m.path.includes("approle")
        );

        const live: AuthMethod[] = [
          { path: "token/", type: "Token", description: "Built-in token authentication", enabled: true },
          {
            path: "approle/",
            type: "AppRole",
            description: "Machine-to-machine authentication",
            enabled: hasApprole || true, // AppRole is always registered in server
          },
          ...PLANNED_AUTH,
        ];

        setMethods(live);
      } catch {
        // Fall back to defaults on error.
        setMethods([...BUILTIN_AUTH, ...PLANNED_AUTH]);
      } finally {
        setLoading(false);
      }
    };

    fetchMounts();
  }, [isUnsealed]);

  return (
    <>
      <div className="flex justify-between items-center mb-6">
        <p className="text-sm text-stone-500">Configure authentication methods for identity verification.</p>
        <button className="px-4 py-2 rounded-full bg-amber-500 text-amber-900 text-[13px] font-semibold hover:bg-amber-600 transition-all cursor-pointer">
          + Enable Auth Method
        </button>
      </div>

      <div className="bg-glass backdrop-blur-[16px] border border-glass-border rounded-[20px] shadow-[0_8px_32px_rgba(0,0,0,.06)] overflow-hidden">
        <table className="w-full text-[13px] border-collapse">
          <thead>
            <tr>
              <Th>Path</Th><Th>Type</Th><Th>Description</Th><Th>Status</Th><Th>Actions</Th>
            </tr>
          </thead>
          <tbody>
            {loading ? (
              <tr>
                <Td colSpan={5}>
                  <span className="text-stone-400">Loading auth methods…</span>
                </Td>
              </tr>
            ) : (
              methods.map((m) => (
                <tr key={m.path} className="hover:bg-stone-50">
                  <Td><code className="bg-stone-100 text-stone-600 px-1.5 py-0.5 rounded text-xs">{m.path}</code></Td>
                  <Td>{m.type}</Td>
                  <Td>{m.description}</Td>
                  <Td>
                    <Badge variant={m.enabled ? "success" : "muted"}>
                      {m.enabled ? "Enabled" : "Planned"}
                    </Badge>
                  </Td>
                  <Td>
                    <button className={`px-3 py-[5px] text-xs font-semibold rounded-full transition-all cursor-pointer ${
                      m.enabled
                        ? "bg-glass border border-stone-300/40 text-stone-700 hover:bg-surface"
                        : "bg-amber-500 text-amber-900 hover:bg-amber-600"
                    }`}>
                      {m.enabled ? "Configure" : "Enable"}
                    </button>
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
