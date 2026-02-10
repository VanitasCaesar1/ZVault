import { useEffect, useState } from "react";
import { Outlet, useNavigate, useLocation } from "react-router";
import { getToken, setToken, getSealStatus, type SealStatus } from "../lib/api";
import { Sidebar } from "../components/Sidebar";
import { Topbar } from "../components/Topbar";

export function DashboardLayout() {
  const navigate = useNavigate();
  const location = useLocation();
  const [sealStatus, setSealStatus] = useState<SealStatus | null>(null);

  useEffect(() => {
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
    // Validate token
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
  }, [navigate]);

  useEffect(() => {
    getSealStatus().then(setSealStatus).catch(() => {});
    const interval = setInterval(() => {
      getSealStatus().then(setSealStatus).catch(() => {});
    }, 15000);
    return () => clearInterval(interval);
  }, []);

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
    }[location.pathname] ?? "ZVault";

  return (
    <div className="flex min-h-screen">
      <Sidebar />
      <div className="ml-[260px] flex-1 min-h-screen max-md:ml-0">
        <Topbar title={pageTitle} sealStatus={sealStatus} />
        <main className="p-8 max-md:p-4">
          <Outlet context={{ sealStatus }} />
        </main>
      </div>
    </div>
  );
}
