import type { SealStatus } from "../lib/api";
import { clearToken } from "../lib/api";

interface TopbarProps {
  title: string;
  sealStatus: SealStatus | null;
  /** Cloud user info (when signed in via Clerk). */
  cloudUser?: { name?: string; email?: string; picture?: string };
  /** Cloud sign-out handler. */
  onCloudSignOut?: () => void;
}

export function Topbar({ title, sealStatus, cloudUser, onCloudSignOut }: TopbarProps) {
  const sealLabel = !sealStatus
    ? "Loading…"
    : !sealStatus.initialized
      ? "Not Initialized"
      : sealStatus.sealed
        ? "Sealed"
        : "Unsealed";

  const dotColor = !sealStatus
    ? "bg-gray-400"
    : !sealStatus.initialized
      ? "bg-red-500"
      : sealStatus.sealed
        ? "bg-amber-500"
        : "bg-green-500";

  function handleSignOut() {
    if (onCloudSignOut) {
      onCloudSignOut();
    } else {
      clearToken();
      window.location.href = "/app/login";
    }
  }

  return (
    <header className="flex items-center justify-between px-9 py-5 bg-glass backdrop-blur-[20px] border-b border-glass-border max-md:px-4">
      <h1 className="text-[22px] font-extrabold tracking-tight text-stone-800">
        {title}
      </h1>
      <div className="flex items-center gap-3.5">
        {/* Seal status badge */}
        <div className="flex items-center gap-[7px] text-[13px] font-semibold text-stone-600 bg-surface px-3.5 py-1.5 rounded-full border border-stone-200/60">
          <span className={`w-2 h-2 rounded-full ${dotColor}`} />
          {sealLabel}
        </div>

        {/* Cloud user avatar + name */}
        {cloudUser && (
          <div className="flex items-center gap-2 text-[13px] font-semibold text-stone-600 bg-surface px-3 py-1.5 rounded-full border border-stone-200/60">
            {cloudUser.picture && (
              <img
                src={cloudUser.picture}
                alt=""
                className="w-5 h-5 rounded-full"
              />
            )}
            <span className="max-w-[120px] truncate">
              {cloudUser.name ?? cloudUser.email ?? "User"}
            </span>
          </div>
        )}

        <button
          onClick={handleSignOut}
          className="px-3.5 py-[7px] text-xs font-semibold text-white bg-red-500 hover:bg-red-600 rounded-full transition-colors cursor-pointer"
        >
          Sign Out
        </button>
        <a
          href="/"
          className="px-3.5 py-[7px] text-xs font-semibold text-stone-700 bg-glass backdrop-blur-[10px] border border-stone-300/40 rounded-full hover:bg-surface hover:border-amber-500 transition-all"
        >
          Back to Site
        </a>
      </div>
    </header>
  );
}
