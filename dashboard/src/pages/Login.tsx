import { useState, useEffect } from "react";
import { useNavigate } from "react-router";
import { setToken } from "../lib/api";

export function LoginPage() {
  const navigate = useNavigate();
  const [token, setTokenValue] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  const springAuthUrl = (window as unknown as Record<string, unknown>).__SPRING_AUTH_URL__ as string | undefined;

  // Handle OIDC callback: the server redirects here with ?oidc_token=...
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const oidcToken = params.get("oidc_token");
    const oidcError = params.get("error");

    if (oidcError) {
      setError(decodeURIComponent(oidcError));
      window.history.replaceState({}, "", "/app/login");
      return;
    }

    if (oidcToken) {
      setToken(oidcToken);
      window.history.replaceState({}, "", "/app/login");
      navigate("/", { replace: true });
    }
  }, [navigate]);

  async function handleLogin() {
    setError("");
    const trimmed = token.trim();
    if (!trimmed) {
      setError("Please enter a vault token.");
      return;
    }
    setLoading(true);
    try {
      const res = await fetch("/v1/auth/token/lookup-self", {
        method: "POST",
        headers: { "X-Vault-Token": trimmed, "Content-Type": "application/json" },
        body: "{}",
      });
      if (!res.ok) throw new Error("Invalid or expired token");
      setToken(trimmed);
      navigate("/", { replace: true });
    } catch (e) {
      setError(e instanceof Error ? e.message : "Authentication failed");
    } finally {
      setLoading(false);
    }
  }

  function handleSpringLogin() {
    if (springAuthUrl) {
      window.location.href = springAuthUrl;
    }
  }

  return (
    <div className="flex items-center justify-center min-h-screen">
      <div className="w-full max-w-[440px] px-5">
        <div className="text-center mb-9">
          <svg viewBox="0 0 32 32" fill="none" className="w-12 h-12 mx-auto mb-4">
            <defs>
              <linearGradient id="zg" x1="0" y1="0" x2="32" y2="32">
                <stop offset="0%" stopColor="#F5C842" />
                <stop offset="100%" stopColor="#E8A817" />
              </linearGradient>
            </defs>
            <rect width="32" height="32" rx="8" fill="url(#zg)" />
            <path d="M9 11h14l-14 10h14" stroke="#2D1F0E" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
          <h1 className="text-[28px] font-extrabold text-stone-800 tracking-tight mb-1.5">
            Sign in to ZVault
          </h1>
          <p className="text-sm text-stone-500">
            Enter your vault token or sign in with Spring.
          </p>
        </div>

        <div className="bg-glass backdrop-blur-[16px] border border-glass-border rounded-[20px] p-8 shadow-[0_8px_32px_rgba(0,0,0,.06)]">
          {springAuthUrl && (
            <>
              <button
                onClick={handleSpringLogin}
                className="w-full flex items-center justify-center gap-2.5 py-3 rounded-full bg-stone-800 text-stone-100 font-semibold text-sm hover:bg-stone-700 transition-colors cursor-pointer mb-5"
              >
                <svg viewBox="0 0 20 20" fill="none" className="w-5 h-5">
                  <circle cx="10" cy="10" r="9" stroke="currentColor" strokeWidth="1.5" />
                  <path d="M6 10h8M10 6v8" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
                </svg>
                Sign in with Spring
              </button>
              <div className="flex items-center gap-3 mb-5">
                <div className="flex-1 h-px bg-stone-300/40" />
                <span className="text-xs text-stone-400 font-medium">or use a token</span>
                <div className="flex-1 h-px bg-stone-300/40" />
              </div>
            </>
          )}

          <div className="mb-5">
            <label className="block text-[13px] font-bold text-stone-700 mb-[7px]">
              Vault Token
            </label>
            <input
              type="password"
              value={token}
              onChange={(e) => setTokenValue(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleLogin()}
              placeholder="hvs.CAESIG..."
              autoComplete="off"
              className="w-full px-4 py-[11px] border border-stone-300/40 rounded-[10px] text-[13px] font-mono bg-glass backdrop-blur-[8px] text-stone-800 transition-all focus:outline-none focus:border-amber-500 focus:ring-[3px] focus:ring-amber-500/12 focus:bg-white/70 placeholder:text-stone-400"
            />
            <p className="text-xs text-stone-400 mt-1.5">
              The root token from initialization, or a scoped token.
            </p>
          </div>

          {error && (
            <div className="bg-red-500/10 text-red-700 px-4 py-2.5 rounded-[10px] text-[13px] font-semibold mb-4">
              {error}
            </div>
          )}

          <button
            onClick={handleLogin}
            disabled={loading}
            className="w-full py-3 rounded-full bg-amber-500 text-amber-900 font-semibold text-sm hover:bg-amber-600 hover:shadow-[0_4px_20px_rgba(0,0,0,.12)] hover:-translate-y-px transition-all disabled:opacity-50 cursor-pointer"
          >
            {loading ? "Verifying…" : "Sign In"}
          </button>
        </div>

        <p className="text-center mt-5 text-[13px] text-stone-400">
          Don't have a token?{" "}
          <a href="/app/init" className="text-amber-500 font-semibold hover:underline">
            Initialize the vault
          </a>
        </p>
      </div>
    </div>
  );
}
