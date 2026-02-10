export function Th({ children }: { children: React.ReactNode }) {
  return (
    <th className="text-left font-bold text-stone-500 text-[11px] uppercase tracking-[.7px] px-[18px] py-[11px] border-b border-stone-200/60 bg-white/20">
      {children}
    </th>
  );
}

export function Td({ children, colSpan }: { children: React.ReactNode; colSpan?: number }) {
  return (
    <td colSpan={colSpan} className="px-[18px] py-[13px] border-b border-stone-200/60 text-stone-800">
      {children}
    </td>
  );
}

const badgeVariants: Record<string, string> = {
  success: "bg-green-500/12 text-green-700",
  warning: "bg-amber-500/12 text-amber-700",
  danger: "bg-red-500/10 text-red-700",
  info: "bg-blue-500/10 text-blue-700",
  primary: "bg-stone-100 text-stone-600",
  muted: "bg-black/6 text-stone-400",
};

export function Badge({ variant = "muted", children }: { variant?: string; children: React.ReactNode }) {
  return (
    <span className={`inline-block px-3 py-1 rounded-full text-[11px] font-bold tracking-[.2px] ${badgeVariants[variant] ?? badgeVariants.muted}`}>
      {children}
    </span>
  );
}

export function Card({ title, actions, children }: { title: string; actions?: React.ReactNode; children: React.ReactNode }) {
  return (
    <div className="bg-glass backdrop-blur-[16px] border border-glass-border rounded-[20px] shadow-[0_8px_32px_rgba(0,0,0,.06)] overflow-hidden mb-[22px]">
      <div className="flex items-center justify-between px-[22px] py-[18px] border-b border-stone-200/60 bg-white/25">
        <span className="text-[15px] font-bold text-stone-800">{title}</span>
        {actions}
      </div>
      {children}
    </div>
  );
}
