import { useState } from "react";
import { useSessionStore } from "../stores/sessions";
import type { AgentSession } from "../stores/sessions";
import { SessionRow } from "./SessionRow";

const ONE_HOUR = 3600000;

function isActive(s: AgentSession): boolean {
  return s.phase === "running" || s.phase === "waiting-for-approval" || s.phase === "waiting-for-answer";
}

function isRecent(s: AgentSession): boolean {
  return Date.now() - s.updatedAt < ONE_HOUR;
}

export function SessionList() {
  const sessions = useSessionStore((s) => s.sessions);
  const [showCompleted, setShowCompleted] = useState(false);

  if (sessions.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 px-4 text-zinc-500">
        <div className="text-3xl mb-3">🏝️</div>
        <p className="text-sm">No active sessions</p>
        <p className="text-xs mt-1 text-zinc-600">
          Start an AI agent to see it here
        </p>
      </div>
    );
  }

  // Split sessions: active first, then recent completed, then old
  const active = sessions.filter(isActive);
  const recentCompleted = sessions.filter((s) => !isActive(s) && isRecent(s));
  const oldCompleted = sessions.filter((s) => !isActive(s) && !isRecent(s));

  return (
    <div className="flex flex-col gap-1 p-2">
      {/* Active sessions — always visible, prominent */}
      {active.map((session) => (
        <SessionRow key={session.id} session={session} />
      ))}

      {/* Recent completed — visible but dimmed */}
      {recentCompleted.map((session) => (
        <SessionRow key={session.id} session={session} compact />
      ))}

      {/* Old completed — collapsed by default */}
      {oldCompleted.length > 0 && (
        <>
          <button
            onClick={() => setShowCompleted(!showCompleted)}
            className="text-xs text-zinc-600 hover:text-zinc-400 py-1.5 transition-colors"
          >
            {showCompleted ? "Hide" : "Show"} {oldCompleted.length} older session{oldCompleted.length !== 1 ? "s" : ""}
          </button>
          {showCompleted &&
            oldCompleted.map((session) => (
              <SessionRow key={session.id} session={session} compact />
            ))}
        </>
      )}

      {/* No active sessions message */}
      {active.length === 0 && recentCompleted.length > 0 && (
        <div className="text-center py-3 text-xs text-zinc-600">
          No active sessions
        </div>
      )}
    </div>
  );
}
