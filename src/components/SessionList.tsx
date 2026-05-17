import { useSessionStore } from "../stores/sessions";
import { SessionRow } from "./SessionRow";

export function SessionList() {
  const sessions = useSessionStore((s) => s.sessions);

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

  return (
    <div className="flex flex-col gap-1 p-2">
      {sessions.map((session) => (
        <SessionRow key={session.id} session={session} />
      ))}
    </div>
  );
}
