import { useState } from "react";
import { useSessionStore } from "../stores/sessions";
import type { AgentSession } from "../stores/sessions";
import { SessionRow } from "./SessionRow";

const ONE_HOUR = 3600000;

function isActive(s: AgentSession): boolean {
  return (
    s.phase === "running" ||
    s.phase === "waiting-for-approval" ||
    s.phase === "waiting-for-answer"
  );
}

function isRecent(s: AgentSession): boolean {
  return Date.now() - s.updatedAt < ONE_HOUR;
}

export function SessionList() {
  const sessions = useSessionStore((s) => s.sessions);
  const [showOlder, setShowOlder] = useState(false);

  if (sessions.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16 px-4">
        <div className="w-10 h-10 rounded-full bg-zinc-800/60 flex items-center justify-center mb-3">
          <span className="text-lg opacity-50">🏝️</span>
        </div>
        <p className="text-[12px] text-zinc-500 font-medium">No sessions</p>
        <p className="text-[11px] text-zinc-600 mt-1">
          Start a coding agent to monitor it here
        </p>
      </div>
    );
  }

  const active = sessions.filter(isActive);
  const recentDone = sessions.filter((s) => !isActive(s) && isRecent(s));
  const olderDone = sessions.filter((s) => !isActive(s) && !isRecent(s));

  return (
    <div className="flex flex-col gap-1.5 p-2">
      {/* Active sessions */}
      {active.map((s) => (
        <SessionRow key={s.id} session={s} />
      ))}

      {/* Divider if both active and completed exist */}
      {active.length > 0 && (recentDone.length > 0 || olderDone.length > 0) && (
        <div className="h-px bg-white/[0.04] mx-2 my-1" />
      )}

      {/* Recent completed */}
      {recentDone.map((s) => (
        <SessionRow key={s.id} session={s} compact />
      ))}

      {/* Older completed toggle */}
      {olderDone.length > 0 && (
        <button
          onClick={() => setShowOlder(!showOlder)}
          className="flex items-center justify-center gap-1 text-[10px] text-zinc-600 hover:text-zinc-400 py-1.5 transition-colors"
        >
          <span>{showOlder ? "▾" : "▸"}</span>
          <span>
            {olderDone.length} older session{olderDone.length !== 1 ? "s" : ""}
          </span>
        </button>
      )}
      {showOlder &&
        olderDone.map((s) => <SessionRow key={s.id} session={s} compact />)}

      {/* Idle indicator */}
      {active.length === 0 && sessions.length > 0 && (
        <div className="text-center py-2">
          <span className="text-[10px] text-zinc-600">All agents idle</span>
        </div>
      )}
    </div>
  );
}
