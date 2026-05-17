import { useSessionStore } from "../stores/sessions";
import type { AgentSession } from "../stores/sessions";
import { SessionRow } from "./SessionRow";

function isVisible(s: AgentSession): boolean {
  // Active sessions always visible
  if (s.phase === "running" || s.phase === "waiting-for-approval" || s.phase === "waiting-for-answer") {
    return true;
  }
  // Completed: only show if updated within last 2 minutes (just finished)
  const twoMinutes = 2 * 60 * 1000;
  return s.phase === "completed" && Date.now() - s.updatedAt < twoMinutes;
}

export function SessionList() {
  const sessions = useSessionStore((s) => s.sessions);
  const visible = sessions.filter(isVisible);

  if (visible.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-14 px-4">
        <div className="w-8 h-8 rounded-full bg-white/[0.04] flex items-center justify-center mb-3">
          <div className="w-3 h-3 rounded-full bg-emerald-500/40" />
        </div>
        <p className="text-[12px] text-[#f1ead9]/40 font-medium">All quiet</p>
        <p className="text-[11px] text-[#f1ead9]/20 mt-1">
          Sessions appear here when agents are active
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-1.5 p-2">
      {visible.map((s) => (
        <SessionRow key={s.id} session={s} />
      ))}
    </div>
  );
}
