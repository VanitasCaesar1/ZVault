import { useEffect, useState } from "react";
import { Outlet, useNavigate, useLocation } from "react-router";
import {
  getToken,
  setToken,
  getSealStatus,
  setCloudTokenGetter,
  clearCloudTokenGetter,
  type SealStatus,
} from "../lib/api";
import { useAuth } from "../hooks/useAuth";
import { Sidebar } from "../components/Sidebar";
import { Topbar } from "../components/Topbar";

export function DashboardLayout() {
  const navigate = useNavigate();
  const location = useLocation();
  const [sealStatus, setSealStatus] = useState<SealStatus | null>(null);

  const {
    isCloudAuthenticated,
    isCloudLoading,
    user: cloudUser,
    getToken: getClerkToken,
    signOut,
  } = useAuth();

  // Register Clerk token getter so cloudFetch() can use it.
  useEffect(() => {
    if (isCloudAuthenticated) {
      setCloudTokenGetter(getClerkToken);
    }
    return () => {
      clearCloudTokenGetter();
    };
  }, [isCloudAuthenticated, getClerkToken]);

  // Auth guard — check vault token OR Clerk session.
  useEffect(() => {
    // Still loading Clerk state — wait.
    if (isCloudLoading) return;

    // Clerk authenticated — good to go.
    if (isCloudAuthenticated) return;

    // Capture token from OAuth callback redirect (/?token=...)
    const params = new URLSearchParams(window.location.search);
    const callbackToken = params.get("token");
    if (callbackToken) {
      setToken(callbackToken);
      window.history.replaceState({}, "", location.pathname);
    }

    const token = callbackToken || getToken();
    if (!token) {
      navigate("/login", { replace: true });
      return;
    }

    // Validate vault token.
    const apiBase = import.meta.env.VITE_API_URL ?? "";
    fetch(`${apiBase}/v1/auth/token/lookup-self`, {
      method: "POST",
      headers: {
        "X-Vault-Token": token,
        "Content-Type": "application/json",
      },
      body: "{}",
    }).catch(() => {
      navigate("/login", { replace: true });
    });
  }, [navigate, isCloudAuthenticated, isCloudLoading, location.pathname]);

  // Poll seal status.
  useEffect(() => {
    getSealStatus().then(setSealStatus).catch(() => {});
    const interval = setInterval(() => {
      getSealStatus().then(setSealStatus).catch(() => {});
    }, 15000);
    return () => clearInterval(interval);
  }, []);

  // Show nothing while Clerk is loading.
  if (isCloudLoading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <p className="text-stone-500 text-sm">Loading…</p>
      </div>
    );
  }

  const pageTitle =
    {
      "/": "Dashboard",
      "/init": "Initialize Vault",
      "/unseal": "Unseal Vault",
      "/secrets": "Secrets",
      "/policies": "Policies",
      "/audit": "Audit Log",
      "/leases": "Leases",
      "/auth": "Auth Methods",
      "/billing": "Billing",
      "/cloud/projects": "Cloud Projects",
      "/cloud/team": "Team",
      "/cloud/tokens": "Service Tokens",
      "/cloud/audit": "Cloud Audit Log",
    }[location.pathname] ?? (location.pathname.startsWith("/cloud/projects/") ? "Project" : "ZVault");

  return (
    <div className="flex min-h-screen">
      <Sidebar />
      <div className="ml-[260px] flex-1 min-h-screen max-md:ml-0">
        <Topbar
          title={pageTitle}
          sealStatus={sealStatus}
          cloudUser={isCloudAuthenticated ? cloudUser : undefined}
          onCloudSignOut={isCloudAuthenticated ? () => signOut() : undefined}
        />
        <main className="p-8 max-md:p-4">
          <Outlet context={{ sealStatus }} />
        </main>
      </div>
    </div>
  );
}
