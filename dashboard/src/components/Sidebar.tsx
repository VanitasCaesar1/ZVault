import { NavLink } from "react-router";

const navSections = [
  {
    label: "System",
    items: [
      { to: "/", icon: "grid", label: "Dashboard" },
      { to: "/init", icon: "zap", label: "Initialize" },
      { to: "/unseal", icon: "lock", label: "Unseal" },
    ],
  },
  {
    label: "Manage",
    items: [
      { to: "/secrets", icon: "key", label: "Secrets" },
      { to: "/policies", icon: "shield", label: "Policies" },
      { to: "/leases", icon: "clock", label: "Leases" },
      { to: "/audit", icon: "file-text", label: "Audit Log" },
      { to: "/auth", icon: "users", label: "Auth Methods" },
      { to: "/billing", icon: "credit-card", label: "Billing" },
    ],
  },
  {
    label: "Cloud",
    items: [
      { to: "/cloud/projects", icon: "cloud", label: "Projects" },
      { to: "/cloud/team", icon: "users", label: "Team" },
      { to: "/cloud/tokens", icon: "key", label: "Service Tokens" },
      { to: "/cloud/audit", icon: "file-text", label: "Cloud Audit" },
    ],
  },
];

const icons: Record<string, React.ReactNode> = {
  grid: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="w-[18px] h-[18px] shrink-0 opacity-60 group-[.active]:opacity-100">
      <rect x="3" y="3" width="7" height="7" rx="1" /><rect x="14" y="3" width="7" height="7" rx="1" />
      <rect x="3" y="14" width="7" height="7" rx="1" /><rect x="14" y="14" width="7" height="7" rx="1" />
    </svg>
  ),
  zap: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="w-[18px] h-[18px] shrink-0 opacity-60 group-[.active]:opacity-100">
      <path d="M12 2v4m0 12v4M2 12h4m12 0h4" /><circle cx="12" cy="12" r="3" />
    </svg>
  ),
  lock: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="w-[18px] h-[18px] shrink-0 opacity-60 group-[.active]:opacity-100">
      <rect x="3" y="11" width="18" height="11" rx="2" /><path d="M7 11V7a5 5 0 0110 0v4" />
    </svg>
  ),
  key: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="w-[18px] h-[18px] shrink-0 opacity-60 group-[.active]:opacity-100">
      <path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 11-7.778 7.778 5.5 5.5 0 017.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4" />
    </svg>
  ),
  shield: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="w-[18px] h-[18px] shrink-0 opacity-60 group-[.active]:opacity-100">
      <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
    </svg>
  ),
  clock: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="w-[18px] h-[18px] shrink-0 opacity-60 group-[.active]:opacity-100">
      <circle cx="12" cy="12" r="10" /><path d="M12 6v6l4 2" />
    </svg>
  ),
  "file-text": (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="w-[18px] h-[18px] shrink-0 opacity-60 group-[.active]:opacity-100">
      <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" />
      <path d="M14 2v6h6" /><path d="M16 13H8m8 4H8m2-8H8" />
    </svg>
  ),
  users: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="w-[18px] h-[18px] shrink-0 opacity-60 group-[.active]:opacity-100">
      <path d="M16 21v-2a4 4 0 00-4-4H6a4 4 0 00-4 4v2" /><circle cx="9" cy="7" r="4" />
      <path d="M22 21v-2a4 4 0 00-3-3.87M16 3.13a4 4 0 010 7.75" />
    </svg>
  ),
  "credit-card": (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="w-[18px] h-[18px] shrink-0 opacity-60 group-[.active]:opacity-100">
      <rect x="1" y="4" width="22" height="16" rx="2" /><path d="M1 10h22" />
    </svg>
  ),
  cloud: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="w-[18px] h-[18px] shrink-0 opacity-60 group-[.active]:opacity-100">
      <path d="M18 10h-1.26A8 8 0 109 20h9a5 5 0 000-10z" />
    </svg>
  ),
};

export function Sidebar() {
  return (
    <aside className="w-[260px] bg-sidebar text-sidebar-text flex flex-col fixed top-0 left-0 bottom-0 z-50 overflow-y-auto max-md:hidden">
      {/* Logo */}
      <div className="flex items-center gap-3 px-6 pt-7 pb-8">
        <svg viewBox="0 0 32 32" fill="none" className="w-8 h-8 shrink-0">
          <defs>
            <linearGradient id="zg" x1="0" y1="0" x2="32" y2="32">
              <stop offset="0%" stopColor="#F5C842" />
              <stop offset="100%" stopColor="#E8A817" />
            </linearGradient>
          </defs>
          <rect width="32" height="32" rx="8" fill="url(#zg)" />
          <path d="M9 11h14l-14 10h14" stroke="#2D1F0E" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
        <span className="text-xl font-extrabold text-sidebar-active tracking-tight">ZVault</span>
      </div>

      {/* Nav */}
      <nav className="flex-1 px-3.5">
        {navSections.map((section) => (
          <div key={section.label} className="mb-7">
            <div className="text-[10px] font-bold uppercase tracking-[1.2px] text-stone-500 px-3 mb-2">
              {section.label}
            </div>
            {section.items.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                end={item.to === "/"}
                className={({ isActive }) =>
                  `group flex items-center gap-[11px] px-3.5 py-2.5 rounded-[10px] text-[13.5px] font-medium transition-all duration-200 ${
                    isActive
                      ? "active text-sidebar-active bg-amber-500/18 font-semibold"
                      : "text-sidebar-text hover:text-[#D4C4A8] hover:bg-white/5"
                  }`
                }
              >
                {icons[item.icon]}
                {item.label}
              </NavLink>
            ))}
          </div>
        ))}
      </nav>

      {/* Footer */}
      <div className="px-6 py-4.5 text-[11px] text-stone-500 border-t border-white/6">
        <a href="https://docs.zvault.cloud" target="_blank" rel="noopener noreferrer" className="block text-xs text-sidebar-text hover:text-sidebar-active transition-colors mb-1.5">
          Documentation
        </a>
        <a href="/login" className="block text-xs text-red-400 hover:text-red-300 transition-colors mb-1.5"
          onClick={(e) => {
            e.preventDefault();
            document.cookie = "zvault-token=;path=/;max-age=0";
            window.location.href = "/login";
          }}
        >
          Sign Out
        </a>
        ZVault v0.1.0
      </div>
    </aside>
  );
}
