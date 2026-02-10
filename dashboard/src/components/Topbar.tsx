import type { SealStatus } from "../lib/api";
import { clearToken } from "../lib/api";

interface TopbarProps {
  title: string;
  sealStatus: SealStatus | null;
}

export function Topbar({ title, sealStatus }: TopbarProps) {
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

  return (
    <header className="flex items-center justify-between px-9 py-5 bg-glass backdrop-blur-[20px] border-b border-glass-border max-md:px-4">
      <h1 className="text-[22px] font-extrabold tracking-tight text-stone-800">
        {title}
      </h1>
      <div className="flex items-center gap-3.5">
        <div className="flex items-center gap-[7px] text-[13px] font-semibold text-stone-600 bg-surface px-3.5 py-1.5 rounded-full border border-stone-200/60">
          <span className={`w-2 h-2 rounded-full ${dotColor}`} />
          {sealLabel}
        </div>
        <button
          onClick={() => {
            clearToken();
            window.location.href = "/app/login";
          }}
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
